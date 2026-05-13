//! Anthropic Claude backend.
//!
//! Talks to <https://api.anthropic.com/v1/messages> directly over
//! HTTP; no SDK. Streams responses via SSE so the editor's prompt
//! shows tokens as they arrive.
//!
//! Wire format:
//!
//!   - `system` is a top-level string (not a message). chan-llm's
//!     `Role::System` entries are concatenated into it; the rest of
//!     the transcript stays in `messages`.
//!   - Tool calls come back as `tool_use` content blocks; their
//!     `input` is built up incrementally over `input_json_delta`
//!     events and parsed once `content_block_stop` arrives. Tool
//!     results from a previous turn go back as `tool_result` blocks
//!     inside a `user` message (Anthropic's convention; the
//!     orchestrator above still treats them as `Role::Tool`).
//!
//! Streaming SSE events of interest:
//!
//!   - `content_block_start`: marks a new block; we capture the
//!     tool id + name when the block is a tool_use.
//!   - `content_block_delta`: text_delta or input_json_delta.
//!   - `content_block_stop`: finalize a tool_use block (parse its
//!     accumulated JSON args).
//!   - `message_delta`: carries the final stop_reason.
//!   - `message_stop`: end of stream.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::LlmError;
use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::retry::{send_with_retry, RetryError, RetryPolicy};
use super::{Backend, Outcome};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const MODELS_ENDPOINT: &str = "https://api.anthropic.com/v1/models";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Per-turn output cap when the user hasn't pinned one in
/// `LlmConfig::max_tokens.anthropic`. 4096 tokens is plenty for
/// the chat replies and multi-tool exchanges chan does today, and
/// well below the per-model upper limits, so we don't surprise
/// users with a 429 from a runaway model. Reachable via
/// `default_max_tokens()` for callers who want the resolver value.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Public so `backends::build` can read the same default the
/// constructor does.
pub fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

/// Hard cap on the SSE re-assembly buffer. Real Anthropic frames
/// are kilobytes; if a frame ever grows past this we treat the
/// stream as broken (a buggy proxy or a hostile MITM) rather than
/// keep accumulating until we OOM.
const SSE_BUF_CAP_BYTES: usize = 1024 * 1024;

