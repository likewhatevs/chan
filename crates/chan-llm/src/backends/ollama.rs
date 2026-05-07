//! Ollama backend (https://ollama.com).
//!
//! Talks to a local Ollama daemon over HTTP. Default base URL is
//! http://localhost:11434, overridable via the `OLLAMA_HOST` env
//! var (matches the Ollama CLI's convention).
//!
//! Wire format mirrors OpenAI more than Anthropic: messages are
//! `{role, content}`. We POST to `/api/chat` with `stream: true`
//! and receive newline-delimited JSON; each line is one chunk
//! whose `message.content` is the next bit of text. The final
//! line carries `done: true` and a `done_reason` we map to
//! `StopReason`.
//!
//! Tool support is omitted in this initial port. qwen2.5:14b
//! and llama3.1 both support it via the `tools` array on the
//! request, but the orchestration loop (assistant proposes
//! call -> host executes -> next turn carries the tool result)
//! lands in the follow-up commit. For now the backend handles
//! plain text completions.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::session::{Delta, Message, Role, SessionListener, StopReason};

use super::Backend;

pub const DEFAULT_URL: &str = "http://localhost:11434";

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
    async fn run(&self, messages: Vec<Message>, listener: Arc<dyn SessionListener>) {
        let body = ChatRequest {
            model: &self.model,
            messages: messages
                .iter()
                .map(|m| ChatMessage {
                    role: role_str(m.role),
                    content: &m.content,
                })
                .collect(),
            stream: true,
        };

        let url = format!("{}/api/chat", self.base_url);
        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                listener.on_error(format!("ollama request: {e}"));
                listener.on_done(StopReason::Error);
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            listener.on_error(format!("ollama {status}: {body}"));
            listener.on_done(StopReason::Error);
            return;
        }

        // Streaming: NDJSON. Each chunk is a complete JSON line.
        // Reqwest's bytes_stream yields arbitrary-sized chunks,
        // so we accumulate into a buffer and pull complete lines
        // out as they appear. The final line carries `done:true`.
        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut stop = StopReason::EndOfTurn;

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    listener.on_error(format!("ollama stream: {e}"));
                    listener.on_done(StopReason::Error);
                    return;
                }
            };
            buf.extend_from_slice(&chunk);
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line = &line[..line.len().saturating_sub(1)]; // strip trailing \n
                if line.is_empty() {
                    continue;
                }
                let parsed: ChatChunk = match serde_json::from_slice(line) {
                    Ok(p) => p,
                    Err(e) => {
                        listener.on_error(format!("ollama parse: {e}"));
                        listener.on_done(StopReason::Error);
                        return;
                    }
                };
                if let Some(msg) = parsed.message.as_ref() {
                    if !msg.content.is_empty() {
                        listener.on_delta(Delta {
                            text: msg.content.to_owned(),
                        });
                    }
                }
                if parsed.done {
                    stop = match parsed.done_reason.as_deref() {
                        Some("stop") | Some("end") => StopReason::EndOfTurn,
                        Some("length") => StopReason::MaxTokens,
                        Some("stop_sequence") => StopReason::StopSequence,
                        _ => StopReason::EndOfTurn,
                    };
                    break;
                }
            }
        }

        listener.on_done(stop);
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
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
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
}
