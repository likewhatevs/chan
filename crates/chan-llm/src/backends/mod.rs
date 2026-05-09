// Backend dispatch.
//
// Backends in scope today:
//   - Anthropic (Claude)        - HTTP, streaming SSE
//   - Gemini (Google)           - HTTP, streaming SSE
//   - Ollama (local server)     - HTTP, streaming JSON
//   - ClaudeCli                 - shell-executor wrapper around the
//                                 `claude` CLI; v1 runs claude as a
//                                 black-box agent (its own tools, its
//                                 own permission decisions). The
//                                 chan-llm tool sandbox does NOT
//                                 mediate writes from this backend;
//                                 see claude_cli.rs for the
//                                 contract gap and the v2 path.
//
// Each provides a `Backend` impl that owns its transport config
// (auth header style, base URL, model defaults, or subprocess args)
// and translates chan-llm's internal `Message` list into the
// backend's wire format, then drives the streaming response,
// dispatching events into the `SessionListener` the caller supplied.
//
// HTTP backends DO NOT touch the filesystem or chan-drive directly.
// The tool sandbox sits between them and disk: the assistant
// proposes a tool call, chan-llm relays it to the host via
// on_tool_call, the host runs `tools::execute` against
// chan-drive::Drive, and the next turn's transcript carries the
// tool result. Backends only translate one HTTP exchange per turn.
//
// The ClaudeCli backend is the deliberate exception: it shells out
// to a full agent, so claude's own tool loop runs against the
// drive root directly. This is logged as a v1 limitation.

pub mod anthropic;
pub mod claude_cli;
pub mod gemini;
pub mod ollama;
mod retry;
pub use retry::{send_with_retry, RetryPolicy};

/// Hard cap on a single turn's accumulated assistant text. The
/// listener (`on_delta`) is fire-and-forget; if it blocks or if the
/// model goes into a runaway emit loop, the per-turn String would
/// grow unbounded. 10 MB is well above any plausible legitimate
/// turn (typical: <100 KB) and well below where a single allocation
/// becomes painful. Backends abort the stream when they cross this
/// threshold; the alternative (silently truncating) corrupts the
/// transcript fed back into the model on the next turn.
pub const ASSISTANT_TEXT_CAP_BYTES: usize = 10 * 1024 * 1024;

use std::path::Path;
use std::sync::atomic::AtomicBool;
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
    ClaudeCli,
}

impl BackendKind {
    pub fn name(self) -> &'static str {
        match self {
            BackendKind::Anthropic => "anthropic",
            BackendKind::Gemini => "gemini",
            BackendKind::Ollama => "ollama",
            BackendKind::ClaudeCli => "claude_cli",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            BackendKind::Anthropic => "claude-opus-4-7",
            BackendKind::Gemini => "gemini-2.5-pro",
            BackendKind::Ollama => "llama3.1",
            // Empty default: claude CLI picks its own configured
            // model when --model is omitted. We only override when
            // the user explicitly sets Models::claude_cli.
            BackendKind::ClaudeCli => "",
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
    /// Drive one HTTP / subprocess exchange. `cancel` is checked at
    /// chunk boundaries; backends should also drop their underlying
    /// stream when it flips so an in-flight request stops promptly
    /// rather than running to completion. The session-level loop
    /// also checks `cancel` between iterations, so a backend that
    /// only checks it once per chunk is acceptable - just less
    /// responsive to the user hitting "stop".
    async fn run(
        &self,
        messages: Vec<Message>,
        tools: Vec<crate::tools::ToolSchema>,
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
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

    pub fn cancelled(assistant_text: String) -> Self {
        Self {
            assistant_text,
            tool_calls: Vec::new(),
            stop_reason: crate::session::StopReason::Cancelled,
        }
    }
}

/// Build a backend for `kind` from the live config. Resolves the
/// API key (env / keychain / file fallback) and the effective
/// model (config override or default). Errors out with a clear
/// message when the key is missing for an http-shaped backend;
/// Ollama needs no key.
///
/// `drive_root` is the absolute path of the chan drive the session
/// is bound to. Most backends ignore it (HTTP transports don't
/// care about the local filesystem). The ClaudeCli backend uses
/// it as the subprocess `cwd` so claude's filesystem tools
/// resolve paths relative to the user's drive, not the host
/// process's cwd.
pub fn build(kind: BackendKind, config: &LlmConfig, drive_root: &Path) -> Result<Arc<dyn Backend>> {
    let model = config
        .models
        .for_backend(kind)
        .map(str::to_owned)
        .unwrap_or_else(|| kind.default_model().to_string());
    // User override > backend default. claude_cli ignores this
    // (claude has its own ceiling), so the resolver returns None
    // for it and the constructor doesn't take the param at all.
    let max_tokens_override = config.max_tokens.for_backend(kind);
    match kind {
        BackendKind::Ollama => {
            // Precedence: OLLAMA_HOST env (per-shell override) wins
            // over config.urls.ollama (Settings UI persistence) wins
            // over the hardcoded default. Mirrors the keys story.
            let base = std::env::var("OLLAMA_HOST")
                .ok()
                .or_else(|| config.urls.ollama.clone())
                .unwrap_or_else(|| ollama::DEFAULT_URL.to_string());
            let _ = drive_root;
            Ok(Arc::new(ollama::OllamaBackend::new(
                base,
                model,
                max_tokens_override,
            )))
        }
        BackendKind::Anthropic => {
            let key = keys::resolve(kind, config)
                .0
                .ok_or_else(|| LlmError::MissingApiKey("anthropic".into()))?;
            let _ = drive_root;
            let max_tokens = max_tokens_override.unwrap_or(anthropic::DEFAULT_MAX_TOKENS);
            Ok(Arc::new(anthropic::AnthropicBackend::new(
                key, model, max_tokens,
            )))
        }
        BackendKind::Gemini => {
            let key = keys::resolve(kind, config)
                .0
                .ok_or_else(|| LlmError::MissingApiKey("gemini".into()))?;
            let _ = drive_root;
            let max_tokens = max_tokens_override.unwrap_or(gemini::DEFAULT_MAX_OUTPUT_TOKENS);
            Ok(Arc::new(gemini::GeminiBackend::new(key, model, max_tokens)))
        }
        BackendKind::ClaudeCli => {
            let cli = config.claude_cli.clone();
            // Empty Models::claude_cli means "let claude pick its
            // configured default"; we only forward --model when the
            // user explicitly set one.
            let model = if model.is_empty() { None } else { Some(model) };
            // v2 MCP-mediated mode kicks in when the host supplied
            // `mcp_command`. The auto-apply flag rides along so the
            // subprocess we spawn (chan-llm-mcp / `chan __mcp`)
            // honors the user's confirmation preference.
            let mcp = cli.mcp_command.map(|command| claude_cli::McpWiring {
                command,
                auto_apply_writes: config.auto_apply_writes,
            });
            Ok(Arc::new(claude_cli::ClaudeCliBackend::new(
                cli.cmd.unwrap_or_else(claude_cli::default_cmd),
                cli.extra_args,
                model,
                drive_root.to_path_buf(),
                mcp,
            )))
        }
    }
}
