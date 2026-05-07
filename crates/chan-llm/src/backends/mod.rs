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
/// translate `messages` to wire format, stream the response,
/// dispatch text deltas + tool calls + the final stop reason
/// into the listener, then return. Errors are dispatched via
/// `on_error` followed by `on_done(StopReason::Error)`; the
/// signature returns nothing because every consumer-facing
/// outcome is on the listener.
#[async_trait]
pub trait Backend: Send + Sync {
    async fn run(&self, messages: Vec<Message>, listener: Arc<dyn SessionListener>);
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
