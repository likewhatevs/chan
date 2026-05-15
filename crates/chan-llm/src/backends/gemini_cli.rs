//! GeminiCli backend: shell-executor wrapper around the `gemini` CLI.
//!
//! End-to-end streaming flow, failure modes, and the in-flight
//! hardening plan are documented in `crates/chan-llm/design.md`
//! sections 6.1 through 6.3 and section 13. Note that gemini-cli's
//! NDJSON output has no upstream flag for token-level partials, so
//! `on_delta` granularity here is per-message.
//!
//! Two modes, selected at construction time, mirroring `claude_cli`:
//!
//! ### v1: black-box (legacy, `mcp = None`)
//!
//! Gemini runs as a full agent. We hand it a prompt and read its
//! `--output-format stream-json` NDJSON; we do NOT mediate its tool
//! calls. Concretely:
//!
//!   - Gemini edits files directly under `cwd` (the drive root)
//!     using its own `read_file` / `replace` / `run_shell_command`
//!     / etc. native tools.
//!   - chan-llm's tool sandbox (path scope, editable-text gate,
//!     auto_apply_writes confirmation) is NOT applied to those
//!     edits.
//!   - We launch gemini with `--approval-mode yolo` because there
//!     is no human in front of gemini's stdin to answer its native
//!     confirmation prompts.
//!
//! v1 stays available for hosts that don't ship a chan-llm-mcp
//! capable binary; tests cover this path.
//!
//! ### v2: MCP-mediated (`mcp = Some(McpWiring { .. })`)
//!
//! Closes the gap by routing gemini's writes through chan-llm's MCP
//! server. Because gemini-cli has no per-invocation `--mcp-config
//! <file>` flag (unlike claude), v2 mode rewrites
//! `GEMINI_CLI_HOME` to a tmpdir we own and lays out a synthetic
//! `~/.gemini/` inside it:
//!
//!   - `<home>/.gemini/settings.json` advertises the chan-llm MCP
//!     server under `mcpServers.chan` with the host-supplied argv.
//!   - `<home>/.gemini/policies/chan.toml` deny-rules gemini's
//!     native edit / shell tools (`replace`, `write_file`,
//!     `edit`, `run_shell_command`) so writes have to flow through
//!     our MCP server, where chan-drive's path sandbox + editable-
//!     text gate apply.
//!   - `--allowed-mcp-server-names chan` keeps any user-installed
//!     servers in the real `~/.gemini` out of the picture.
//!
//! Redirecting `GEMINI_CLI_HOME` blocks gemini from reading the
//! user's real `~/.gemini` auth directly, so the synthetic home
//! symlinks/copies the real credential files. Shell-provided
//! `GEMINI_API_KEY` / `GOOGLE_API_KEY` also survive env
//! sanitization.
//!
//! The auto-apply gate is owned by the MCP server side; when it's
//! off, `write_file` returns a deferred error to gemini (same
//! contract as the claude_cli v2 path).
//!
//! Wire format:
//!
//!   - Input: chan-llm's transcript is concatenated into a single
//!     prompt string (system / user / assistant labels) and passed
//!     to gemini via `-p`. Stateless per-call; multi-turn fidelity
//!     is lossy (assistant turns become labelled text). Same v1
//!     trade-off as claude_cli; resuming a gemini session via
//!     `-r <session-id>` is the natural follow-up.
//!
//!   - Output: gemini emits NDJSON on stdout. Events of interest:
//!     - `{"type":"init",...}` (ignored)
//!     - `{"type":"message","role":"assistant","content":"...",
//!         "delta":true|false?}` (emit on_delta on assistant)
//!     - `{"type":"tool_use","tool_name":"...","tool_id":"...",
//!         "parameters":{...}}` (forward via on_tool_call)
//!     - `{"type":"tool_result","tool_id":"...","status":"...",
//!         "output":"..."}` (forward via on_tool_result)
//!     - `{"type":"error","severity":"...","message":"..."}`
//!     - `{"type":"result","status":"success"|"error",...}`
//!
//!     Tool calls/results are observational in v1; the orchestration
//!     loop treats `Outcome.tool_calls` as empty so it exits after
//!     one backend turn (matches claude_cli semantics).

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value as Json;
use tempfile::TempDir;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::time::timeout;

use crate::session::{
    AgentActivity, AgentStatus, Delta, Message, Role, SessionListener, StopReason, ToolCall,
    ToolResult,
};
use crate::tools::ToolSchema;

use super::{
    read_line_capped, sanitize_env_for_gemini_cli, spawn_stderr_drainer, Backend, Outcome,
    StderrDrainer, NDJSON_LINE_CAP_BYTES, PARSE_ERROR_EMIT_LIMIT,
};

/// Default command to launch gemini. Plain `gemini` so PATH wins;
/// users override via `LlmConfig.gemini_cli.cmd` when gemini lives
/// somewhere non-standard or when wrapping the binary.
pub fn default_cmd() -> Vec<String> {
    vec!["gemini".to_string()]
}

/// Host-supplied wiring that switches the backend into v2
/// MCP-mediated mode. `command` is the full argv used to spawn the
/// MCP server (typically the host binary plus a hidden subcommand
/// and a socket path or drive root). The auto-apply gate is owned
/// by the MCP server itself (in chan-server it lives on the bridge
/// and is read per-connection), so it's not threaded through here.
#[derive(Debug, Clone)]
pub struct McpWiring {
    pub command: Vec<String>,
}

/// MCP server name as it appears under `mcpServers` in the generated
/// settings.json. Used by `--allowed-mcp-server-names` and surfaced
/// to gemini as the `mcp_chan_<tool>` tool prefix.
const MCP_SERVER_KEY: &str = "chan";

#[derive(Debug)]
pub struct GeminiCliBackend {
    cmd: Vec<String>,
    extra_args: Vec<String>,
    model: Option<String>,
    cwd: PathBuf,
    mcp: Option<McpWiring>,
    /// Max time between consecutive stdout lines before the
    /// subprocess is treated as wedged. Resolved upstream by
    /// `backends::build` from `LlmConfig.stream_inactivity_timeout_secs`.
    inactivity_timeout: Duration,
    /// When true, env sanitization uses an explicit-name allowlist
    /// instead of the loose `GOOGLE_` / `GEMINI_` prefix match.
    /// Resolved from `LlmConfig.hardened_subprocess_env`.
    hardened_env: bool,
    path_env: Option<OsString>,
}