/// Hit `/v1/models` and return live model IDs. Used by the Settings
/// UI to populate the model dropdown when an API key is configured;
/// falls back to a curated list when the call fails. Pagination
/// follows Anthropic's `has_more` cursor, but the catalog is small
/// enough that one page is the typical case.
pub async fn list_models(api_key: &str) -> Result<Vec<String>, LlmError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client builds with default rustls config");
    let mut out: Vec<String> = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let mut req = client
            .get(MODELS_ENDPOINT)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .query(&[("limit", "100")]);
        if let Some(cursor) = &after {
            req = req.query(&[("after_id", cursor.as_str())]);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| LlmError::Http(format!("anthropic models: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let (body, _) = super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
            return Err(LlmError::Http(format!("anthropic models {status}: {body}")));
        }
        let page: ModelsPage = resp
            .json()
            .await
            .map_err(|e| LlmError::Http(format!("anthropic models decode: {e}")))?;
        for m in &page.data {
            out.push(m.id.clone());
        }
        if !page.has_more {
            break;
        }
        after = page.data.last().map(|m| m.id.clone());
        if after.is_none() {
            break;
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct ModelsPage {
    data: Vec<ModelInfo>,
    has_more: bool,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

pub struct AnthropicBackend {
    // Wrapped in Zeroizing so the heap allocation is overwritten
    // with zeros on Drop. Defense-in-depth: an attacker with a
    // post-mortem heap snapshot (core dump, swap, container reuse)
    // would otherwise still see the raw key.
    api_key: Zeroizing<String>,
    model: String,
    max_tokens: u32,
    client: reqwest::Client,
}

// Hand-rolled Debug so tracing / dbg! / panic-with-state never echoes
// the api_key. The field stays private; this redaction is the safety
// net for accidental `?backend` formatting.
impl std::fmt::Debug for AnthropicBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicBackend")
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .finish_non_exhaustive()
    }
}

impl AnthropicBackend {
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        // 5 minute timeout: tool-use loops can take a while when the
        // assistant is iterating through reads / searches before
        // composing a reply. Per-event latency is what matters for
        // UX (tokens arrive as they're generated); the timeout is
        // a safety net for a wedged upstream.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            // Identifies chan-llm in upstream usage logs; helps
            // operators tell chan traffic apart from generic SDK
            // traffic when debugging quotas or 429s.
            .user_agent(concat!("chan-llm/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds with default rustls config");
        Self {
            api_key: Zeroizing::new(api_key),
            model,
            max_tokens,
            client,
        }
    }
}

#[async_trait]
impl Backend for AnthropicBackend {
    async fn run(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let (system, wire_messages) = split_system(&messages);
        let body = AnthropicRequest {
            model: &self.model,
            max_tokens: self.max_tokens,
            stream: true,
            system: system.as_deref(),
            messages: &wire_messages,
            tools: &tools
                .iter()
                .map(|t| AnthropicTool {
                    name: t.name,
                    description: t.description,
                    input_schema: &t.parameters,
                })
                .collect::<Vec<_>>(),
        };

        // Serialize once; the closure passed to send_with_retry
        // builds a fresh RequestBuilder each attempt because reqwest
        // consumes RequestBuilder on send. Body bytes are reused.
        let body_bytes = match serde_json::to_vec(&body) {
            Ok(b) => b,
            Err(e) => {
                listener.on_error(format!("anthropic body: {e}"));
                return Outcome::error();
            }
        };
        let resp = match send_with_retry(
            || {
                self.client
                    .post(ENDPOINT)
                    .header("x-api-key", self.api_key.as_str())
                    .header("anthropic-version", ANTHROPIC_VERSION)
                    .header("content-type", "application/json")
                    .body(body_bytes.clone())
            },
            RetryPolicy::default(),
            &cancel,
            "anthropic",
        )
        .await
        {
            Ok(r) => r,
            Err(RetryError::Cancelled) => return Outcome::cancelled(String::new()),
            Err(RetryError::Network(msg)) => {
                listener.on_error(format!("anthropic request: {msg}"));
                return Outcome::error();
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            // Drain a capped slice of the body rather than calling
            // `resp.text()` (which would allocate the full body even
            // if upstream returned megabytes of HTML). 16 KiB is more
            // than enough for Anthropic's JSON error envelopes; the
            // truncation marker lets a reader notice when the upstream
            // returned something unexpectedly large.
            let (raw, truncated) =
                super::read_capped_text(resp, super::DEFAULT_BODY_CAP_BYTES).await;
            let snippet: String = raw.chars().take(800).collect();
            let suffix = if truncated { " (body truncated)" } else { "" };
            listener.on_error(format!("anthropic {status}: {snippet}{suffix}"));
            return Outcome::error();
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        // tool_use blocks are streamed across multiple deltas; we
        // index them by content-block index and finalize on
        // content_block_stop.
        let mut pending_tools: std::collections::BTreeMap<u32, PendingTool> =
            std::collections::BTreeMap::new();
        let mut completed_tools: Vec<ToolCall> = Vec::new();
        let mut stop = StopReason::EndOfTurn;

        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                // Drop the stream by returning early; reqwest cancels
                // the underlying connection when the response is
                // dropped at the end of this scope. Carry whatever
                // assistant text we'd already streamed so the host
                // can keep partial UX state if it wants.
                return Outcome::cancelled(assistant_text);
            }
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("anthropic stream: {e}"));
                    return Outcome::error();
                }
            };
            buf.extend_from_slice(&chunk);
            if buf.len() > SSE_BUF_CAP_BYTES {
                listener.on_error(format!(
                    "anthropic stream: re-assembly buffer exceeded {SSE_BUF_CAP_BYTES} bytes; dropping connection",
                ));
                return Outcome::error();
            }

            // SSE frames are separated by a blank line ("\n\n"). We
            // split on that boundary so partial frames stay in `buf`
            // for the next chunk.
            while let Some(end) = find_event_end(&buf) {
                let raw_event: Vec<u8> = buf.drain(..end).collect();
                // Skip the blank-line terminator that follows.
                let _ = buf.drain(..2.min(buf.len())).collect::<Vec<u8>>();
                let payload = match extract_data(&raw_event) {
                    Some(p) => p,
                    None => continue,
                };
                let parsed: SseEvent = match serde_json::from_str(&payload) {
                    Ok(p) => p,
                    Err(e) => {
                        // Single bad frame: log and continue. Some
                        // proxies inject keepalive lines that aren't
                        // valid JSON; bailing on the whole turn for
                        // a transient parse blip is the wrong trade.
                        tracing::warn!(?e, raw = %truncate_payload(&payload, 400), "anthropic parse: skipping bad frame");
                        continue;
                    }
                };
                match parsed {
                    SseEvent::ContentBlockStart {
                        index,
                        content_block,
                    } => {
                        if let ContentBlock::ToolUse { id, name } = content_block {
                            pending_tools.insert(
                                index,
                                PendingTool {
                                    id,
                                    name,
                                    args_buf: String::new(),
                                },
                            );
                        }
                    }
                    SseEvent::ContentBlockDelta { index, delta } => match delta {
                        BlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                listener.on_delta(Delta { text: text.clone() });
                                assistant_text.push_str(&text);
                                if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                                    listener.on_error(format!(
                                        "anthropic stream: assistant text exceeded {} bytes; aborting",
                                        super::ASSISTANT_TEXT_CAP_BYTES,
                                    ));
                                    return Outcome::error();
                                }
                            }
                        }
                        BlockDelta::InputJsonDelta { partial_json } => {
                            if let Some(t) = pending_tools.get_mut(&index) {
                                t.args_buf.push_str(&partial_json);
                            }
                        }
                    },
                    SseEvent::ContentBlockStop { index } => {
                        if let Some(t) = pending_tools.remove(&index) {
                            let args: serde_json::Value = if t.args_buf.is_empty() {
                                serde_json::json!({})
                            } else {
                                match serde_json::from_str(&t.args_buf) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        listener.on_error(format!(
                                            "anthropic tool-args parse: {e}; raw: {}",
                                            t.args_buf
                                        ));
                                        return Outcome::error();
                                    }
                                }
                            };
                            completed_tools.push(ToolCall {
                                id: t.id,
                                name: t.name,
                                args,
                            });
                        }
                    }
                    SseEvent::MessageDelta { delta } => {
                        if let Some(reason) = delta.stop_reason {
                            stop = parse_stop_reason(&reason);
                        }
                    }
                    SseEvent::MessageStart { .. }
                    | SseEvent::MessageStop
                    | SseEvent::Ping
                    | SseEvent::Other => {}
                    SseEvent::Error { error } => {
                        listener.on_error(format!(
                            "anthropic error event: {} {}",
                            error.kind, error.message
                        ));
                        return Outcome::error();
                    }
                }
            }
        }

        // Final-tail flush: if the connection closed without a
        // trailing blank line, the last frame is still in `buf` and
        // never went through the inner parser. Try one decode pass
        // before bailing so we don't silently drop a tail event.
        if !buf.is_empty() {
            if let Some(payload) = extract_data(&buf) {
                if let Ok(parsed) = serde_json::from_str::<SseEvent>(&payload) {
                    if let SseEvent::MessageDelta { delta } = parsed {
                        if let Some(reason) = delta.stop_reason {
                            stop = parse_stop_reason(&reason);
                        }
                    }
                } else {
                    tracing::warn!(
                        raw = %truncate_payload(&payload, 400),
                        "anthropic parse: discarded tail frame on stream close",
                    );
                }
            }
            buf.clear();
        }

        // tool_use stop_reason wins over text-only end_turn when the
        // assistant proposed any tools. Anthropic already sets the
        // header correctly in message_delta, but defending against
        // a missing reason keeps the orchestrator's loop honest.
        if !completed_tools.is_empty() {
            stop = StopReason::ToolUse;
        }

        Outcome {
            assistant_text,
            tool_calls: completed_tools,
            stop_reason: stop,
        }
    }
}

