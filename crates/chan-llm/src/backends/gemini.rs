//! Google Gemini backend.
//!
//! Talks to `generativelanguage.googleapis.com` directly, no SDK.
//! That's the public Generative Language API; Vertex AI is a
//! different surface (OAuth + project ID) and out of scope here.
//!
//! Streaming via `:streamGenerateContent?alt=sse`. Each SSE event
//! carries a partial `GenerateContentResponse`; we accumulate
//! `candidates[0].content.parts[*].text` deltas, collect any
//! `functionCall` parts, and read `finishReason` off the final
//! chunk.
//!
//! Wire shape differs from Anthropic and Ollama:
//!
//!   - Roles are `user` and `model` (not `assistant`).
//!   - The system prompt sits in a top-level `systemInstruction`
//!     field instead of inside the messages array.
//!   - Tool calls round-trip as `functionCall` parts on a `model`
//!     turn and `functionResponse` parts on a `user` turn (the API
//!     doesn't accept a `tool` role on this endpoint).
//!   - Per-call ids aren't returned, so we synthesize stable
//!     `gemini-<idx>` ids per response position. The orchestrator's
//!     "tool_call_id <-> tool_result" pairing still works because
//!     the next turn's `Role::Tool` message carries the same id.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::LlmError;
use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::{Backend, Outcome};

const ENDPOINT_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;

#[derive(Debug)]
pub struct GeminiBackend {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiBackend {
    pub fn new(api_key: String, model: String) -> Self {
        // 5 minute timeout: same headroom as the other backends for
        // tool-use loops with iterated reads / searches.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client builds with default rustls config");
        Self {
            api_key,
            model,
            client,
        }
    }
}

#[async_trait]
impl Backend for GeminiBackend {
    async fn run(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        listener: Arc<dyn SessionListener>,
    ) -> Outcome {
        let (system_instruction, contents) = build_contents(&messages);
        let tools_wire = if tools.is_empty() {
            Vec::new()
        } else {
            vec![GeminiTools {
                function_declarations: tools
                    .iter()
                    .map(|t| GeminiFunctionDecl {
                        name: t.name,
                        description: t.description,
                        parameters: &t.parameters,
                    })
                    .collect(),
            }]
        };
        let body = GeminiRequest {
            contents,
            system_instruction,
            tools: tools_wire,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: Some(DEFAULT_MAX_OUTPUT_TOKENS),
            },
        };

        let url = format!(
            "{ENDPOINT_BASE}/models/{}:streamGenerateContent?alt=sse",
            self.model
        );
        let resp = match self
            .client
            .post(&url)
            // x-goog-api-key keeps the secret out of the URL (and
            // out of access logs) compared to ?key=.
            .header("x-goog-api-key", &self.api_key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                listener.on_error(format!("gemini request: {e}"));
                return Outcome::error();
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();
            let snippet: String = raw.chars().take(800).collect();
            listener.on_error(format!("gemini {status}: {snippet}"));
            return Outcome::error();
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("gemini stream: {e}"));
                    return Outcome::error();
                }
            };
            buf.extend_from_slice(&chunk);
            while let Some(end) = buf.windows(2).position(|w| w == b"\n\n") {
                let raw_event: Vec<u8> = buf.drain(..end).collect();
                let _: Vec<u8> = buf.drain(..2.min(buf.len())).collect();
                let payload = match extract_data(&raw_event) {
                    Some(p) => p,
                    None => continue,
                };
                let parsed: GeminiResponseChunk = match serde_json::from_str(&payload) {
                    Ok(p) => p,
                    Err(e) => {
                        listener.on_error(format!("gemini parse: {e}; raw: {payload}"));
                        return Outcome::error();
                    }
                };
                if let Some(candidate) = parsed.candidates.into_iter().next() {
                    if let Some(reason) = candidate.finish_reason {
                        finish_reason = Some(reason);
                    }
                    if let Some(parts) = candidate.content.and_then(|c| c.parts) {
                        for part in parts {
                            if let Some(text) = part.text {
                                if !text.is_empty() {
                                    listener.on_delta(Delta { text: text.clone() });
                                    assistant_text.push_str(&text);
                                }
                            }
                            if let Some(call) = part.function_call {
                                // Synthesize a stable id by position
                                // in this turn. The orchestrator pairs
                                // the next Role::Tool message via this
                                // id; build_contents recovers the tool
                                // name from the prior `model` turn.
                                let id = format!("gemini-{}", tool_calls.len());
                                tool_calls.push(ToolCall {
                                    id,
                                    name: call.name,
                                    args: call.args.unwrap_or(serde_json::Value::Null),
                                });
                            }
                        }
                    }
                }
            }
        }

        let stop_reason = if !tool_calls.is_empty() {
            StopReason::ToolUse
        } else {
            match finish_reason.as_deref() {
                Some("STOP") | None => StopReason::EndOfTurn,
                Some("MAX_TOKENS") => StopReason::MaxTokens,
                Some("STOP_SEQUENCE") => StopReason::StopSequence,
                _ => StopReason::EndOfTurn,
            }
        };

        Outcome {
            assistant_text,
            tool_calls,
            stop_reason,
        }
    }
}

