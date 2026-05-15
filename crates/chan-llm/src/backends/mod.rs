// Backend dispatch.
//
// Backends in scope today:
//   - ClaudeCli                 - shell-executor wrapper around the
//                                 `claude` CLI. v1 runs claude as a
//                                 black-box agent against the drive
//                                 root; v2 routes claude's writes
//                                 through chan-llm's MCP server via
//                                 a temp `--mcp-config`. See
//                                 claude_cli.rs.
//   - GeminiCli                 - shell-executor wrapper around the
//                                 `gemini` CLI. Same v1/v2 split as
//                                 ClaudeCli; v2 redirects
//                                 GEMINI_CLI_HOME at a tmpdir we
//                                 own (gemini-cli has no per-
//                                 invocation --mcp-config flag) and
//                                 deny-policies the native edit/
//                                 shell tools. See gemini_cli.rs.
//   - CodexCli                  - shell-executor wrapper around
//                                 `codex exec --json`. v2 injects
//                                 chan's MCP server via per-run
//                                 config overrides so the user's
//                                 ~/.codex auth/config stays intact.
//
// Each provides a `Backend` impl that owns its subprocess args,
// translates chan-llm's internal `Message` list into the CLI's
// prompt format, then drives the CLI's streaming response,
// dispatching events into the `SessionListener` the caller supplied.
// In v1 mode the CLI's own tool loop hits the drive root directly
// and bypasses chan-llm's gates; in v2 mode writes flow through a
// chan-llm MCP subprocess, which re-applies the gates. The
// session-level loop returns empty `tool_calls` for both modes since
// the CLI has already executed them.

pub mod claude_cli;
pub mod codex_cli;
pub mod gemini_cli;
#[cfg(any(test, feature = "bench"))]
pub mod mock;
mod ndjson;
mod subprocess_env;
pub(crate) use ndjson::{read_line_capped, NDJSON_LINE_CAP_BYTES, PARSE_ERROR_EMIT_LIMIT};
pub(crate) use subprocess_env::{
    sanitize_env_for_claude_cli, sanitize_env_for_codex_cli, sanitize_env_for_gemini_cli,
    spawn_stderr_drainer, StderrDrainer,
};

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

use crate::cli::{command_path_env, resolve_backend_command};
use crate::config::LlmConfig;
use crate::error::{LlmError, Result};
use crate::session::{Message, SessionListener};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    ClaudeCli,
    GeminiCli,
    CodexCli,
}

impl BackendKind {
    pub fn name(self) -> &'static str {
        match self {
            BackendKind::ClaudeCli => "claude_cli",
            BackendKind::GeminiCli => "gemini_cli",
            BackendKind::CodexCli => "codex_cli",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            // Empty default: the CLI picks its own configured model
            // when --model is omitted. We only override when the
            // user explicitly sets Models::claude_cli / gemini_cli / codex_cli.
            BackendKind::ClaudeCli | BackendKind::GeminiCli | BackendKind::CodexCli => "",
        }
    }
}

/// What every backend implements. `run` drives one subprocess
/// exchange: translate `messages` to the CLI's input shape, stream
/// the response, emit text deltas via `on_delta` (and `on_error` on
/// failure) into the listener, then return an `Outcome`.
///
/// Backends do NOT emit `on_tool_call`, `on_tool_result`, or
/// `on_done` themselves. Those are the orchestration loop's
/// concern (in `session.rs::send`); a backend is just one CLI run
/// translated to chan-llm's event vocabulary.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Drive one subprocess exchange. `cancel` is checked at
    /// chunk boundaries; backends should also drop their underlying
    /// stream when it flips so an in-flight request stops promptly
    /// rather than running to completion. The session-level loop
    /// also checks `cancel` between iterations, so a backend that
    /// only checks it once per chunk is acceptable - just less
    /// responsive to the user hitting "stop".
    ///
    /// `messages` and `tools` are read-only slices owned by the
    /// orchestrator; backends must not store them past the call
    /// (they're tied to the future's lifetime via async_trait).
    /// Taking slices instead of `Vec<_>` saves a per-iteration clone
    /// of the full transcript across the orchestrator's tool-call
    /// loop.
    async fn run(
        &self,
        messages: &[Message],
        tools: &[crate::tools::ToolSchema],
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome;
}

