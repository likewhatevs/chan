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
//!     `gemini-<turn>-<idx>` ids per turn position. `<turn>` is the
//!     0-based index of the model turn this call belongs to, counted
//!     against the assistant messages already in the transcript at
//!     request time; `<idx>` is the call's position inside that
//!     turn. Including the turn index keeps ids unique across the
//!     whole session, so a tool result from an earlier turn cannot
//!     collide with a later turn that happened to emit the same
//!     positional `<idx>`. `lookup_tool_name` mirrors this layout
//!     when re-serializing tool results back to Gemini.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::LlmError;
use crate::session::{Delta, LlmEventError, Message, Role, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::retry::{send_with_retry, RetryError, RetryPolicy};
use super::{Backend, Outcome};

const ENDPOINT_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default per-turn output cap when the user hasn't pinned one in
/// `LlmConfig::max_tokens.gemini`.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;

pub fn default_max_output_tokens() -> u32 {
    DEFAULT_MAX_OUTPUT_TOKENS
}

/// Hard cap on the SSE re-assembly buffer. See `anthropic.rs` for
/// rationale; Gemini frames are similarly small in practice.
const SSE_BUF_CAP_BYTES: usize = 1024 * 1024;

pub struct GeminiBackend {
    // Wrapped in Zeroizing so the heap allocation is overwritten
    // with zeros on Drop. Same rationale as `AnthropicBackend`.
    api_key: Zeroizing<String>,
    model: String,
    max_output_tokens: u32,
    client: reqwest::Client,
}

// Hand-rolled Debug so tracing / dbg! / panic-with-state never echoes
// the api_key. Same rationale as `AnthropicBackend`.
impl std::fmt::Debug for GeminiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiBackend")
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("max_output_tokens", &self.max_output_tokens)
            .finish_non_exhaustive()
    }
}

impl GeminiBackend {
    pub fn new(api_key: String, model: String, max_output_tokens: u32) -> Self {
        // 5 minute timeout: same headroom as the other backends for
        // tool-use loops with iterated reads / searches.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .user_agent(concat!("chan-llm/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds with default rustls config");
        Self {
            api_key: Zeroizing::new(api_key),
            model,
            max_output_tokens,
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
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        // 0-based index of the model turn we're about to produce.
        // Used as the `<turn>` field in synthesized tool-call ids so
        // they stay unique across the whole session, not just the
        // current turn.
        let turn_index = messages
            .iter()
            .filter(|m| matches!(m.role, Role::Assistant))
            .count();
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
                max_output_tokens: Some(self.max_output_tokens),
            },
        };