struct PendingTool {
    id: String,
    name: String,
    args_buf: String,
}

/// Concatenate all `Role::System` messages into a single Anthropic
/// `system` string (Anthropic doesn't take system as a message).
/// Translate the rest of the transcript: assistant turns carrying
/// tool_calls become content blocks with a tool_use block per call;
/// tool result messages flip to `user` role with a `tool_result`
/// block per Anthropic's convention.
fn split_system(msgs: &[Message]) -> (Option<String>, Vec<AnthropicMessage<'_>>) {
    let mut system_chunks: Vec<&str> = Vec::new();
    let mut out: Vec<AnthropicMessage<'_>> = Vec::new();
    for m in msgs {
        match m.role {
            Role::System => system_chunks.push(&m.content),
            Role::User => {
                // Anthropic wants images listed BEFORE the text in
                // a multimodal user message — the model reads the
                // visuals first then the instruction. We mirror
                // that ordering so prompts like "what's in this
                // image?" attach cleanly.
                let mut blocks: Vec<AnthropicMessageContent<'_>> = m
                    .images
                    .iter()
                    .map(|img| AnthropicMessageContent::Image {
                        source: AnthropicImageSource::Base64 {
                            media_type: &img.mime_type,
                            data: &img.data,
                        },
                    })
                    .collect();
                if !m.content.is_empty() || blocks.is_empty() {
                    blocks.push(AnthropicMessageContent::Text { text: &m.content });
                }
                out.push(AnthropicMessage {
                    role: "user",
                    content: blocks,
                });
            }
            Role::Assistant => {
                let mut blocks = Vec::new();
                if !m.content.is_empty() {
                    blocks.push(AnthropicMessageContent::Text { text: &m.content });
                }
                for tc in &m.tool_calls {
                    blocks.push(AnthropicMessageContent::ToolUse {
                        id: &tc.id,
                        name: &tc.name,
                        input: &tc.args,
                    });
                }
                out.push(AnthropicMessage {
                    role: "assistant",
                    content: blocks,
                });
            }
            Role::Tool => {
                let id = m.tool_call_id.as_deref().unwrap_or("");
                out.push(AnthropicMessage {
                    role: "user",
                    content: vec![AnthropicMessageContent::ToolResult {
                        tool_use_id: id,
                        content: &m.content,
                    }],
                });
            }
        }
    }
    let system = if system_chunks.is_empty() {
        None
    } else {
        Some(system_chunks.join("\n\n"))
    };
    (system, out)
}

