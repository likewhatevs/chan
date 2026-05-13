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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::retry::{send_with_retry, RetryError, RetryPolicy};
use super::{Backend, Outcome};

pub const DEFAULT_URL: &str = "http://localhost:11434";

/// Hard cap on the NDJSON re-assembly buffer. Ollama lines are
/// small in practice; past this we treat the stream as broken
/// rather than accumulate without bound.
const NDJSON_BUF_CAP_BYTES: usize = 1024 * 1024;

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
        let (body, _) = super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
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
    /// Maps to Ollama's `options.num_predict`. None = no cap (the
    /// upstream default; Ollama generates until the model emits a
    /// stop token or context fills up).
    max_tokens: Option<u32>,
    client: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(base_url: String, model: String, max_tokens: Option<u32>) -> Self {
        // Reject unexpected schemes early. reqwest would fail at
        // request time anyway, but the error message ("relative URL
        // without a base") is opaque; this keeps the user-visible
        // failure on misconfigured `OLLAMA_HOST` clear. http and
        // https only; anything else (file://, ssh://, ws://) is
        // either a typo or an attempt to do something we don't
        // support.
        let base_for_check = base_url.trim();
        if !(base_for_check.starts_with("http://") || base_for_check.starts_with("https://")) {
            tracing::warn!(
                base = %base_url,
                "ollama: base URL has unsupported scheme; expected http:// or https://",
            );
        }
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
            max_tokens,
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
        cancel: Arc<AtomicBool>,
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
                    // Ollama's chat API takes images as a top-level
                    // `images` array on the message, holding the raw
                    // base64 payload (no `data:` prefix). Only the
                    // multimodal models (llava, llama3.2-vision, etc.)
                    // act on them; the rest ignore the field.
                    images: m.images.iter().map(|img| img.data.as_str()).collect(),
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
            options: self.max_tokens.map(|n| ChatOptions {
                num_predict: Some(n),
            }),
        };

        let url = format!("{}/api/chat", self.base_url);
        let body_bytes = match serde_json::to_vec(&body) {
            Ok(b) => b,
            Err(e) => {
                listener.on_error(format!("ollama body: {e}"));
                return Outcome::error();
            }
        };
        let resp = match send_with_retry(
            || {
                self.client
                    .post(&url)
                    .header("content-type", "application/json")
                    .body(body_bytes.clone())
            },
            RetryPolicy::default(),
            &cancel,
            "ollama",
        )
        .await
        {
            Ok(r) => r,
            Err(RetryError::Cancelled) => return Outcome::cancelled(String::new()),
            Err(RetryError::Network(msg)) => {
                listener.on_error(format!("ollama request: {msg}"));
                return Outcome::error();
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let (body, truncated) =
                super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
            let snippet: String = body.chars().take(800).collect();
            let suffix = if truncated { " (body truncated)" } else { "" };
            listener.on_error(format!("ollama {status}: {snippet}{suffix}"));
            return Outcome::error();
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut stop = StopReason::EndOfTurn;

        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                return Outcome::cancelled(assistant_text);
            }
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("ollama stream: {e}"));
                    return Outcome::error();
                }
            };
            buf.extend_from_slice(&chunk);
            if buf.len() > NDJSON_BUF_CAP_BYTES {
                listener.on_error(format!(
                    "ollama stream: re-assembly buffer exceeded {NDJSON_BUF_CAP_BYTES} bytes; dropping connection",
                ));
                return Outcome::error();
            }
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line = &line[..line.len().saturating_sub(1)];
                if line.is_empty() {
                    continue;
                }
                let parsed: ChatChunk = match serde_json::from_slice(line) {
                    Ok(p) => p,
                    Err(e) => {
                        // Single bad line: log and keep reading.
                        // Future Ollama versions sometimes interleave
                        // status lines that don't match ChatChunk.
                        tracing::warn!(?e, "ollama parse: skipping bad line");
                        continue;
                    }
                };
                if let Some(msg) = parsed.message.as_ref() {
                    if !msg.content.is_empty() {
                        listener.on_delta(Delta {
                            text: msg.content.clone(),
                        });
                        assistant_text.push_str(&msg.content);
                        if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                            listener.on_error(format!(
                                "ollama stream: assistant text exceeded {} bytes; aborting",
                                super::ASSISTANT_TEXT_CAP_BYTES,
                            ));
                            return Outcome::error();
                        }
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

        // Final-tail flush: NDJSON streams that end without a
        // trailing newline leave the last record in `buf`. Try one
        // decode pass before returning so we don't drop the final
        // chunk.
        if !buf.is_empty() {
            match serde_json::from_slice::<ChatChunk>(&buf) {
                Ok(parsed) => {
                    if let Some(msg) = parsed.message.as_ref() {
                        if !msg.content.is_empty() {
                            listener.on_delta(Delta {
                                text: msg.content.clone(),
                            });
                            assistant_text.push_str(&msg.content);
                        }
                        for (idx, tc) in msg.tool_calls.iter().enumerate() {
                            let id = format!("call-{}", tool_calls.len() + idx);
                            tool_calls.push(ToolCall {
                                id,
                                name: tc.function.name.clone(),
                                args: tc.function.arguments.clone(),
                            });
                        }
                    }
                    if parsed.done && !tool_calls.is_empty() {
                        stop = StopReason::ToolUse;
                    }
                }
                Err(e) => {
                    tracing::warn!(?e, "ollama parse: discarded tail line on stream close");
                }
            }
            buf.clear();
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
    /// Ollama's per-request `options` table; only populated when
    /// the user pinned a max-tokens value, otherwise we omit the
    /// field so Ollama uses its default. Other generation params
    /// (temperature, top_p, etc.) live here too and can be added
    /// the same way when chan-llm starts surfacing them.
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Serialize)]
struct ChatOptions {
    /// Ollama wire name. -1 means "no cap"; we never emit that
    /// because `None` already encodes the same intent.
    #[serde(rename = "num_predict", skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<OutToolCall<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    images: Vec<&'a str>,
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