/// Pull `generateContent`-eligible models off `/v1beta/models`.
/// Filters to entries whose `supportedGenerationMethods` includes
/// `generateContent` so embedding-only models don't pollute the
/// dropdown.
pub async fn list_models(api_key: &str) -> Result<Vec<String>, LlmError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client builds with default rustls config");
    let mut out: Vec<String> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut req = client
            .get(format!("{ENDPOINT_BASE}/models"))
            .header("x-goog-api-key", api_key)
            .query(&[("pageSize", "100")]);
        if let Some(token) = &page_token {
            req = req.query(&[("pageToken", token.as_str())]);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| LlmError::Http(format!("gemini models: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Http(format!("gemini models {status}: {body}")));
        }
        let page: ModelsPage = resp
            .json()
            .await
            .map_err(|e| LlmError::Http(format!("gemini models decode: {e}")))?;
        for m in page.models.unwrap_or_default() {
            if !m
                .supported_generation_methods
                .iter()
                .any(|s| s == "generateContent")
            {
                continue;
            }
            // The API returns `models/<id>`; strip the prefix so the
            // bare id matches what the Settings UI passes back to
            // chan-llm.
            let id = m.name.strip_prefix("models/").unwrap_or(&m.name).to_owned();
            out.push(id);
        }
        match page.next_page_token {
            Some(t) if !t.is_empty() => page_token = Some(t),
            _ => break,
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct ModelsPage {
    #[serde(default)]
    models: Option<Vec<ModelInfo>>,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
    #[serde(default, rename = "supportedGenerationMethods")]
    supported_generation_methods: Vec<String>,
}

/// Translate chan-llm's Message list into Gemini's split shape:
/// system prompt as a top-level `systemInstruction`, conversation
/// turns as `contents`. Tool results travel as `functionResponse`
/// parts on a `user` role turn (Gemini's convention; the API
/// doesn't accept `tool` as a role).
fn build_contents(msgs: &[Message]) -> (Option<GeminiSystemInstruction>, Vec<GeminiContent<'_>>) {
    let mut system_chunks: Vec<&str> = Vec::new();
    let mut out: Vec<GeminiContent<'_>> = Vec::new();
    for m in msgs {
        match m.role {
            Role::System => system_chunks.push(&m.content),
            Role::User => out.push(GeminiContent {
                role: "user",
                parts: vec![GeminiPart::Text { text: &m.content }],
            }),
            Role::Assistant => {
                let mut parts: Vec<GeminiPart<'_>> = Vec::new();
                if !m.content.is_empty() {
                    parts.push(GeminiPart::Text { text: &m.content });
                }
                for tc in &m.tool_calls {
                    parts.push(GeminiPart::FunctionCall {
                        function_call: GeminiFunctionCallOut {
                            name: &tc.name,
                            args: &tc.args,
                        },
                    });
                }
                if parts.is_empty() {
                    // Gemini rejects empty parts; skip the turn.
                    continue;
                }
                out.push(GeminiContent {
                    role: "model",
                    parts,
                });
            }
            Role::Tool => {
                // Gemini's functionResponse needs the tool name (not
                // the synthesized id); recover it by walking back
                // through `out` for the matching prior `model` turn.
                let name = lookup_tool_name(&out, m.tool_call_id.as_deref())
                    .unwrap_or_else(|| "tool".to_owned());
                out.push(GeminiContent {
                    role: "user",
                    parts: vec![GeminiPart::FunctionResponse {
                        function_response: GeminiFunctionResponse {
                            name,
                            // Wrap the tool's string return value in
                            // an object: Gemini requires `response`
                            // to be a JSON object, not a bare string.
                            response: serde_json::json!({ "output": m.content }),
                        },
                    }],
                });
            }
        }
    }
    let system = if system_chunks.is_empty() {
        None
    } else {
        // Allocate a single owned string so the value survives the
        // function return. The non-System variants borrow from
        // `msgs`, which the caller still holds while the future is
        // pending.
        Some(GeminiSystemInstruction {
            parts: vec![GeminiSystemPart {
                text: system_chunks.join("\n\n"),
            }],
        })
    };
    (system, out)
}

/// Walk the most recent `model` turn for a functionCall whose
/// synthesized id matches `id`, and recover the tool name. Without
/// this, every Tool message would have to thread the name through
/// from the orchestrator.
fn lookup_tool_name(out: &[GeminiContent<'_>], id: Option<&str>) -> Option<String> {
    let id = id?;
    let position = id
        .strip_prefix("gemini-")
        .and_then(|s| s.parse::<usize>().ok())?;
    let last_model = out.iter().rev().find(|c| c.role == "model")?;
    let mut seen = 0usize;
    for part in &last_model.parts {
        if let GeminiPart::FunctionCall { function_call } = part {
            if seen == position {
                return Some(function_call.name.to_string());
            }
            seen += 1;
        }
    }
    None
}

/// Pull the JSON payload out of an SSE frame. Mirrors the
/// Anthropic helper: Gemini also emits `data: <json>\n\n` per event.
fn extract_data(frame: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(frame).ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            return Some(rest.trim_start().to_string());
        }
    }
    None
}

// ---- request wire types -------------------------------------------------

#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GeminiTools<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    role: &'a str,
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPart<'a> {
    Text {
        text: &'a str,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCallOut<'a>,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

#[derive(Serialize)]
struct GeminiFunctionCallOut<'a> {
    name: &'a str,
    args: &'a serde_json::Value,
}

#[derive(Serialize)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiSystemPart>,
}

#[derive(Serialize)]
struct GeminiSystemPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiTools<'a> {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDecl<'a>>,
}