        let url = format!(
            "{ENDPOINT_BASE}/models/{}:streamGenerateContent?alt=sse",
            self.model
        );
        let body_bytes = match serde_json::to_vec(&body) {
            Ok(b) => b,
            Err(e) => {
                listener.on_error(format!("gemini body: {e}"));
                return Outcome::error();
            }
        };
        let resp = match send_with_retry(
            || {
                // x-goog-api-key keeps the secret out of the URL (and
                // out of access logs) compared to ?key=.
                self.client
                    .post(&url)
                    .header("x-goog-api-key", self.api_key.as_str())
                    .header("content-type", "application/json")
                    .body(body_bytes.clone())
            },
            RetryPolicy::default(),
            &cancel,
            "gemini",
        )
        .await
        {
            Ok(r) => r,
            Err(RetryError::Cancelled) => return Outcome::cancelled(String::new()),
            Err(RetryError::Network(msg)) => {
                listener.on_error_kind(LlmEventError::BackendUnreachable {
                    backend: "gemini".into(),
                    message: msg,
                });
                return Outcome::error();
            }
        };

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let retry_after_secs = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            // See anthropic.rs: cap the body read so a runaway upstream
            // can't force a multi-GB allocation on the error path.
            let (raw, _truncated) =
                super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
            let kind = super::classify_http_error("gemini", status, &raw, retry_after_secs);
            listener.on_error_kind(kind);
            return Outcome::error();
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                return Outcome::cancelled(assistant_text);
            }
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("gemini stream: {e}"));
                    return Outcome::error();
                }
            };
            buf.extend_from_slice(&chunk);
            if buf.len() > SSE_BUF_CAP_BYTES {
                listener.on_error(format!(
                    "gemini stream: re-assembly buffer exceeded {SSE_BUF_CAP_BYTES} bytes; dropping connection",
                ));
                return Outcome::error();
            }
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
                        // Single bad frame: log and continue. Same
                        // rationale as the anthropic backend.
                        tracing::warn!(?e, raw = %truncate_payload(&payload, 400), "gemini parse: skipping bad frame");
                        continue;
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
                                    if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                                        listener.on_error(format!(
                                            "gemini stream: assistant text exceeded {} bytes; aborting",
                                            super::ASSISTANT_TEXT_CAP_BYTES,
                                        ));
                                        return Outcome::error();
                                    }
                                }
                            }
                            if let Some(call) = part.function_call {
                                // Synthesize a stable id from
                                // (turn_index, position in this
                                // turn). The orchestrator pairs the
                                // next Role::Tool message via this
                                // id; build_contents recovers the
                                // tool name by walking to the matching
                                // `model` turn.
                                let id = format!("gemini-{}-{}", turn_index, tool_calls.len());
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

        // Final-tail flush: pick up the last frame if the connection
        // closed without a trailing blank line. We only care about
        // finish_reason here; deltas already streamed via on_delta.
        if !buf.is_empty() {
            if let Some(payload) = extract_data(&buf) {
                if let Ok(parsed) = serde_json::from_str::<GeminiResponseChunk>(&payload) {
                    if let Some(candidate) = parsed.candidates.into_iter().next() {
                        if let Some(reason) = candidate.finish_reason {
                            finish_reason = Some(reason);
                        }
                    }
                } else {
                    tracing::warn!(
                        raw = %truncate_payload(&payload, 400),
                        "gemini parse: discarded tail frame on stream close",
                    );
                }
            }
            buf.clear();
        }

        let stop_reason = if !tool_calls.is_empty() {
            StopReason::ToolUse
        } else {
            match finish_reason.as_deref() {
                Some("STOP") | None => StopReason::EndOfTurn,
                Some("MAX_TOKENS") => StopReason::MaxTokens,
                Some("STOP_SEQUENCE") => StopReason::StopSequence,
                // Content-policy / safety blocks: the model stopped
                // because the upstream refused to keep generating.
                // Map to Error and surface the raw reason so the host
                // can show an actionable state instead of pretending
                // the turn ended naturally.
                Some(
                    other @ ("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII"
                    | "IMAGE_SAFETY"),
                ) => {
                    listener.on_error_kind(LlmEventError::Other {
                        backend: "gemini".into(),
                        message: format!("model stopped early: {other}"),
                    });
                    StopReason::Error
                }
                // Forward-compat: any other unknown reason from
                // upstream lands here. Surface as Error + warn so we
                // notice the divergence rather than silently hiding it
                // as EndOfTurn.
                Some(other) => {
                    tracing::warn!(
                        finish_reason = %other,
                        "gemini: unknown finishReason; mapping to Error",
                    );
                    listener.on_error_kind(LlmEventError::Other {
                        backend: "gemini".into(),
                        message: format!("unknown finishReason: {other}"),
                    });
                    StopReason::Error
                }
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
            let (body, _) = super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
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
                // If we can't recover the name (broken transcript,
                // host stitched together turns we never saw), skip
                // the message rather than send `name: "tool"` which
                // Gemini rejects as an unknown function.
                let Some(name) = lookup_tool_name(&out, m.tool_call_id.as_deref()) else {
                    tracing::warn!(
                        id = ?m.tool_call_id,
                        "gemini: dropping tool result with no matching prior functionCall",
                    );
                    continue;
                };
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

/// Resolve a synthesized `gemini-<turn>-<idx>` id back to the tool
/// name by walking `out` to the `<turn>`th `model` turn (0-based)
/// and indexing into its functionCall parts at `<idx>`. Without this
/// every Tool message would have to thread the name through from the
/// orchestrator.
///
/// Falls back to the legacy `gemini-<idx>` shape (no turn segment)
/// for back-compat with transcripts produced by older builds: those
/// can only be resolved against the most recent model turn, which
/// matches the prior behaviour.
fn lookup_tool_name(out: &[GeminiContent<'_>], id: Option<&str>) -> Option<String> {
    let rest = id?.strip_prefix("gemini-")?;
    let target_turn: Option<usize>;
    let position: usize;
    match rest.split_once('-') {
        Some((turn_s, pos_s)) => {
            target_turn = Some(turn_s.parse().ok()?);
            position = pos_s.parse().ok()?;
        }
        None => {
            target_turn = None;
            position = rest.parse().ok()?;
        }
    }
    let model_turn = match target_turn {
        Some(t) => out.iter().filter(|c| c.role == "model").nth(t)?,
        None => out.iter().rev().find(|c| c.role == "model")?,
    };
    let mut seen = 0usize;
    for part in &model_turn.parts {
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
/// Multiple `data:` lines per frame concatenate with `\n` per the
/// SSE spec; Gemini occasionally splits longer payloads that way.
fn extract_data(frame: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(frame).ok()?;
    let mut out: Option<String> = None;
    for line in s.lines() {
        let Some(rest) = line.strip_prefix("data:") else {
            continue;
        };
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        match &mut out {
            Some(acc) => {
                acc.push('\n');
                acc.push_str(rest);
            }
            None => out = Some(rest.to_string()),
        }
    }
    out
}

fn truncate_payload(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
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
    fn build_contents_drops_orphan_tool_result() {
        // Tool result with an id that doesn't match any prior
        // functionCall in the transcript. The previous behavior
        // synthesized name="tool" which Gemini rejects; the new
        // behavior drops the message and logs.
        let msgs = vec![
            Message::user("hi"),
            Message {
                role: Role::Tool,
                content: "ghost result".into(),
                tool_call_id: Some("never-existed".into()),
                tool_calls: Vec::new(),
                images: Vec::new(),
            },
        ];
        let (_, contents) = build_contents(&msgs);
        // Only the user turn survives; the orphan Tool message is
        // dropped rather than sent with name="tool".
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "user");
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
                images: Vec::new(),
            },
            Message {
                role: Role::Tool,
                content: "[\"notes/a.md\"]".into(),
                tool_call_id: Some("gemini-0".into()),
                tool_calls: Vec::new(),
                images: Vec::new(),
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

    #[test]
    fn lookup_resolves_cross_turn_ids_without_collision() {
        // Two model turns each emit a call at position 0. Old ids
        // ("gemini-0") would collide and route turn-0's tool result
        // to turn-1's tool name. New ids embed the turn index so the
        // lookup walks to the correct model turn.
        let msgs = vec![
            Message::user("first"),
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: "gemini-0-0".into(),
                    name: "read_file".into(),
                    args: serde_json::json!({"path": "a.md"}),
                }],
                images: Vec::new(),
            },
            Message {
                role: Role::Tool,
                content: "alpha".into(),
                tool_call_id: Some("gemini-0-0".into()),
                tool_calls: Vec::new(),
                images: Vec::new(),
            },
            Message::user("now do another"),
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: "gemini-1-0".into(),
                    name: "list_files".into(),
                    args: serde_json::json!({"prefix": "notes"}),
                }],
                images: Vec::new(),
            },
            Message {
                role: Role::Tool,
                content: "[]".into(),
                tool_call_id: Some("gemini-1-0".into()),
                tool_calls: Vec::new(),
                images: Vec::new(),
            },
        ];
        let (_, contents) = build_contents(&msgs);
        // user, model, user(=tool result), user, model, user(=tool result)
        assert_eq!(contents.len(), 6);
        let first_response = serde_json::to_value(&contents[2]).unwrap();
        assert_eq!(
            first_response["parts"][0]["functionResponse"]["name"],
            "read_file"
        );
        let second_response = serde_json::to_value(&contents[5]).unwrap();
        assert_eq!(
            second_response["parts"][0]["functionResponse"]["name"],
            "list_files"
        );
    }

    #[test]
    fn lookup_back_compat_legacy_id_uses_last_model_turn() {
        // Legacy "gemini-0" id (pre cross-turn fix). For transcripts
        // produced by older builds, the lookup falls back to the
        // most recent model turn.
        let msgs = vec![
            Message::user("hello"),
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: "gemini-0".into(),
                    name: "search_content".into(),
                    args: serde_json::json!({"q": "foo"}),
                }],
                images: Vec::new(),
            },
            Message {
                role: Role::Tool,
                content: "no hits".into(),
                tool_call_id: Some("gemini-0".into()),
                tool_calls: Vec::new(),
                images: Vec::new(),
            },
        ];
        let (_, contents) = build_contents(&msgs);
        assert_eq!(contents.len(), 3);
        let resp = serde_json::to_value(&contents[2]).unwrap();
        assert_eq!(
            resp["parts"][0]["functionResponse"]["name"],
            "search_content"
        );
    }
}
