//! Ollama backend (https://ollama.com).
//!
//! Talks to a local Ollama daemon over HTTP. Default base URL is
//! http://localhost:11434, overridable via the `OLLAMA_HOST` env
//! var (matches the Ollama CLI's convention).
//!
//! Wire format is OpenAI-shaped: messages are `{role, content}`,
//! tools are `{type: "function", function: {name, description,
//! parameters}}`, and tool calls come back as
//! `message.tool_calls = [{function: {name, arguments}}]` with
//! `arguments` already JSON-decoded.
//!
//! Streaming: POST `/api/chat` with `stream: true`. Response is
//! NDJSON; each line is one chunk whose `message.content` is the
//! next bit of text. The final line carries `done: true`, a
//! `done_reason`, and (when the assistant proposed tool calls)
//! `message.tool_calls`. We accumulate text into `assistant_text`,
//! collect tool_calls from the final chunk, and let the
//! session-level orchestration loop drive the next turn.
//!
//! Tool support is per-model. qwen2.5 / llama3.1 (and most modern
//! models in the Ollama catalog) handle tools; older models
//! ignore the array and return text only. We unconditionally
//! send tools and let the model decide; if your model doesn't
//! support them, you just get text-only completions.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::{Backend, Outcome};

pub const DEFAULT_URL: &str = "http://localhost:11434";

/// Hit the Ollama daemon's `/api/tags` endpoint and return every
/// installed model's name. Used by the Settings UI to populate the
/// Ollama model dropdown without the user having to type a model
/// name by hand. `base_url` is whatever the caller resolved (env
/// `OLLAMA_HOST` > `LlmConfig.urls.ollama` > `DEFAULT_URL`).
///
/// Returns an empty list when the daemon is reachable but has no
/// models installed; errors when the daemon isn't reachable so
/// the Settings UI can surface "ollama: connection refused" copy
/// instead of an unexplained empty dropdown.
pub async fn list_models(base_url: &str) -> Result<Vec<String>, crate::error::LlmError> {
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        // Tags is a tiny list response; fail fast so the Settings
        // UI doesn't hang for 5 minutes when ollama isn't running.
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client builds with default rustls config");
    let url = format!("{base}/api/tags");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| crate::error::LlmError::Http(format!("ollama tags: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(crate::error::LlmError::Http(format!(
            "ollama tags {status}: {body}"
        )));
    }
    #[derive(Deserialize)]
    struct TagsResponse {
        #[serde(default)]
        models: Vec<TagEntry>,
    }
    #[derive(Deserialize)]
    struct TagEntry {
        name: String,
    }
    let parsed: TagsResponse = resp
        .json()
        .await
        .map_err(|e| crate::error::LlmError::Http(format!("ollama tags decode: {e}")))?;
    Ok(parsed.models.into_iter().map(|m| m.name).collect())
}

#[derive(Debug)]
pub struct OllamaBackend {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(base_url: String, model: String) -> Self {
        let base = base_url.trim_end_matches('/').to_owned();
        let client = reqwest::Client::builder()
            // Generous timeout: a 14B-class model on CPU can take
            // a minute to start producing tokens. Streaming means
            // the per-chunk latency is what matters for UX, not
            // the total wall time.
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client builds with default rustls config");
        Self {
            base_url: base,
            model,
            client,
        }
    }
}

#[async_trait]
impl Backend for OllamaBackend {
    async fn run(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        listener: Arc<dyn SessionListener>,
    ) -> Outcome {
        let body = ChatRequest {
            model: &self.model,
            messages: messages
                .iter()
                .map(|m| ChatMessage {
                    role: role_str(m.role),
                    content: &m.content,
                    tool_calls: m
                        .tool_calls
                        .iter()
                        .map(|c| OutToolCall {
                            function: OutFunction {
                                name: &c.name,
                                arguments: &c.args,
                            },
                        })
                        .collect(),
                })
                .collect(),
            tools: tools
                .iter()
                .map(|t| ToolWire {
                    kind: "function",
                    function: ToolFnWire {
                        name: t.name,
                        description: t.description,
                        parameters: &t.parameters,
                    },
                })
                .collect(),
            stream: true,
        };

        let url = format!("{}/api/chat", self.base_url);
        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                listener.on_error(format!("ollama request: {e}"));
                return Outcome::error();
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            listener.on_error(format!("ollama {status}: {body}"));
            return Outcome::error();
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut stop = StopReason::EndOfTurn;

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("ollama stream: {e}"));
                    return Outcome::error();
                }
            };
            buf.extend_from_slice(&chunk);
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line = &line[..line.len().saturating_sub(1)];
                if line.is_empty() {
                    continue;
                }
                let parsed: ChatChunk = match serde_json::from_slice(line) {
                    Ok(p) => p,
                    Err(e) => {
                        listener.on_error(format!("ollama parse: {e}"));
                        return Outcome::error();
                    }
                };
                if let Some(msg) = parsed.message.as_ref() {
                    if !msg.content.is_empty() {
                        listener.on_delta(Delta {
                            text: msg.content.clone(),
                        });
                        assistant_text.push_str(&msg.content);
                    }
                    // Tool calls only appear in the final chunk
                    // for streaming responses, but we accumulate
                    // defensively in case a future Ollama version
                    // streams them too.
                    for (idx, tc) in msg.tool_calls.iter().enumerate() {
                        let id = format!("call-{}", tool_calls.len() + idx);
                        tool_calls.push(ToolCall {
                            id,
                            name: tc.function.name.clone(),
                            args: tc.function.arguments.clone(),
                        });
                    }
                }
                if parsed.done {
                    stop = if !tool_calls.is_empty() {
                        StopReason::ToolUse
                    } else {
                        match parsed.done_reason.as_deref() {
                            Some("stop") | Some("end") | None => StopReason::EndOfTurn,
                            Some("length") => StopReason::MaxTokens,
                            Some("stop_sequence") => StopReason::StopSequence,
                            _ => StopReason::EndOfTurn,
                        }
                    };
                    break;
                }
            }
        }

        Outcome {
            assistant_text,
            tool_calls,
            stop_reason: stop,
        }
    }
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

// ---- wire types ---------------------------------------------------------

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolWire<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<OutToolCall<'a>>,
}

#[derive(Serialize)]
struct OutToolCall<'a> {
    function: OutFunction<'a>,
}

#[derive(Serialize)]
struct OutFunction<'a> {
    name: &'a str,
    arguments: &'a serde_json::Value,
}

#[derive(Serialize)]
struct ToolWire<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    function: ToolFnWire<'a>,
}

#[derive(Serialize)]
struct ToolFnWire<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct ChatChunk {
    #[serde(default)]
    message: Option<ChunkMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChunkMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<InToolCall>,
}

#[derive(Deserialize)]
struct InToolCall {
    function: InFunction,
}

#[derive(Deserialize)]
struct InFunction {
    name: String,
    /// Ollama returns arguments as a parsed JSON value (not a
    /// string), so we accept whatever serde_json gives us.
    #[serde(default)]
    arguments: serde_json::Value,
}