impl GeminiCliBackend {
    // Each parameter maps to a distinct LlmConfig field; collapsing
    // them into a struct would just shuffle the names around without
    // making the call sites clearer.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cmd: Vec<String>,
        extra_args: Vec<String>,
        model: Option<String>,
        cwd: PathBuf,
        mcp: Option<McpWiring>,
        inactivity_timeout: Duration,
        hardened_env: bool,
        path_env: Option<OsString>,
    ) -> Self {
        Self {
            cmd,
            extra_args,
            model,
            cwd,
            mcp,
            inactivity_timeout,
            hardened_env,
            path_env,
        }
    }
}

#[async_trait]
impl Backend for GeminiCliBackend {
    async fn run(
        &self,
        messages: &[Message],
        _tools: &[ToolSchema],
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let prompt = render_prompt(messages);

        let Some((bin, leading)) = self.cmd.split_first() else {
            listener.on_error("gemini_cli: empty cmd".into());
            return Outcome::error();
        };

        let mut command = Command::new(bin);
        // Drop the parent env so unrelated secrets (OPENAI_API_KEY,
        // GH_TOKEN, AWS_*) don't leak into a spawned child's
        // /proc/<pid>/environ. The wrapper picks the loose
        // (prefix-based) or strict (explicit-name) variant based on
        // `hardened_env`. Strict mode drops
        // `GOOGLE_APPLICATION_CREDENTIALS` so a tainted parent env
        // can't redirect gemini-cli at an attacker-controlled
        // service-account JSON. The explicit `.env(...)` calls below
        // for `GEMINI_CLI_HOME` / `GEMINI_API_KEY` run after this and
        // override anything the parent env forwarded.
        sanitize_env_for_gemini_cli(&mut command, self.hardened_env);
        if let Some(path) = self.path_env.as_ref() {
            command.env("PATH", path);
        }
        // Kill the spawned gemini on Drop. Normal exit paths call
        // `child.kill().await` explicitly; this guards against a
        // panic anywhere below leaving the subprocess running.
        command.kill_on_drop(true);
        command
            .args(leading)
            .arg("--prompt")
            .arg(&prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--approval-mode")
            .arg("yolo")
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // v2 wiring: redirect GEMINI_CLI_HOME at a tmpdir we own,
        // populate <home>/.gemini/{settings.json, policies/chan.toml},
        // restrict MCP servers to ours. Auth comes from the user's
        // real ~/.gemini files symlinked/copied into the temp home,
        // or from shell-provided GEMINI_/GOOGLE_ env.
        let mcp_home = match self.mcp.as_ref() {
            None => None,
            Some(wiring) => {
                let home = match write_gemini_home(wiring) {
                    Ok(h) => h,
                    Err(e) => {
                        listener.on_error(format!("gemini_cli mcp-home: {e}"));
                        return Outcome::error();
                    }
                };
                command
                    .env("GEMINI_CLI_HOME", home.path())
                    .arg("--allowed-mcp-server-names")
                    .arg(MCP_SERVER_KEY);
                Some(home)
            }
        };

        if let Some(model) = self.model.as_deref() {
            command.arg("--model").arg(model);
        }
        for arg in &self.extra_args {
            command.arg(arg);
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                listener.on_error(format!("gemini_cli spawn: {e}"));
                return Outcome::error();
            }
        };
        listener.on_status(AgentStatus::Spawned {
            backend: "gemini_cli".into(),
            pid: child.id(),
        });

        // Drainer captures stderr concurrently with the stdout loop
        // so the child can't wedge on a full stderr pipe while still
        // streaming events. On non-zero exit we read up to
        // STDERR_CAP_BYTES from the buffer for the on_error snippet.
        let mut stderr_drainer: Option<StderrDrainer> = spawn_stderr_drainer(child.stderr.take());
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_error("gemini_cli: stdout not piped".into());
                let _ = child.kill().await;
                return Outcome::error();
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut line_buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut stop = StopReason::EndOfTurn;
        let mut saw_result = false;
        let mut parse_errors_emitted = 0usize;
        let mut parse_errors_silenced = 0usize;
        let heartbeat = Heartbeat::spawn(
            listener.clone(),
            "gemini_cli",
            heartbeat_interval(self.inactivity_timeout),
        );
        // Per-message streamed-text accumulator. gemini-cli marks
        // streaming chunks with delta:true and the canonical assistant
        // message with delta:false; the canonical event carries the
        // FULL accumulated text for its message. Comparing canonical
        // against the global `assistant_text` works for the single-
        // message turns gemini-cli emits today, but breaks the moment
        // it ever emits two messages in one turn (the second canonical
        // wouldn't prefix-match the global, the drift branch would
        // fire, and half the text would vanish). Track this-message
        // streamed text and reset on canonical so multi-message turns
        // stay correct.
        let mut current_message_streamed = String::new();

        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill().await;
                let _ = child.wait().await;
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_status(AgentStatus::Cancelled {
                    backend: "gemini_cli".into(),
                });
                return Outcome::cancelled(assistant_text);
            }
            let read = timeout(
                self.inactivity_timeout,
                read_line_capped(&mut reader, &mut line_buf, NDJSON_LINE_CAP_BYTES),
            )
            .await;
            let got_line = match read {
                Ok(Ok(true)) => {
                    heartbeat.mark_stdout();
                    true
                }
                Ok(Ok(false)) => break,
                Ok(Err(e)) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let stderr_snippet = match stderr_drainer.take() {
                        Some(d) => truncate(&d.finish().await, 800),
                        None => String::new(),
                    };
                    if stderr_snippet.is_empty() {
                        listener.on_error(format!("gemini_cli stdout: {e}"));
                    } else {
                        listener
                            .on_error(format!("gemini_cli stdout: {e}; stderr: {stderr_snippet}"));
                    }
                    listener.on_status(AgentStatus::Unhealthy {
                        backend: "gemini_cli".into(),
                        reason: "stdout".into(),
                        detail: Some(e.to_string()),
                    });
                    return Outcome::error();
                }
                Err(_elapsed) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let stderr_snippet = match stderr_drainer.take() {
                        Some(d) => truncate(&d.finish().await, 800),
                        None => String::new(),
                    };
                    let base = format!(
                        "gemini_cli: no output for {}s; subprocess wedged",
                        self.inactivity_timeout.as_secs(),
                    );
                    if stderr_snippet.is_empty() {
                        listener.on_error(base);
                    } else {
                        listener.on_error(format!("{base}; stderr: {stderr_snippet}"));
                    }
                    listener.on_status(AgentStatus::Unhealthy {
                        backend: "gemini_cli".into(),
                        reason: format!("no_output_for_{}s", self.inactivity_timeout.as_secs()),
                        detail: if stderr_snippet.is_empty() {
                            None
                        } else {
                            Some(stderr_snippet)
                        },
                    });
                    return Outcome::error();
                }
            };
            if !got_line {
                break;
            }
            let line = match std::str::from_utf8(&line_buf) {
                Ok(s) => s.trim_end_matches('\n'),
                Err(_) => {
                    if parse_errors_emitted < PARSE_ERROR_EMIT_LIMIT {
                        listener.on_error("gemini_cli stdout: non-utf8 line".into());
                        parse_errors_emitted += 1;
                    } else {
                        parse_errors_silenced += 1;
                    }
                    continue;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            // gemini-cli sometimes prepends a status banner without a
            // trailing newline before the first JSON event (observed:
            // "MCP issues detected. Run /mcp list for status." glued
            // straight onto the init event). Skip any prefix before
            // the first '{' so the parser sees clean JSON.
            let payload = match line.find('{') {
                Some(i) => &line[i..],
                None => {
                    // Line has no JSON at all (pure banner). Drop it
                    // silently rather than failing the turn; the
                    // banner is informational.
                    continue;
                }
            };
            let event: StreamEvent = match serde_json::from_str(payload) {
                Ok(e) => e,
                Err(e) => {
                    if parse_errors_emitted < PARSE_ERROR_EMIT_LIMIT {
                        listener.on_error(format!(
                            "gemini_cli parse: {e}; raw: {}",
                            truncate(line, 400)
                        ));
                        parse_errors_emitted += 1;
                    } else {
                        parse_errors_silenced += 1;
                    }
                    continue;
                }
            };
            match event {
                StreamEvent::Message {
                    role,
                    content,
                    delta,
                } => {
                    // The user echo arrives as role=user; ignore it,
                    // the chan-llm transcript already has it.
                    if role.as_deref() != Some("assistant") || content.is_empty() {
                        continue;
                    }
                    // gemini-cli's NDJSON tags streaming chunks with
                    // `delta:true` and the final canonical assistant
                    // message with `delta:false`. The canonical event
                    // carries the FULL accumulated text, so emitting
                    // it via on_delta on top of the streamed chunks
                    // doubles every assistant turn. Reconcile: when
                    // delta is explicitly false, treat content as
                    // canonical and emit only the suffix beyond what
                    // partials already streamed (drop on drift, same
                    // defensive choice as claude_cli). Missing-delta
                    // is treated as a streaming chunk so the legacy
                    // fixture shape (no flag at all) keeps emitting.
                    let is_canonical = matches!(delta, Some(false));
                    let emit: Option<&str> = if is_canonical {
                        // Compare against THIS message's streamed
                        // prefix, not the global `assistant_text`. The
                        // distinction matters when gemini-cli ever
                        // emits more than one assistant message in a
                        // turn (today it doesn't, but the wire format
                        // permits it). After we emit the canonical
                        // suffix, reset the per-message accumulator so
                        // the next message starts fresh.
                        let suffix = if let Some(s) =
                            content.strip_prefix(current_message_streamed.as_str())
                        {
                            if s.is_empty() {
                                None
                            } else {
                                Some(s)
                            }
                        } else {
                            tracing::warn!(
                                streamed_bytes = current_message_streamed.len(),
                                canonical_bytes = content.len(),
                                "gemini_cli: streamed/canonical drift; \
                                 keeping streamed view, dropping canonical to avoid \
                                 doubled assistant_text",
                            );
                            None
                        };
                        current_message_streamed.clear();
                        suffix
                    } else {
                        current_message_streamed.push_str(content.as_str());
                        Some(content.as_str())
                    };
                    if let Some(s) = emit {
                        listener.on_delta(Delta {
                            text: s.to_string(),
                        });
                        assistant_text.push_str(s);
                        if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                            listener.on_error(format!(
                                "gemini_cli stream: assistant text exceeded {} bytes; aborting",
                                super::ASSISTANT_TEXT_CAP_BYTES,
                            ));
                            let _ = child.kill().await;
                            return Outcome::error();
                        }
                    }
                }
                StreamEvent::ToolUse {
                    tool_name,
                    tool_id,
                    parameters,
                } => {
                    listener.on_activity(AgentActivity::ToolStarted {
                        backend: "gemini_cli".into(),
                        id: tool_id.clone(),
                        name: tool_name.clone(),
                        parent_id: None,
                    });
                    listener.on_tool_call(ToolCall {
                        id: tool_id,
                        name: tool_name,
                        args: parameters,
                    });
                }
                StreamEvent::ToolResult {
                    tool_id,
                    output,
                    error,
                    status,
                } => {
                    let is_error = status.as_deref() == Some("error") || error.is_some();
                    let body = if is_error {
                        // Gemini's error payload is structured
                        // ({type, message}); the chan-llm listener
                        // expects a JSON Value, so wrap it.
                        error.map(|e| serde_json::json!(e)).unwrap_or(Json::Null)
                    } else {
                        match output {
                            Some(s) => Json::String(s),
                            None => Json::Null,
                        }
                    };
                    listener.on_activity(AgentActivity::ToolFinished {
                        backend: "gemini_cli".into(),
                        id: tool_id.clone(),
                        name: None,
                        output: body.clone(),
                        is_error,
                        parent_id: None,
                    });
                    listener.on_tool_result(ToolResult {
                        id: tool_id,
                        output: body,
                    });
                }
                StreamEvent::Error { severity, message } => {
                    // Surface non-fatal warnings via on_error too;
                    // a real fatal arrives as `result: error`.
                    listener.on_activity(AgentActivity::AgentNote {
                        backend: "gemini_cli".into(),
                        text: match severity {
                            Some(severity) => format!("{severity}: {message}"),
                            None => message.clone(),
                        },
                        parent_id: None,
                    });
                    listener.on_error(format!("gemini_cli: {message}"));
                }
                StreamEvent::Result { status, error, .. } => {
                    saw_result = true;
                    if status.as_deref() == Some("error") {
                        let msg = error
                            .map(|e| e.message)
                            .unwrap_or_else(|| "gemini exited with error".to_string());
                        listener.on_error(format!("gemini_cli result: {msg}"));
                        listener.on_status(AgentStatus::Unhealthy {
                            backend: "gemini_cli".into(),
                            reason: "result_error".into(),
                            detail: Some(msg),
                        });
                        stop = StopReason::Error;
                    }
                }
                StreamEvent::Init { session_id, model } => {
                    listener.on_status(AgentStatus::Ready {
                        backend: "gemini_cli".into(),
                        session_id: session_id.clone(),
                        model,
                        version: None,
                    });
                    listener.on_activity(AgentActivity::SessionStarted {
                        backend: "gemini_cli".into(),
                        session_id,
                    });
                }
                StreamEvent::Other => {}
            }
        }

        let status = match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_error(format!("gemini_cli wait: {e}"));
                listener.on_status(AgentStatus::Unhealthy {
                    backend: "gemini_cli".into(),
                    reason: "wait".into(),
                    detail: Some(e.to_string()),
                });
                return Outcome::error();
            }
        };
        if !status.success() {
            let stderr_text = match stderr_drainer.take() {
                Some(d) => d.finish().await,
                None => String::new(),
            };
            let snippet = truncate(&stderr_text, 800);
            listener.on_error(format!("gemini_cli exit {status}: {snippet}"));
            listener.on_status(AgentStatus::Unhealthy {
                backend: "gemini_cli".into(),
                reason: format!("exit:{status}"),
                detail: if snippet.is_empty() {
                    None
                } else {
                    Some(snippet)
                },
            });
            return Outcome::error();
        }
        drop(stderr_drainer);
        if parse_errors_silenced > 0 {
            listener.on_error(format!(
                "gemini_cli parse: {parse_errors_silenced} additional parse errors suppressed",
            ));
        }
        if !saw_result && stop != StopReason::Error {
            // gemini exited cleanly but never emitted a `result`
            // event. The transcript may be incomplete; treat as
            // an error so the host renders an actionable state.
            listener.on_error("gemini_cli: stream ended without a result event".into());
            listener.on_status(AgentStatus::Unhealthy {
                backend: "gemini_cli".into(),
                reason: "no_result".into(),
                detail: None,
            });
            stop = StopReason::Error;
        }
        listener.on_status(AgentStatus::Exited {
            backend: "gemini_cli".into(),
            code: status.code(),
            success: stop != StopReason::Error,
        });

        // Drop the home tmpdir now that gemini has exited.
        drop(mcp_home);

        Outcome {
            assistant_text,
            // Gemini runs its own tool loop; we don't ask the
            // orchestrator to dispatch the surfaced calls.
            tool_calls: Vec::new(),
            stop_reason: stop,
        }
    }
}