/// What the backend collected during one subprocess exchange. The
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
/// effective model (config override or CLI default).
///
/// `drive_root` is the absolute path of the chan drive the session
/// is bound to. CLI backends use it as the subprocess `cwd` so
/// filesystem tools resolve paths relative to the user's drive, not
/// the host process's cwd.
pub fn build(kind: BackendKind, config: &LlmConfig, drive_root: &Path) -> Result<Arc<dyn Backend>> {
    let model = config
        .models
        .for_backend(kind)
        .map(str::to_owned)
        .unwrap_or_else(|| kind.default_model().to_string());
    let detected = resolve_backend_command(kind, config);
    let command = if detected.present() {
        detected.command
    } else {
        let command = detected
            .command
            .first()
            .cloned()
            .unwrap_or_else(|| "<empty>".to_string());
        return Err(LlmError::CliNotFound {
            backend: kind.name().into(),
            command,
            reason: match detected.status {
                crate::cli::CliStatus::NotFound => "not found in CLI search path".to_string(),
                crate::cli::CliStatus::Rejected { message, .. } => message,
                crate::cli::CliStatus::Present => "not found in CLI search path".to_string(),
            },
        });
    };
    let path_env = command_path_env(config);
    match kind {
        BackendKind::ClaudeCli => {
            let cli = config.claude_cli.clone();
            // Empty Models::claude_cli means "let claude pick its
            // configured default"; we only forward --model when the
            // user explicitly set one.
            let model = if model.is_empty() { None } else { Some(model) };
            // v2 MCP-mediated mode kicks in when the host supplied
            // `mcp_command`. The auto-apply gate is owned by the MCP
            // server side (in chan-server, the bridge reads it per
            // connection), so it isn't threaded through the wiring.
            let mcp = cli
                .mcp_command
                .map(|command| claude_cli::McpWiring { command });
            let inactivity =
                ndjson::resolve_inactivity_timeout(config.stream_inactivity_timeout_secs);
            Ok(Arc::new(claude_cli::ClaudeCliBackend::new(
                command,
                cli.extra_args,
                model,
                drive_root.to_path_buf(),
                mcp,
                inactivity,
                config.hardened_subprocess_env,
                path_env,
            )))
        }
        BackendKind::GeminiCli => {
            let cli = config.gemini_cli.clone();
            let model = if model.is_empty() { None } else { Some(model) };
            // gemini-cli has no per-invocation `--mcp-config <file>`
            // flag, so v2 mode rewrites GEMINI_CLI_HOME to a tmpdir
            // we own and symlinks/copies the user's real auth state
            // into it. chan-llm no longer stores provider API keys;
            // gemini-cli owns auth via `gemini login` or inherited
            // shell env.
            let mcp = cli
                .mcp_command
                .map(|command| gemini_cli::McpWiring { command });
            let inactivity =
                ndjson::resolve_inactivity_timeout(config.stream_inactivity_timeout_secs);
            Ok(Arc::new(gemini_cli::GeminiCliBackend::new(
                command,
                cli.extra_args,
                model,
                drive_root.to_path_buf(),
                mcp,
                inactivity,
                config.hardened_subprocess_env,
                path_env,
            )))
        }
        BackendKind::CodexCli => {
            let cli = config.codex_cli.clone();
            let model = if model.is_empty() { None } else { Some(model) };
            let mcp = cli
                .mcp_command
                .map(|command| codex_cli::McpWiring { command });
            let inactivity =
                ndjson::resolve_inactivity_timeout(config.stream_inactivity_timeout_secs);
            Ok(Arc::new(codex_cli::CodexCliBackend::new(
                command,
                cli.extra_args,
                model,
                drive_root.to_path_buf(),
                mcp,
                inactivity,
                config.hardened_subprocess_env,
                path_env,
            )))
        }
    }
}
