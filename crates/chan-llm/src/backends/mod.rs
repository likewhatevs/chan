// Backend dispatch.
//
// Three backends in scope today:
//   - Anthropic (Claude)        - stub; ports next
//   - Gemini (Google)           - stub; ports next
//   - Ollama (local server)     - real
//
// Each provides a `Backend` impl that owns its HTTP client config
// (auth header style, base URL, model defaults) and translates
// chan-llm's internal `Message` list into the backend's wire format,
// then drives the streaming response, dispatching events into the
// `SessionListener` the caller supplied.
//
// Backends DO NOT touch the filesystem or chan-core directly. The
// tool sandbox sits between them and disk: the assistant proposes
// a tool call, chan-llm relays it to the host via on_tool_call,
// the host runs `tools::execute` against chan-core::Drive, and the
// next turn's transcript carries the tool result. Backends only
// translate one HTTP exchange per turn.

pub mod anthropic;
pub mod gemini;
pub mod ollama;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;
use crate::error::{LlmError, Result};
use crate::keys;
use crate::session::{Message, SessionListener};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    Anthropic,
    Gemini,
    Ollama,
}

impl BackendKind {
    pub fn name(self) -> &'static str {
        match self {
            BackendKind::Anthropic => "anthropic",
            BackendKind::Gemini => "gemini",
            BackendKind::Ollama => "ollama",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            BackendKind::Anthropic => "claude-opus-4-7",
            BackendKind::Gemini => "gemini-2.5-pro",
            BackendKind::Ollama => "llama3.1",
        }
    }
}

/// What every backend implements. `run` drives one HTTP exchange:
/// translate `messages` + `tools` to wire format, stream the
/// response, emit text deltas via `on_delta` (and `on_error` on
/// failure) into the listener, then return an `Outcome` so the
/// session-level orchestration loop can decide whether to
/// dispatch tool calls and continue.
///
/// Backends do NOT emit `on_tool_call`, `on_tool_result`, or
/// `on_done` themselves. Those are the orchestration loop's
/// concern (in `session.rs::send`); a backend is just one HTTP
/// turn translated to chan-llm's event vocabulary.
#[async_trait]
pub trait Backend: Send + Sync {
    async fn run(
        &self,
        messages: Vec<Message>,
        tools: Vec<crate::tools::ToolSchema>,
        listener: Arc<dyn SessionListener>,
    ) -> Outcome;
}

/// What the backend collected during one HTTP exchange. The
/// session-level loop consumes this to decide the next step:
///   - tool_calls non-empty -> run them, append results to the
///     transcript, call backend.run again
///   - tool_calls empty     -> emit `on_done(stop_reason)`, end
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Accumulated assistant text (the concatenation of every
    /// streaming delta this turn). Backends emit each chunk via
    /// `on_delta` AND keep a running buffer here so the loop can
    /// reconstruct the full assistant message for the next turn's
    /// transcript.
    pub assistant_text: String,
    /// Tool calls the assistant proposed. Empty when the
    /// assistant produced text only.
    pub tool_calls: Vec<crate::session::ToolCall>,
    pub stop_reason: crate::session::StopReason,
}

impl Outcome {
    pub fn error() -> Self {
        Self {
            assistant_text: String::new(),
            tool_calls: Vec::new(),
            stop_reason: crate::session::StopReason::Error,
        }
    }
}

/// Build a backend for `kind` from the live config. Resolves the
/// API key (env / keychain / file fallback) and the effective
/// model (config override or default). Errors out with a clear
/// message when the key is missing for an http-shaped backend;
/// Ollama needs no key.
pub fn build(kind: BackendKind, config: &LlmConfig) -> Result<Arc<dyn Backend>> {
    let model = config
        .models
        .for_backend(kind)
        .map(str::to_owned)
        .unwrap_or_else(|| kind.default_model().to_string());
    match kind {
        BackendKind::Ollama => {
            let base =
                std::env::var("OLLAMA_HOST").unwrap_or_else(|_| ollama::DEFAULT_URL.to_string());
            Ok(Arc::new(ollama::OllamaBackend::new(base, model)))
        }
        BackendKind::Anthropic => {
            let _key = keys::resolve(kind, config)
                .0
                .ok_or_else(|| LlmError::MissingApiKey("anthropic".into()))?;
            Err(LlmError::NotImplemented(
                "anthropic backend ports in a follow-up commit".into(),
            ))
        }
        BackendKind::Gemini => {
            let _key = keys::resolve(kind, config)
                .0
                .ok_or_else(|| LlmError::MissingApiKey("gemini".into()))?;
            Err(LlmError::NotImplemented(
                "gemini backend ports in a follow-up commit".into(),
            ))
        }
    }
}