/// Build the synthetic `~/.gemini` layout for v2 mode. Returns the
/// `TempDir` that owns it (must outlive the gemini subprocess), and
/// inside it lays out:
///
/// ```text
/// <home>/.gemini/settings.json     # merged user + chan mcpServer
/// <home>/.gemini/policies/chan.toml  # deny native edit/shell tools
/// <home>/.gemini/oauth_creds.json    # symlink -> user's ~/.gemini
/// <home>/.gemini/google_accounts.json
/// <home>/.gemini/state.json
/// <home>/.gemini/installation_id
/// ```
///
/// settings.json merges the user's real `~/.gemini/settings.json`
/// (so `security.auth.selectedType`, telemetry, sandbox flags, ...
/// carry over) and force-overwrites `mcpServers.chan` with the
/// chan-llm wiring. Auth credential files are symlinked (Unix) or
/// copied (other) into the synthetic home so non-interactive runs
/// authenticate against the user's existing OAuth state. Without
/// this, redirecting `GEMINI_CLI_HOME` to a fresh tempdir leaves
/// gemini-cli with no credentials and it fails with
/// `FATAL_AUTHENTICATION_ERROR`.
fn write_gemini_home(wiring: &McpWiring) -> std::io::Result<TempDir> {
    write_gemini_home_with_user(wiring, dirs::home_dir().map(|h| h.join(".gemini")))
}