fn parse_stop_reason(s: &str) -> StopReason {
    match s {
        "end_turn" => StopReason::EndOfTurn,
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "stop_sequence" => StopReason::StopSequence,
        _ => StopReason::EndOfTurn,
    }
}

/// Find the byte index where the next SSE event ends (the "\n\n"
/// separator). Returns the index of the FIRST `\n` of the pair so
/// the caller can drain the event payload + the newlines together.
fn find_event_end(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

/// Pull the JSON payload out of an SSE frame. Anthropic frames look
/// like `event: <type>\ndata: <json>` (with the optional `event:`
/// line we ignore; the JSON's own `type` field is the source of
/// truth). Per SSE spec, multiple `data:` lines within one frame
/// concatenate with `\n` to form the final payload. Anthropic
/// doesn't currently use multi-line data, but a future release or
/// an intermediary proxy might; the spec-compliant join keeps us
/// safe either way.
fn extract_data(frame: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(frame).ok()?;
    let mut out: Option<String> = None;
    for line in s.lines() {
        let Some(rest) = line.strip_prefix("data:") else {
            continue;
        };
        // SSE's "data: <value>" optional-leading-space rule.
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

/// Truncate a string at `max` chars for log lines. Cheap O(n) walk
/// that respects char boundaries, matches the helper in claude_cli.
fn truncate_payload(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

// ---- request wire types -------------------------------------------------

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: &'a [AnthropicMessage<'a>],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tools: &'a [AnthropicTool<'a>],
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: Vec<AnthropicMessageContent<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicMessageContent<'a> {
    Text {
        text: &'a str,
    },
    /// Base64-encoded image attached to a user message. Anthropic
    /// expects `{ "type": "image", "source": { "type": "base64",
    /// "media_type": "image/png", "data": "..." } }`. We accept
    /// the standard set of MIMEs Anthropic supports (png, jpeg,
    /// webp, gif); callers are responsible for clamping size
    /// (model-side cap is 5 MiB / image, 20 images / request).
    Image {
        source: AnthropicImageSource<'a>,
    },
    ToolUse {
        id: &'a str,
        name: &'a str,
        input: &'a serde_json::Value,
    },
    ToolResult {
        tool_use_id: &'a str,
        content: &'a str,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicImageSource<'a> {
    Base64 {
        media_type: &'a str,
        data: &'a str,
    },
}

#[derive(Serialize)]
struct AnthropicTool<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: &'a serde_json::Value,
}

// ---- streaming response wire types --------------------------------------

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SseEvent {
    MessageStart {
        // We don't read anything off message_start today (id, model,
        // usage on the start frame); reserved for future telemetry.
        #[allow(dead_code)]
        message: serde_json::Value,
    },
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: BlockDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: MessageDeltaPayload,
    },
    MessageStop,
    Ping,
    Error {
        error: AnthropicError,
    },
    /// Future event types we don't model yet pass through silently.
    /// Anthropic occasionally introduces new SSE events; surfacing
    /// them as parse errors would break compatibility.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        // Text blocks may arrive with a (typically empty) starting
        // text. We don't push it through on_delta because the
        // subsequent text_delta carries the actual content; we just
        // need to know the block exists.
        #[allow(dead_code)]
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        // The starting `input` is always `{}`. The real input is
        // streamed in input_json_delta events.
    },
    /// Forward-compat catch-all (e.g. future thinking blocks).
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize)]
struct MessageDeltaPayload {
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::extract_data;

    #[test]
    fn extract_data_single_line() {
        let frame = b"data: {\"hello\":1}";
        assert_eq!(extract_data(frame).as_deref(), Some("{\"hello\":1}"));
    }

    #[test]
    fn extract_data_concatenates_multiple_data_lines() {
        // Per SSE spec, multiple data: lines join with '\n'.
        let frame = b"data: {\"a\":1\ndata: ,\"b\":2}";
        // Resulting payload should be "{\"a\":1\n,\"b\":2}".
        let got = extract_data(frame).expect("data");
        assert_eq!(got, "{\"a\":1\n,\"b\":2}");
    }

    #[test]
    fn extract_data_strips_optional_leading_space() {
        let with_space = b"data: hello";
        let without = b"data:hello";
        assert_eq!(extract_data(with_space).as_deref(), Some("hello"));
        assert_eq!(extract_data(without).as_deref(), Some("hello"));
    }

    #[test]
    fn extract_data_returns_none_when_no_data_line() {
        let frame = b"event: ping\n: comment";
        assert!(extract_data(frame).is_none());
    }
}