#[derive(Serialize)]
struct GeminiFunctionDecl<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

// ---- streaming response wire types --------------------------------------

#[derive(Deserialize)]
struct GeminiResponseChunk {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiCandidateContent>,
    #[serde(default, rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "functionCall")]
    function_call: Option<GeminiFunctionCallIn>,
}

#[derive(Deserialize)]
struct GeminiFunctionCallIn {
    name: String,
    #[serde(default)]
    args: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_contents_extracts_system() {
        let msgs = vec![
            Message::system("you are helpful"),
            Message::user("hi"),
            Message::assistant("hello"),
        ];
        let (system, contents) = build_contents(&msgs);
        let system = system.expect("system instruction");
        assert_eq!(system.parts.len(), 1);
        assert_eq!(system.parts[0].text, "you are helpful");
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
    }

    #[test]
    fn build_contents_round_trips_function_call_and_response() {
        let msgs = vec![
            Message::user("list please"),
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: "gemini-0".into(),
                    name: "list_files".into(),
                    args: serde_json::json!({"prefix": "notes"}),
                }],
            },
            Message {
                role: Role::Tool,
                content: "[\"notes/a.md\"]".into(),
                tool_call_id: Some("gemini-0".into()),
                tool_calls: Vec::new(),
            },
        ];
        let (_, contents) = build_contents(&msgs);
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[2].role, "user");
        let serialized = serde_json::to_value(&contents[2]).unwrap();
        let part = &serialized["parts"][0]["functionResponse"];
        assert_eq!(part["name"], "list_files");
        assert_eq!(part["response"]["output"], "[\"notes/a.md\"]");
    }
}