fn write_gemini_home_with_user(
    wiring: &McpWiring,
    user_home: Option<std::path::PathBuf>,
) -> std::io::Result<TempDir> {
    use std::io::Write;

    let (bin, base_args) = wiring.command.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty mcp_command")
    })?;
    let args: Vec<String> = base_args.to_vec();

    let home = TempDir::new()?;
    let dot_gemini = home.path().join(".gemini");
    let policies_dir = dot_gemini.join("policies");
    std::fs::create_dir_all(&policies_dir)?;
    // tempfile::TempDir creates `home` with mode 0700 on Unix via
    // mkdtemp(3), so the parent dir already blocks other-user
    // traversal. We tighten the inner subdirs and files anyway for
    // defense in depth: a `cp -r` / `tar` of this directory tree
    // would otherwise lose the perimeter on the destination.
    set_user_only(&dot_gemini)?;
    set_user_only(&policies_dir)?;

    // Start from the user's real settings.json (when present) so
    // auth selection and other preferences carry into the sandbox,
    // then force `mcpServers.chan` to chan-llm's wiring.
    let mut settings: serde_json::Value = user_home
        .as_ref()
        .and_then(|h| std::fs::read_to_string(h.join("settings.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if !settings.is_object() {
        settings = serde_json::json!({});
    }
    if let serde_json::Value::Object(map) = &mut settings {
        let mcp = map
            .entry("mcpServers".to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if !mcp.is_object() {
            *mcp = serde_json::Value::Object(serde_json::Map::new());
        }
        if let serde_json::Value::Object(mcp_map) = mcp {
            mcp_map.insert(
                MCP_SERVER_KEY.to_string(),
                serde_json::json!({
                    "command": bin,
                    "args": args,
                    "trust": true,
                }),
            );
        }
    }
    let settings_path = dot_gemini.join("settings.json");
    let mut sf = std::fs::File::create(&settings_path)?;
    sf.write_all(serde_json::to_string_pretty(&settings)?.as_bytes())?;
    sf.flush()?;
    set_user_only_file(&settings_path)?;

    // Bridge auth state. Best-effort: a missing or unreadable user
    // file just means gemini-cli will surface its own auth error.
    if let Some(real) = user_home.as_ref() {
        for name in [
            "oauth_creds.json",
            "google_accounts.json",
            "state.json",
            "installation_id",
        ] {
            let src = real.join(name);
            if !src.exists() {
                continue;
            }
            let dst = dot_gemini.join(name);
            let _ = link_or_copy(&src, &dst);
        }
    }

    // Deny gemini's native write/edit/shell tools so any mutation
    // has to flow through the chan MCP server (whose dispatch runs
    // chan-drive's gates). The names cover gemini-cli's built-in
    // tool set as of writing; new natives that show up later just
    // mean we update this list.
    let policy_body = "\
# chan-llm v2 lockdown: writes flow through the MCP server only.
# `deny` here removes the tool from gemini's tool list entirely.

[[rule]]
toolName = \"replace\"
decision = \"deny\"
priority = 900

[[rule]]
toolName = \"write_file\"
decision = \"deny\"
priority = 900

[[rule]]
toolName = \"edit\"
decision = \"deny\"
priority = 900

[[rule]]
toolName = \"run_shell_command\"
decision = \"deny\"
priority = 900
";
    let policy_path = policies_dir.join("chan.toml");
    let mut pf = std::fs::File::create(&policy_path)?;
    pf.write_all(policy_body.as_bytes())?;
    pf.flush()?;
    set_user_only_file(&policy_path)?;

    Ok(home)
}

/// Set mode 0700 on `path` (Unix only). No-op on other platforms
/// where the umask / inheritance story is different. Errors are
/// returned so the caller can decide whether to fail the launch;
/// today we propagate, since a chmod failure on a directory we just
/// created suggests something structural is wrong.
fn set_user_only(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Set mode 0600 on `path` (Unix only). Same shape as `set_user_only`
/// for files.
fn set_user_only_file(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Bring a credential file from the user's real `~/.gemini` into
/// the synthetic home. Symlinking on Unix lets OAuth token refresh
/// flow back to the user's real file; on other platforms we fall
/// back to a one-shot copy (refreshed tokens won't persist across
/// restarts there, but the immediate auth still works).
fn link_or_copy(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(src, dst).map(|_| ())
    }
}

/// Render the chan-llm transcript into a single labelled prompt
/// string. gemini-cli's `-p` accepts a single user-shaped query,
/// so prior assistant turns become labelled prose. Same v1 lossy
/// trade-off as `claude_cli::split_system_and_prompt`.
///
/// Always leads with the chan CLI session directive; gemini-cli
/// doesn't expose `--system-prompt` / `--append-system-prompt` so
/// we can't push a true system message, but seeding the prompt
/// body with a `[system]` block at the top is the next-best
/// override for the CLI's default "ask before writing" stance.
fn render_prompt(messages: &[Message]) -> String {
    let mut body = String::new();
    body.push_str("[system]\n");
    body.push_str(crate::prompts::CLI_SESSION_DIRECTIVE);
    for m in messages {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        match m.role {
            Role::System => {
                body.push_str("[system]\n");
                body.push_str(&m.content);
            }
            Role::User => {
                body.push_str("[user]\n");
                body.push_str(&m.content);
            }
            Role::Assistant => {
                body.push_str("[assistant]\n");
                body.push_str(&m.content);
            }
            Role::Tool => {
                body.push_str("[tool_result ");
                body.push_str(m.tool_call_id.as_deref().unwrap_or(""));
                body.push_str("]\n");
                body.push_str(&m.content);
            }
        }
    }
    body
}

fn heartbeat_interval(inactivity_timeout: Duration) -> Duration {
    let five = Duration::from_secs(5);
    let half = inactivity_timeout / 2;
    if half.is_zero() {
        Duration::from_secs(1)
    } else {
        five.min(half)
    }
}

struct Heartbeat {
    active: Arc<AtomicBool>,
    last_stdout_at: Arc<Mutex<Instant>>,
}

impl Heartbeat {
    fn spawn(
        listener: Arc<dyn SessionListener>,
        backend: &'static str,
        interval: Duration,
    ) -> Self {
        let active = Arc::new(AtomicBool::new(true));
        let last_stdout_at = Arc::new(Mutex::new(Instant::now()));
        let task_active = active.clone();
        let task_last_stdout_at = last_stdout_at.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if !task_active.load(Ordering::Relaxed) {
                    break;
                }
                let idle = task_last_stdout_at
                    .lock()
                    .map(|t| t.elapsed())
                    .unwrap_or_default();
                listener.on_status(AgentStatus::Heartbeat {
                    backend: backend.into(),
                    idle_ms: idle.as_millis().try_into().unwrap_or(u64::MAX),
                });
            }
        });
        Self {
            active,
            last_stdout_at,
        }
    }

    fn mark_stdout(&self) {
        if let Ok(mut last) = self.last_stdout_at.lock() {
            *last = Instant::now();
        }
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
    }
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

// ---- stream-json wire types -------------------------------------------
//
// Mirrors gemini-cli's `JsonStreamEvent` discriminated union (see
// packages/core/src/output/types.ts upstream). We only decode the
// fields chan-llm actually consumes; unknown fields pass through
// silently via serde's default behavior, and `Other` catches future
// event types so a single new variant doesn't break the stream.

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    Init {
        #[serde(default)]
        #[allow(dead_code)]
        session_id: Option<String>,
        #[serde(default)]
        #[allow(dead_code)]
        model: Option<String>,
    },
    Message {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        content: String,
        /// `true` for streaming chunks, `false` for the final
        /// canonical assistant message (which carries the full
        /// accumulated text). Missing flag is treated as a streaming
        /// chunk to keep the legacy fixture shape working.
        #[serde(default)]
        delta: Option<bool>,
    },
    ToolUse {
        tool_name: String,
        tool_id: String,
        #[serde(default)]
        parameters: Json,
    },
    ToolResult {
        tool_id: String,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        error: Option<ToolError>,
    },
    Error {
        #[serde(default)]
        severity: Option<String>,
        message: String,
    },
    Result {
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        error: Option<ResultError>,
        // stats: ignored; chan-llm doesn't surface gemini stats
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, serde::Serialize)]
struct ToolError {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct ResultError {
    #[serde(default)]
    #[allow(dead_code)]
    kind: Option<String>,
    #[serde(default)]
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{StopReason, ToolCall, ToolResult, UserRequest};
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct Collector(Mutex<Vec<Event>>);

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum Event {
        Delta(String),
        ToolCall(String),
        ToolResult(String),
        Error(String),
        Status(AgentStatus),
        Activity(AgentActivity),
        UserRequest,
    }

    impl SessionListener for Collector {
        fn on_delta(&self, d: Delta) {
            self.0.lock().unwrap().push(Event::Delta(d.text));
        }
        fn on_tool_call(&self, c: ToolCall) {
            self.0.lock().unwrap().push(Event::ToolCall(c.name));
        }
        fn on_tool_result(&self, r: ToolResult) {
            self.0.lock().unwrap().push(Event::ToolResult(r.id));
        }
        fn on_done(&self, _: StopReason) {}
        fn on_error(&self, e: String) {
            self.0.lock().unwrap().push(Event::Error(e));
        }
        fn on_status(&self, status: AgentStatus) {
            self.0.lock().unwrap().push(Event::Status(status));
        }
        fn on_activity(&self, activity: AgentActivity) {
            self.0.lock().unwrap().push(Event::Activity(activity));
        }
        fn on_user_request(&self, _: UserRequest) {
            self.0.lock().unwrap().push(Event::UserRequest);
        }
    }

    fn fake_gemini(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("fake_gemini.sh");
        let script = format!("#!/bin/sh\ncat <<'EOF'\n{body}\nEOF\n");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn fake_gemini_script(dir: &std::path::Path, script: &str) -> PathBuf {
        let path = dir.join("fake_gemini.sh");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn backend_for(
        script: &std::path::Path,
        cwd: &std::path::Path,
        timeout: Duration,
    ) -> GeminiCliBackend {
        GeminiCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            cwd.to_path_buf(),
            None,
            timeout,
            false,
            None,
        )
    }

    #[tokio::test]
    async fn streams_assistant_text_and_forwards_tool_events() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"init","timestamp":"2026-05-09T00:00:00Z","session_id":"s1","model":"gemini-2.5-pro"}
{"type":"message","timestamp":"t","role":"assistant","content":"hello ","delta":true}
{"type":"message","timestamp":"t","role":"assistant","content":"world","delta":true}
{"type":"tool_use","timestamp":"t","tool_name":"read_file","tool_id":"call_1","parameters":{"path":"a.md"}}
{"type":"tool_result","timestamp":"t","tool_id":"call_1","status":"success","output":"ok"}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.assistant_text, "hello world");
        assert!(outcome.tool_calls.is_empty(), "v1 returns no tool calls");
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);

        let events = listener.0.lock().unwrap();
        let deltas: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let Event::Delta(t) = e {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(deltas, vec!["hello ", "world"]);
        let tool_calls: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let Event::ToolCall(n) = e {
                    Some(n.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tool_calls, vec!["read_file"]);
        let tool_results: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let Event::ToolResult(id) = e {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tool_results, vec!["call_1"]);
    }

    #[tokio::test]
    async fn emits_spawned_ready_and_session_activity() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"init","timestamp":"t","session_id":"s1","model":"gemini-2.5-pro"}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);

        let events = listener.0.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Spawned {
                backend,
                pid: Some(_),
            }) if backend == "gemini_cli"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Ready {
                backend,
                session_id: Some(id),
                model: Some(model),
                version: None,
            }) if backend == "gemini_cli" && id == "s1" && model == "gemini-2.5-pro"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Activity(AgentActivity::SessionStarted {
                backend,
                session_id: Some(id),
            }) if backend == "gemini_cli" && id == "s1"
        )));
    }

    #[tokio::test]
    async fn heartbeat_fires_while_child_is_alive_but_quiet() {
        let tmp = TempDir::new().unwrap();
        let script = fake_gemini_script(
            tmp.path(),
            r#"#!/bin/sh
printf '%s\n' '{"type":"init","timestamp":"t","session_id":"s1","model":"m"}'
sleep 2
printf '%s\n' '{"type":"result","timestamp":"t","status":"success"}'
"#,
        );
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(3));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);

        let events = listener.0.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Heartbeat { backend, idle_ms })
                if backend == "gemini_cli" && *idle_ms > 0
        )));
    }

    #[tokio::test]
    async fn tool_activity_frames_preserve_compat_callbacks() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"tool_use","timestamp":"t","tool_name":"read_file","tool_id":"call_1","parameters":{"path":"a.md"}}
{"type":"tool_result","timestamp":"t","tool_id":"call_1","status":"success","output":"ok"}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);

        let events = listener.0.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Activity(AgentActivity::ToolStarted { id, name, .. })
                if id == "call_1" && name == "read_file"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Activity(AgentActivity::ToolFinished {
                id,
                output,
                is_error: false,
                ..
            }) if id == "call_1" && output == &Json::String("ok".into())
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ToolCall(name) if name == "read_file")));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ToolResult(id) if id == "call_1")));
    }

    #[tokio::test]
    async fn tool_error_activity_sets_is_error() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"tool_result","timestamp":"t","tool_id":"call_1","status":"error","error":{"type":"permission","message":"denied"}}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);

        let events = listener.0.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Activity(AgentActivity::ToolFinished {
                id,
                output,
                is_error: true,
                ..
            }) if id == "call_1"
                && output.get("message").and_then(Json::as_str) == Some("denied")
        )));
    }

    #[tokio::test]
    async fn numbered_choice_text_remains_plain_delta() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"message","timestamp":"t","role":"assistant","content":"Pick one:\n1. Red\n2. Blue\n3. Green","delta":true}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(
            outcome.assistant_text,
            "Pick one:\n1. Red\n2. Blue\n3. Green"
        );

        let events = listener.0.lock().unwrap();
        let deltas: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Delta(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, vec!["Pick one:\n1. Red\n2. Blue\n3. Green"]);
        assert!(!events.iter().any(|e| matches!(e, Event::UserRequest)));
    }

    #[tokio::test]
    async fn canonical_assistant_message_after_delta_chunks_is_not_re_emitted() {
        // Regression for the same shape of bug as claude_cli's
        // partials/canonical doubling: gemini-cli's NDJSON message
        // events carry a `delta:true|false` discriminator. When
        // streaming, gemini emits `delta:true` chunks; the final
        // canonical assistant message lands as `delta:false` (or
        // absent) carrying the full accumulated text. The dispatch
        // currently emits `on_delta` for every assistant message
        // regardless of the flag, so the canonical event doubles
        // the text seen by the listener.
        //
        // Expected behaviour: deltas are exactly the streamed
        // chunks; the canonical message is reconciled (not re-
        // emitted) so the listener's joined deltas equal the
        // final assistant_text.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"init","timestamp":"t","session_id":"s1","model":"gemini-2.5-pro"}
{"type":"message","timestamp":"t","role":"assistant","content":"hello ","delta":true}
{"type":"message","timestamp":"t","role":"assistant","content":"world","delta":true}
{"type":"message","timestamp":"t","role":"assistant","content":"hello world","delta":false}
{"type":"result","timestamp":"t","status":"success"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = backend_for(&script, tmp.path(), Duration::from_secs(60));
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        let events = listener.0.lock().unwrap();
        let deltas: Vec<String> = events
            .iter()
            .filter_map(|e| {
                if let Event::Delta(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            deltas,
            vec!["hello ".to_string(), "world".to_string()],
            "canonical (delta:false) message must not re-fire on_delta on top of \
             the streamed chunks; got {deltas:?}"
        );
        // Joined deltas must equal the final assistant_text. If we
        // double-counted, this would be 2x.
        let joined: String = deltas.concat();
        assert_eq!(joined, outcome.assistant_text);
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);
    }

    #[tokio::test]
    async fn stream_ended_without_result_event_is_error() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"init","cwd":"."}
{"type":"message","role":"assistant","content":"partial"}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = GeminiCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let events = listener.0.lock().unwrap();
        let errs: Vec<String> = events
            .iter()
            .filter_map(|e| {
                if let Event::Error(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            errs.iter().any(|e| e.contains("stream ended without")),
            "expected stream-ended error: {errs:?}",
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Unhealthy { reason, .. }) if reason == "no_result"
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Status(AgentStatus::Exited { success: false, .. }))));
    }

    #[tokio::test]
    async fn parse_error_emissions_are_rate_limited() {
        let tmp = TempDir::new().unwrap();
        let mut body = String::new();
        for i in 0..12 {
            body.push_str(&format!("not json line {i}\n"));
        }
        body.push_str(r#"{"type":"result","status":"success"}"#);
        let script = fake_gemini(tmp.path(), &body);
        let backend = GeminiCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let _ = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        let events = listener.0.lock().unwrap();
        // Some "not json line N" entries do contain a digit but no
        // '{', so they hit the banner-strip path and are dropped
        // silently. Lines containing braces would hit the parse path.
        // To make the test robust, generate explicit non-banner
        // malformed lines (with a '{' so the banner-strip pass them
        // to serde_json, which then rejects them).
        let parse_errs: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Error(t) if t.contains("parse:") => Some(t.as_str()),
                _ => None,
            })
            .collect();
        // The banner-strip drops lines without '{' silently, so this
        // particular fixture won't hit the parse path. Verify the
        // alternate fixture next.
        let _ = parse_errs;
    }

    #[tokio::test]
    async fn parse_error_with_brace_lines_is_rate_limited() {
        // Lines that contain '{' but aren't valid JSON survive the
        // banner-strip and reach serde_json, which fails: that's the
        // path the rate limit guards. 12 of them -> 5 detailed + 1
        // summary.
        let tmp = TempDir::new().unwrap();
        let mut body = String::new();
        for i in 0..12 {
            body.push_str(&format!("{{ broken line {i}\n"));
        }
        body.push_str(r#"{"type":"result","status":"success"}"#);
        let script = fake_gemini(tmp.path(), &body);
        let backend = GeminiCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let _ = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        let events = listener.0.lock().unwrap();
        let parse_errs: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Error(t) if t.contains("parse:") => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            parse_errs.len(),
            super::super::PARSE_ERROR_EMIT_LIMIT + 1,
            "expected {} parse errors, got: {:?}",
            super::super::PARSE_ERROR_EMIT_LIMIT + 1,
            parse_errs,
        );
        assert!(
            parse_errs
                .iter()
                .any(|e| e.contains("additional parse errors suppressed")),
            "expected summary line: {parse_errs:?}",
        );
    }

    #[tokio::test]
    async fn oversize_line_aborts_with_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("flood.sh");
        std::fs::write(
            &path,
            "#!/bin/sh\nawk 'BEGIN { for (i=0;i<5242880;i++) printf \"x\"; print \"\" }'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let backend = GeminiCliBackend::new(
            vec![path.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let events = listener.0.lock().unwrap();
        let errs: Vec<String> = events
            .iter()
            .filter_map(|e| {
                if let Event::Error(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            errs.iter().any(|e| e.contains("line exceeds")),
            "expected line-cap error: {errs:?}",
        );
    }

    #[tokio::test]
    async fn inactivity_timeout_fires_when_subprocess_emits_nothing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("silent.sh");
        std::fs::write(&path, "#!/bin/sh\nsleep 30\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let backend = GeminiCliBackend::new(
            vec![path.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_millis(200),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let start = std::time::Instant::now();
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        let elapsed = start.elapsed();
        assert_eq!(outcome.stop_reason, StopReason::Error);
        assert!(
            elapsed < Duration::from_secs(5),
            "loop must exit on timeout, not on the sleep ending; elapsed={elapsed:?}",
        );
        let events = listener.0.lock().unwrap();
        let errs: Vec<String> = events
            .iter()
            .filter_map(|e| {
                if let Event::Error(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            errs.iter().any(|e| e.contains("subprocess wedged")),
            "expected wedged-subprocess error: {errs:?}",
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Unhealthy { reason, .. })
                if reason.starts_with("no_output_for_")
        )));
    }

    #[tokio::test]
    async fn nonzero_exit_emits_on_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("fail.sh");
        std::fs::write(&path, "#!/bin/sh\necho boom 1>&2\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let backend = GeminiCliBackend::new(
            vec![path.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let events = listener.0.lock().unwrap();
        let errs: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let Event::Error(t) = e {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            errs.iter()
                .any(|e| e.contains("exit") && e.contains("boom")),
            "errs={errs:?}"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Status(AgentStatus::Unhealthy { reason, detail, .. })
                if reason.contains("exit") && detail.as_deref().is_some_and(|d| d.contains("boom"))
        )));
    }

    #[tokio::test]
    async fn result_error_marks_stop_reason() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"init","timestamp":"t","session_id":"s","model":"m"}
{"type":"result","timestamp":"t","status":"error","error":{"type":"turn_limit","message":"too many turns"}}"#;
        let script = fake_gemini(tmp.path(), body);
        let backend = GeminiCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let events = listener.0.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Error(s) if s.contains("too many turns"))),
            "events={events:?}"
        );
    }

    #[tokio::test]
    async fn skips_banner_glued_to_first_json_line() {
        // gemini-cli emits a status banner without a trailing newline
        // when MCP server registration fails ("MCP issues detected.
        // Run /mcp list for status."), gluing it directly onto the
        // init event. Without the prefix-strip, the parse error
        // bubbles up as an on_error and chan-server turns the whole
        // turn into a 502 even though the assistant text streamed
        // fine afterwards.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("banner.sh");
        let script = "#!/bin/sh\nprintf 'MCP issues detected. Run /mcp list for status.'\n\
            cat <<'EOF'\n\
            {\"type\":\"init\",\"timestamp\":\"t\",\"session_id\":\"s\",\"model\":\"m\"}\n\
            {\"type\":\"message\",\"timestamp\":\"t\",\"role\":\"assistant\",\"content\":\"hi\",\"delta\":true}\n\
            {\"type\":\"result\",\"timestamp\":\"t\",\"status\":\"success\"}\n\
            EOF\n";
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let backend = GeminiCliBackend::new(
            vec![path.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
            false,
            None,
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                &[Message::user("hi")],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.assistant_text, "hi");
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);
        let events = listener.0.lock().unwrap();
        assert!(
            !events.iter().any(|e| matches!(e, Event::Error(_))),
            "banner must not surface as on_error: {events:?}"
        );
    }

    #[test]
    fn render_prompt_labels_each_role() {
        let msgs = vec![
            Message::system("be helpful"),
            Message::user("first"),
            Message::assistant("ok"),
            Message::user("second"),
        ];
        let body = render_prompt(&msgs);
        assert!(body.contains("[system]\nbe helpful"));
        assert!(body.contains("[user]\nfirst"));
        assert!(body.contains("[assistant]\nok"));
        assert!(body.contains("[user]\nsecond"));
    }

    #[test]
    fn mcp_home_forwards_command_verbatim() {
        // The MCP server owns the auto-apply gate; the proxy / MCP
        // command line is forwarded as-is. The v2 chan-server proxy
        // subcommand (`chan __mcp-proxy <socket>`) doesn't accept a
        // `--auto-apply` flag and would clap-error if we appended one,
        // so it's important that we don't.
        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into(), "__mcp-proxy".into(), "/tmp/s".into()],
            },
            None,
        )
        .unwrap();
        let body =
            std::fs::read_to_string(home.path().join(".gemini").join("settings.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let entry = &v["mcpServers"][MCP_SERVER_KEY];
        assert_eq!(entry["command"], "chan");
        assert_eq!(entry["trust"], true);
        let args: Vec<&str> = entry["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        assert_eq!(args, vec!["__mcp-proxy", "/tmp/s"]);
    }

    #[cfg(unix)]
    #[test]
    fn mcp_home_subdirs_and_files_are_user_only() {
        use std::os::unix::fs::PermissionsExt;
        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into()],
            },
            None,
        )
        .unwrap();
        let dot = home.path().join(".gemini");
        let pol = dot.join("policies");
        let settings = dot.join("settings.json");
        let chan_toml = pol.join("chan.toml");
        let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dot), 0o700, ".gemini dir");
        assert_eq!(mode(&pol), 0o700, "policies dir");
        assert_eq!(mode(&settings), 0o600, "settings.json");
        assert_eq!(mode(&chan_toml), 0o600, "chan.toml");
    }

    #[test]
    fn mcp_home_writes_deny_policy_for_native_writes() {
        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into()],
            },
            None,
        )
        .unwrap();
        let policy = std::fs::read_to_string(
            home.path()
                .join(".gemini")
                .join("policies")
                .join("chan.toml"),
        )
        .unwrap();
        // The deny-policy must cover at minimum gemini's native
        // edit + shell entry points, otherwise a tool call could
        // route around the chan MCP server.
        for needle in ["replace", "write_file", "edit", "run_shell_command"] {
            assert!(
                policy.contains(needle),
                "deny policy missing {needle}: {policy}"
            );
        }
        assert!(policy.contains("decision = \"deny\""), "{policy}");
    }

    #[test]
    fn mcp_home_merges_user_settings_and_overrides_chan_server() {
        let user = TempDir::new().unwrap();
        let user_dot = user.path().join(".gemini");
        std::fs::create_dir_all(&user_dot).unwrap();
        // User has oauth-personal selected, plus a stale `chan`
        // mcp entry from a previous chan-llm version that we must
        // overwrite (not merge with).
        std::fs::write(
            user_dot.join("settings.json"),
            r#"{
              "security": {"auth": {"selectedType": "oauth-personal"}},
              "telemetry": {"enabled": false},
              "mcpServers": {"chan": {"command": "/old/chan"}, "other": {"command": "/keep"}}
            }"#,
        )
        .unwrap();

        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into(), "__mcp".into(), "/d".into()],
            },
            Some(user_dot.clone()),
        )
        .unwrap();

        let body =
            std::fs::read_to_string(home.path().join(".gemini").join("settings.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["security"]["auth"]["selectedType"], "oauth-personal");
        assert_eq!(v["telemetry"]["enabled"], false);
        // chan entry rewritten to the current wiring; sibling entries kept.
        assert_eq!(v["mcpServers"]["chan"]["command"], "chan");
        assert_eq!(v["mcpServers"]["chan"]["trust"], true);
        assert_eq!(v["mcpServers"]["other"]["command"], "/keep");
    }

    #[cfg(unix)]
    #[test]
    fn mcp_home_symlinks_user_auth_files() {
        let user = TempDir::new().unwrap();
        let user_dot = user.path().join(".gemini");
        std::fs::create_dir_all(&user_dot).unwrap();
        std::fs::write(user_dot.join("oauth_creds.json"), b"{\"token\":\"x\"}").unwrap();
        std::fs::write(user_dot.join("google_accounts.json"), b"{}").unwrap();
        std::fs::write(user_dot.join("installation_id"), b"id-1").unwrap();
        // state.json deliberately absent: bridge must skip cleanly.

        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into()],
            },
            Some(user_dot.clone()),
        )
        .unwrap();

        let dst = home.path().join(".gemini");
        // Symlink-not-copy: link target must point at the user's
        // real file so OAuth refresh writes flow back upstream.
        let creds_link = std::fs::read_link(dst.join("oauth_creds.json")).unwrap();
        assert_eq!(creds_link, user_dot.join("oauth_creds.json"));
        assert!(dst.join("google_accounts.json").exists());
        assert!(dst.join("installation_id").exists());
        assert!(!dst.join("state.json").exists());
    }

    #[test]
    fn mcp_home_tolerates_missing_user_home() {
        // Common first-run case: user has never invoked gemini-cli
        // before, so the home directory doesn't exist. Spawn must
        // succeed and emit a minimal settings.json with just our
        // chan mcp server.
        let user = TempDir::new().unwrap();
        let absent = user.path().join("does-not-exist").join(".gemini");
        let home = write_gemini_home_with_user(
            &McpWiring {
                command: vec!["chan".into()],
            },
            Some(absent),
        )
        .unwrap();
        let body =
            std::fs::read_to_string(home.path().join(".gemini").join("settings.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["mcpServers"][MCP_SERVER_KEY]["command"], "chan");
    }
}
