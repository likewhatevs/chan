//! ClaudeCli backend: shell-executor wrapper around the `claude` CLI.
//!
//! End-to-end streaming flow, failure modes, the listener contract,
//! and the in-flight hardening plan (per-message partials reset,
//! NDJSON line cap, inactivity timeout, secret redaction, structured
//! error kinds) live in `crates/chan-llm/design.md` sections 6.1
//! through 6.3 and section 13. Keep this header focused on what
//! claude itself does.
//!
//! Two modes, selected at construction time:
//!
//! ### v1: black-box (legacy, `mcp = None`)
//!
//! Claude runs as a full agent. We hand it a prompt and read its
//! stream-json NDJSON output; we do NOT mediate its tool calls.
//! Concretely:
//!
//!   - Claude edits files directly under `cwd` (the drive root)
//!     using its own Read / Write / Edit / Bash tools.
//!   - chan-llm's tool sandbox (path scope, editable-text gate,
//!     auto_apply_writes confirmation) is NOT applied to those
//!     edits.
//!   - We launch claude with `--permission-mode bypassPermissions`
//!     because there is no human in front of claude's stdin to
//!     answer its native permission prompts.
//!
//! v1 stays available for hosts that don't ship a chan-llm-mcp
//! capable binary; tests cover this path.
//!
//! ### v2: MCP-mediated (`mcp = Some(McpWiring { .. })`)
//!
//! Closes the gap by routing claude's writes through chan-llm's
//! MCP server. The host (chan-server, future native shell) supplies
//! an `mcp_command` (e.g. `["chan", "__mcp", "/drive/path"]`); we
//! write a one-shot `--mcp-config` JSON pointing at it and launch
//! claude with:
//!
//!   - `--mcp-config <tmp>.json` pointing at the chan-llm MCP
//!     server (the host binary in stdio mode).
//!   - `--allowedTools` enumerating chan-llm's MCP tools plus
//!     claude's read-only natives (Read / Glob / Grep). Claude
//!     auto-approves these without an interactive prompt.
//!   - `--disallowedTools Write,Edit,MultiEdit,NotebookEdit,Bash`
//!     so writes are forced through chan-llm's tools, where
//!     chan-drive's path sandbox + editable-text gate apply.
//!   - `--permission-mode default` (we drop `bypassPermissions`):
//!     anything not allow-listed blocks, which matches the contract.
//!
//! The auto-apply gate is owned by the MCP server side (in chan-
//! server, the bridge reads `auto_apply_writes` per connection from
//! the live config). When it's off, `write_file` returns a "deferred"
//! error to claude (the host-approval side channel for resuming
//! claude mid-call is tracked in chan-llm issue #1).
//!
//! Wire format:
//!
//!   - Input: chan-llm's transcript is concatenated into a single
//!     prompt string (system / user / assistant labels) and piped
//!     to claude on stdin. Stateless per-call; multi-turn fidelity
//!     is lossy (assistant turns become labelled text), which is
//!     a v1 tradeoff. Switching to `--input-format stream-json`
//!     with `--resume <session-id>` is the natural follow-up if
//!     conversation continuity becomes a problem.
//!
//!   - Output: claude emits NDJSON on stdout. Events of interest:
//!     - `{"type":"system","subtype":"init",...}` (ignored)
//!     - `{"type":"stream_event","event":{...}}` (token-level
//!       partials, see below)
//!     - `{"type":"assistant","message":{...}}` (text + tools)
//!     - `{"type":"user","message":{...}}` (tool results)
//!     - `{"type":"result","subtype":"...",...}` (end of turn)
//!
//!     We launch claude with `--include-partial-messages` so each
//!     assistant turn arrives twice: first as a sequence of
//!     `stream_event` envelopes carrying Anthropic SDK partial
//!     events (`content_block_delta` with `text_delta`), then once
//!     more as the final `assistant` event with the assembled
//!     content. We emit `on_delta` from partial `text_delta`s so
//!     UI consumers see typewriter-style updates, and suppress the
//!     redundant `on_delta` on the final assistant event when any
//!     partial text streamed (we still accumulate the canonical
//!     text into the Outcome from one path or the other, never
//!     both). Tool-use blocks are not streamed incrementally:
//!     `on_tool_call` fires from the final assistant event so the
//!     listener gets a complete `input` payload.
//!
//!     Inside an assistant message, content blocks are either
//!     `{"type":"text","text":"..."}` (emit on_delta when no
//!     partials preceded it) or `{"type":"tool_use","id":"...",
//!     "name":"...","input":...}` (forward to listener via
//!     on_tool_call for visibility; not executed by chan-llm).
//!     Tool results inside user messages are forwarded as
//!     on_tool_result. Both are observational in v1; the
//!     orchestration loop treats `Outcome.tool_calls` as empty so
//!     it exits after one backend turn.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value as Json;
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::session::{
    AgentActivity, AgentStatus, Delta, Message, Role, SessionListener, StopReason, ToolCall,
    ToolResult, UserOption, UserQuestion, UserRequest,
};
use crate::tools::ToolSchema;

use super::{
    read_line_capped, sanitize_env_for_claude_cli, spawn_stderr_drainer, Backend, Outcome,
    StderrDrainer, NDJSON_LINE_CAP_BYTES, PARSE_ERROR_EMIT_LIMIT,
};

/// Default command to launch claude. Plain `claude` so PATH wins;
/// users can override via `LlmConfig.claude_cli.cmd` when claude
/// lives somewhere non-standard or when wrapping the binary in
/// `nix shell` / `flatpak run` / similar.
pub fn default_cmd() -> Vec<String> {
    vec!["claude".to_string()]
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

/// MCP server name as it appears under `mcpServers` in the
/// generated `--mcp-config` JSON. Surfaced to claude as the
/// `mcp__<server>__<tool>` tool prefix and used by the
/// `--allowedTools` allowlist below.
const MCP_SERVER_KEY: &str = "chan";

/// Tools claude is allowed to call without an interactive prompt.
/// Read / Glob / Grep are claude's native read-only tools; they
/// can touch any file under cwd but don't write. The `mcp__`
/// entries are the chan-llm MCP server's tools (their dispatch
/// runs through chan-drive's gates).
const ALLOWED_TOOLS: &str = concat!(
    "Read,Glob,Grep,",
    "mcp__chan__read_file,",
    "mcp__chan__write_file,",
    "mcp__chan__list_files,",
    "mcp__chan__search_content,",
    "mcp__chan__read_image,",
    "mcp__chan__graph_neighbors,",
    "mcp__chan__graph_tags,",
    "mcp__chan__graph_files_with_tag,",
    "mcp__chan__repo_report",
);

/// Tools claude is explicitly NOT allowed to use in v2 mode.
/// Forces every mutation through chan-llm's MCP `write_file`,
/// where chan-drive's path sandbox and editable-text gate apply.
/// Bash is denied because it would otherwise let the agent
/// reach around the gates.
const DISALLOWED_TOOLS: &str = "Write,Edit,MultiEdit,NotebookEdit,Bash";

#[derive(Debug)]
pub struct ClaudeCliBackend {
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
    /// (`sanitize_env_strict`) instead of the loose `ANTHROPIC_` /
    /// `CLAUDE_` prefix match. Resolved from
    /// `LlmConfig.hardened_subprocess_env`.
    hardened_env: bool,
    path_env: Option<OsString>,
}

impl ClaudeCliBackend {
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
impl Backend for ClaudeCliBackend {
    async fn run(
        &self,
        messages: &[Message],
        _tools: &[ToolSchema],
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let (system, prompt) = split_system_and_prompt(messages);

        let Some((bin, leading)) = self.cmd.split_first() else {
            listener.on_error("claude_cli: empty cmd".into());
            return Outcome::error();
        };

        let mut command = Command::new(bin);
        // Drop the parent env so unrelated secrets (OPENAI_API_KEY,
        // GH_TOKEN, AWS_*) don't leak into a spawned child's
        // /proc/<pid>/environ. The wrapper picks the loose
        // (prefix-based) or strict (explicit-name) variant based on
        // `hardened_env`; loose is the interactive-shell default,
        // strict tightens for service-host deployments where
        // `ANTHROPIC_BEDROCK_BASE_URL` / `ANTHROPIC_CUSTOM_HEADERS`
        // could come from a tainted parent env.
        sanitize_env_for_claude_cli(&mut command, self.hardened_env);
        if let Some(path) = self.path_env.as_ref() {
            command.env("PATH", path);
        }
        // Kill the spawned claude on Drop. Every normal exit path
        // already calls `child.kill().await` explicitly, but a panic
        // inside this async fn would otherwise leave the subprocess
        // running until it noticed its stdin was closed (or
        // forever if no I/O was pending). The guard ensures that an
        // unexpected unwind from anywhere below (deserialization,
        // listener callback, channel write) reaps the child.
        command.kill_on_drop(true);
        command
            .args(leading)
            .arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("text")
            .arg("--include-partial-messages")
            .arg("--verbose")
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // v1 black-box mode: no host-supplied MCP wiring, so we
        // bypass claude's per-tool prompts and let it use its own
        // write/edit/bash tools against the drive directly. v2
        // mode: --allowedTools auto-approves the chan-llm MCP
        // tools (and read-only natives) while --disallowedTools
        // blocks claude's native writes; the default permission
        // mode applies to everything else.
        let mcp_config_file = match self.mcp.as_ref() {
            None => {
                command.arg("--permission-mode").arg("bypassPermissions");
                None
            }
            Some(wiring) => {
                let file = match write_mcp_config(wiring) {
                    Ok(f) => f,
                    Err(e) => {
                        listener.on_error(format!("claude_cli mcp-config: {e}"));
                        return Outcome::error();
                    }
                };
                command
                    .arg("--mcp-config")
                    .arg(file.path())
                    .arg("--allowedTools")
                    .arg(ALLOWED_TOOLS)
                    .arg("--disallowedTools")
                    .arg(DISALLOWED_TOOLS);
                Some(file)
            }
        };

        // Always inject the chan host's CLI session directive so
        // the agent doesn't fall back to its "paste proposal in
        // chat and wait for verbal approval" default; chan wraps
        // every write_file in a diff-card review UI, so the model
        // must emit the tool call directly. Order: chan directive
        // first, then the host's system message (which carries
        // per-conversation scope + tool catalog). claude-code
        // appends both to its own built-in agent instructions via
        // --append-system-prompt; the most-recent block wins for
        // conflicts, so per-session host rules still ride above
        // the directive's general guidance.
        let combined_system = match system.as_deref() {
            Some(host) => format!("{}\n\n{}", crate::prompts::CLI_SESSION_DIRECTIVE, host),
            None => crate::prompts::CLI_SESSION_DIRECTIVE.to_string(),
        };
        command.arg("--append-system-prompt").arg(&combined_system);
        if let Some(model) = self.model.as_deref() {
            command.arg("--model").arg(model);
        }
        for arg in &self.extra_args {
            command.arg(arg);
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                listener.on_error(format!("claude_cli spawn: {e}"));
                return Outcome::error();
            }
        };
        listener.on_status(AgentStatus::Spawned {
            backend: "claude_cli".into(),
            pid: child.id(),
        });

        // Pipe the rendered transcript on stdin. Drop stdin so
        // claude sees EOF and processes the prompt. Take stderr
        // BEFORE writing stdin so the drainer is already running if
        // the write fails (the failure surface is "claude died on
        // startup", and we want its stderr to come through).
        let stderr_drainer = spawn_stderr_drainer(child.stderr.take());
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                let stderr_snippet = match stderr_drainer {
                    Some(d) => d.finish().await,
                    None => String::new(),
                };
                let snippet = truncate(&stderr_snippet, 800);
                let _ = child.kill().await;
                let _ = child.wait().await;
                if snippet.is_empty() {
                    listener.on_error(format!("claude_cli stdin: {e}"));
                } else {
                    listener.on_error(format!("claude_cli stdin: {e}; stderr: {snippet}"));
                }
                return Outcome::error();
            }
            drop(stdin);
        }

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                if let Some(d) = stderr_drainer {
                    let _ = d.finish().await;
                }
                listener.on_error("claude_cli: stdout not piped".into());
                let _ = child.kill().await;
                return Outcome::error();
            }
        };
        let mut stderr_drainer: Option<StderrDrainer> = stderr_drainer;

        let mut reader = BufReader::new(stdout);
        let mut line_buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut stop = StopReason::EndOfTurn;
        let mut saw_result = false;
        // FIFO of per-message partial-text trackers. Each entry is a
        // `block_index -> already-streamed text` map for one assistant
        // message. `message_start` partials push a fresh map at the
        // BACK (the message currently being filled); the final
        // `assistant` event pops from the FRONT (the oldest un-
        // reconciled message). FIFO matters when claude-cli emits the
        // SDK's `message_start` for message N+1 BEFORE it synthesizes
        // the final `assistant` envelope for message N; the older
        // queue model (a single index-keyed map cleared on
        // message_start) would lose message N's partials at that
        // moment and re-emit the whole text from the canonical
        // block, doubling on_delta.
        let mut partial_buffers: VecDeque<HashMap<usize, String>> = VecDeque::new();
        // Block-index -> tool_use id for the in-flight assistant
        // message. Populated by `content_block_start { tool_use }` so
        // a follow-up `content_block_delta { input_json_delta }` (whose
        // envelope carries only the index) can be tagged with the tool
        // id the partial JSON belongs to. Cleared at every
        // `message_start` because indexes reset per message.
        let mut tool_ids_by_index: HashMap<usize, String> = HashMap::new();
        let mut parse_errors_emitted = 0usize;
        let mut parse_errors_silenced = 0usize;
        let heartbeat = Heartbeat::spawn(
            listener.clone(),
            "claude_cli",
            heartbeat_interval(self.inactivity_timeout),
        );

        loop {
            if cancel.load(Ordering::Relaxed) {
                // Kill the subprocess and return what we have. The
                // assistant_text we collected so far stays on the
                // outcome so the host can keep partial UI state. The
                // drainer flushes on the natural pipe close that
                // follows the kill; we don't await it on the cancel
                // path because the user is already gone.
                let _ = child.kill().await;
                let _ = child.wait().await;
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_status(AgentStatus::Cancelled {
                    backend: "claude_cli".into(),
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
                    // Either an I/O failure on the pipe or the per-line
                    // cap fired. Both are unrecoverable for this turn.
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let stderr_snippet = match stderr_drainer.take() {
                        Some(d) => truncate(&d.finish().await, 800),
                        None => String::new(),
                    };
                    if stderr_snippet.is_empty() {
                        listener.on_error(format!("claude_cli stdout: {e}"));
                    } else {
                        listener
                            .on_error(format!("claude_cli stdout: {e}; stderr: {stderr_snippet}"));
                    }
                    listener.on_status(AgentStatus::Unhealthy {
                        backend: "claude_cli".into(),
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
                        "claude_cli: no output for {}s; subprocess wedged",
                        self.inactivity_timeout.as_secs(),
                    );
                    if stderr_snippet.is_empty() {
                        listener.on_error(base);
                    } else {
                        listener.on_error(format!("{base}; stderr: {stderr_snippet}"));
                    }
                    listener.on_status(AgentStatus::Unhealthy {
                        backend: "claude_cli".into(),
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
            // The trailing '\n' is harmless to serde_json but we strip
            // it for the parse-error preview. Non-UTF-8 bytes mean a
            // garbled pipe; surface as a parse error and keep going.
            let line = match std::str::from_utf8(&line_buf) {
                Ok(s) => s.trim_end_matches('\n'),
                Err(_) => {
                    if parse_errors_emitted < PARSE_ERROR_EMIT_LIMIT {
                        listener.on_error("claude_cli stdout: non-utf8 line".into());
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
            let event: StreamEvent = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(e) => {
                    if parse_errors_emitted < PARSE_ERROR_EMIT_LIMIT {
                        listener.on_error(format!(
                            "claude_cli parse: {e}; raw: {}",
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
                StreamEvent::System {
                    subtype,
                    session_id,
                    model,
                    claude_code_version,
                    status,
                } => {
                    if subtype == "init" {
                        listener.on_status(AgentStatus::Ready {
                            backend: "claude_cli".into(),
                            session_id: session_id.clone(),
                            model,
                            version: claude_code_version,
                        });
                        listener.on_activity(AgentActivity::SessionStarted {
                            backend: "claude_cli".into(),
                            session_id,
                        });
                    } else if subtype == "status" {
                        listener.on_status(AgentStatus::Thinking {
                            backend: "claude_cli".into(),
                            status,
                        });
                    }
                }
                StreamEvent::Partial {
                    event: partial,
                    parent_tool_use_id,
                } => match partial {
                    PartialEvent::MessageStart { message } => {
                        // A new assistant message begins. Push a
                        // fresh tracker at the back; the previous
                        // message's tracker stays in the queue
                        // until its final `assistant` event consumes
                        // it. This is what prevents a premature
                        // `message_start` for message N+1 from
                        // clearing message N's partials before its
                        // final event arrives.
                        partial_buffers.push_back(HashMap::new());
                        // Block indexes reset per message, so the
                        // index-keyed tool_use lookup must reset too;
                        // otherwise a tool at index 1 of message N+1
                        // would collide with index 1 of message N.
                        tool_ids_by_index.clear();
                        listener.on_activity(AgentActivity::MessageStarted {
                            backend: "claude_cli".into(),
                            message_id: message.and_then(|m| m.id),
                            parent_id: parent_tool_use_id,
                        });
                    }
                    PartialEvent::ContentBlockStart {
                        index,
                        content_block: PartialContentBlock::ToolUse { id, name, .. },
                    } => {
                        tool_ids_by_index.insert(index, id.clone());
                        listener.on_activity(AgentActivity::ToolStarted {
                            backend: "claude_cli".into(),
                            id,
                            name,
                            parent_id: parent_tool_use_id,
                        });
                    }
                    PartialEvent::ContentBlockStart {
                        content_block: PartialContentBlock::Thinking,
                        ..
                    } => {
                        listener.on_activity(AgentActivity::ThinkingStarted {
                            backend: "claude_cli".into(),
                            parent_id: parent_tool_use_id,
                        });
                    }
                    PartialEvent::ContentBlockDelta {
                        index,
                        delta: PartialDelta::TextDelta { text },
                    } => {
                        if !text.is_empty() {
                            listener.on_delta(Delta { text: text.clone() });
                            assistant_text.push_str(&text);
                            // Write to the most recently started
                            // message's tracker. If no MessageStart
                            // has been seen yet (claude-cli stripped
                            // it, or stream resumed mid-message),
                            // implicitly start one so the next
                            // Assistant event can still dedupe.
                            if partial_buffers.is_empty() {
                                partial_buffers.push_back(HashMap::new());
                            }
                            if let Some(buf) = partial_buffers.back_mut() {
                                buf.entry(index).or_default().push_str(&text);
                            }
                            if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                                listener.on_error(format!(
                                    "claude_cli stream: assistant text exceeded {} bytes; aborting",
                                    super::ASSISTANT_TEXT_CAP_BYTES,
                                ));
                                let _ = child.kill().await;
                                return Outcome::error();
                            }
                        }
                    }
                    PartialEvent::ContentBlockDelta {
                        index,
                        delta: PartialDelta::InputJsonDelta { partial_json },
                    } => {
                        listener.on_activity(AgentActivity::ToolArgsDelta {
                            backend: "claude_cli".into(),
                            id: tool_ids_by_index.get(&index).cloned(),
                            partial_json,
                            parent_id: parent_tool_use_id,
                        });
                    }
                    PartialEvent::ContentBlockDelta {
                        delta: PartialDelta::ThinkingDelta { thinking },
                        ..
                    } => {
                        listener.on_activity(AgentActivity::ThinkingDelta {
                            backend: "claude_cli".into(),
                            text: thinking,
                            parent_id: parent_tool_use_id,
                        });
                    }
                    PartialEvent::MessageDelta { delta } => {
                        listener.on_status(AgentStatus::TurnStopping {
                            backend: "claude_cli".into(),
                            reason: delta.stop_reason,
                        });
                    }
                    PartialEvent::ContentBlockStart { .. }
                    | PartialEvent::ContentBlockDelta { .. }
                    | PartialEvent::Other => {}
                },
                StreamEvent::Assistant {
                    message,
                    parent_tool_use_id,
                } => {
                    // Consume the oldest un-reconciled per-message
                    // tracker. Empty / None means claude-cli buffered
                    // this message without emitting partials for it,
                    // which the canonical-emit branch below handles
                    // by falling back to the full block text.
                    let partials = partial_buffers.pop_front().unwrap_or_default();
                    tracing::debug!(
                        block_count = message.content.len(),
                        queued_after_pop = partial_buffers.len(),
                        partials_indices = ?partials.keys().collect::<Vec<_>>(),
                        partials_lens = ?partials
                            .iter()
                            .map(|(k, v)| (*k, v.len()))
                            .collect::<Vec<_>>(),
                        "claude_cli: Assistant event received",
                    );
                    for (i, block) in message.content.into_iter().enumerate() {
                        match block {
                            ContentBlock::Text { text } => {
                                // Emit only the slice of the canonical
                                // block that wasn't already streamed
                                // via partials at this index. Covers
                                // three cases:
                                //   1. All partials matched: skip.
                                //   2. No partials for this index
                                //      (claude buffered): emit full.
                                //   3. Partials covered a prefix
                                //      (unusual; happens if a network
                                //      drop kills mid-block): emit
                                //      the suffix.
                                let already = partials.get(&i).map(String::as_str).unwrap_or("");
                                // Decide what (if anything) to emit
                                // from this canonical block:
                                //
                                //   1. No partials seen for this
                                //      index: claude buffered the
                                //      whole block; emit the full
                                //      canonical text.
                                //   2. Partials are an exact prefix
                                //      of the canonical text: emit
                                //      only the missing suffix (the
                                //      common case is suffix=="", a
                                //      complete match, which skips).
                                //   3. Partials/canonical drift (the
                                //      partials don't prefix-match
                                //      the canonical block): keep
                                //      the partials' view and DROP
                                //      the canonical. Re-emitting it
                                //      would double both the host's
                                //      on_delta snapshot and our
                                //      assistant_text. The user has
                                //      already seen the partials
                                //      live; a one-block divergence
                                //      is preferable to N copies of
                                //      the same paragraph.
                                let emit: Option<&str> = if already.is_empty() {
                                    Some(text.as_str())
                                } else if let Some(suffix) = text.strip_prefix(already) {
                                    if suffix.is_empty() {
                                        None
                                    } else {
                                        Some(suffix)
                                    }
                                } else {
                                    tracing::warn!(
                                        block_index = i,
                                        partials_bytes = already.len(),
                                        canonical_bytes = text.len(),
                                        "claude_cli: partials/canonical drift; \
                                         keeping partials, dropping canonical block to avoid \
                                         doubled assistant_text",
                                    );
                                    None
                                };
                                if let Some(s) = emit {
                                    listener.on_delta(Delta {
                                        text: s.to_string(),
                                    });
                                    assistant_text.push_str(s);
                                    if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                                        listener.on_error(format!(
                                            "claude_cli stream: assistant text exceeded {} bytes; aborting",
                                            super::ASSISTANT_TEXT_CAP_BYTES,
                                        ));
                                        let _ = child.kill().await;
                                        return Outcome::error();
                                    }
                                }
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                listener.on_activity(AgentActivity::ToolFinished {
                                    backend: "claude_cli".into(),
                                    id: id.clone(),
                                    name: Some(name.clone()),
                                    output: input.clone(),
                                    is_error: false,
                                    parent_id: parent_tool_use_id.clone(),
                                });
                                if name == "AskUserQuestion" {
                                    if let Some(request) = ask_user_question_request(
                                        &id,
                                        &input,
                                        parent_tool_use_id.clone(),
                                    ) {
                                        listener.on_user_request(request);
                                    }
                                } else if name == "SendUserMessage" {
                                    if let Some(text) = input.get("message").and_then(Json::as_str)
                                    {
                                        listener.on_activity(AgentActivity::AgentNote {
                                            backend: "claude_cli".into(),
                                            text: text.to_string(),
                                            parent_id: parent_tool_use_id.clone(),
                                        });
                                    }
                                }
                                listener.on_tool_call(ToolCall {
                                    id,
                                    name,
                                    args: input,
                                });
                            }
                            // ToolResult blocks belong to user messages
                            // in claude's protocol; ignore if one shows
                            // up here. Other catches future block types.
                            ContentBlock::ToolResult { .. } | ContentBlock::Other => {}
                        }
                    }
                }
                StreamEvent::User {
                    message,
                    parent_tool_use_id,
                } => {
                    for block in message.content {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                            ..
                        } = block
                        {
                            listener.on_activity(AgentActivity::ToolFinished {
                                backend: "claude_cli".into(),
                                id: tool_use_id.clone(),
                                name: None,
                                output: content.clone(),
                                is_error: is_error.unwrap_or(false),
                                parent_id: parent_tool_use_id.clone(),
                            });
                            listener.on_tool_result(ToolResult {
                                id: tool_use_id,
                                output: content,
                            });
                        }
                    }
                }
                StreamEvent::RateLimit { rate_limit_info } => {
                    listener.on_status(AgentStatus::RateLimit {
                        backend: "claude_cli".into(),
                        status: rate_limit_info
                            .get("status")
                            .and_then(Json::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                        resets_at: rate_limit_info
                            .get("resetsAt")
                            .and_then(Json::as_str)
                            .map(str::to_owned),
                        rate_limit_type: rate_limit_info
                            .get("rateLimitType")
                            .and_then(Json::as_str)
                            .map(str::to_owned),
                        in_overage: rate_limit_info
                            .get("isUsingOverage")
                            .and_then(Json::as_bool)
                            .unwrap_or(false),
                    });
                }
                StreamEvent::Result {
                    subtype,
                    result,
                    is_error,
                    permission_denials,
                } => {
                    // Denials are independent of success/error; surface
                    // them first so the frontend can attribute "we
                    // tried X but it was blocked" before reading any
                    // Unhealthy that follows.
                    for denial in permission_denials {
                        listener.on_activity(AgentActivity::ToolDenied {
                            backend: "claude_cli".into(),
                            id: denial.tool_use_id,
                            name: denial.tool_name,
                            reason: None,
                            input: denial.tool_input,
                            parent_id: None,
                        });
                    }
                    saw_result = true;
                    if is_error.unwrap_or(false) {
                        let msg = result
                            .clone()
                            .unwrap_or_else(|| format!("claude exit ({subtype})"));
                        listener.on_error(format!("claude_cli result: {msg}"));
                        listener.on_status(AgentStatus::Unhealthy {
                            backend: "claude_cli".into(),
                            reason: subtype,
                            detail: Some(msg),
                        });
                        stop = StopReason::Error;
                    } else if assistant_text.is_empty() {
                        // claude's `result.result` carries the
                        // final assistant text; if no incremental
                        // text deltas arrived (e.g. claude emitted
                        // tool calls only and we lost them), fall
                        // back to the result string so the
                        // transcript isn't blank.
                        if let Some(text) = result {
                            if !text.is_empty() {
                                listener.on_delta(Delta { text: text.clone() });
                                if text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                                    listener.on_error(format!(
                                        "claude_cli result: assistant text exceeded {} bytes",
                                        super::ASSISTANT_TEXT_CAP_BYTES,
                                    ));
                                    stop = StopReason::Error;
                                } else {
                                    assistant_text.push_str(&text);
                                }
                            }
                        }
                    }
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
                listener.on_error(format!("claude_cli wait: {e}"));
                listener.on_status(AgentStatus::Unhealthy {
                    backend: "claude_cli".into(),
                    reason: "wait".into(),
                    detail: Some(e.to_string()),
                });
                return Outcome::error();
            }
        };
        if !status.success() {
            // Wait for the drainer to flush whatever's still in the
            // pipe before reading the captured buffer. The drainer
            // exits naturally when the child's stderr closes, which
            // happened on the child exit above.
            let stderr_text = match stderr_drainer.take() {
                Some(d) => d.finish().await,
                None => String::new(),
            };
            let snippet = truncate(&stderr_text, 800);
            listener.on_error(format!("claude_cli exit {status}: {snippet}"));
            listener.on_status(AgentStatus::Unhealthy {
                backend: "claude_cli".into(),
                reason: format!("exit:{status}"),
                detail: if snippet.is_empty() {
                    None
                } else {
                    Some(snippet)
                },
            });
            return Outcome::error();
        }
        listener.on_status(AgentStatus::Exited {
            backend: "claude_cli".into(),
            code: status.code(),
            success: true,
        });
        // Drainer is no longer needed on the success path; let it
        // wind down on its own as the pipe closes. Awaiting would
        // serialise the unrelated drainer task against the happy
        // path, so we just drop it.
        drop(stderr_drainer);
        if parse_errors_silenced > 0 {
            // The first PARSE_ERROR_EMIT_LIMIT failures arrived as
            // their own on_error events with the raw-line preview;
            // anything past that gets summarised here so the host
            // knows the cap fired without seeing thousands of frames.
            listener.on_error(format!(
                "claude_cli parse: {parse_errors_silenced} additional parse errors suppressed",
            ));
        }
        if !saw_result && stop != StopReason::Error {
            // claude exited cleanly but never emitted a `result`
            // event. The transcript may be incomplete (truncated
            // mid-message, missing the final stop reason). Treat as
            // an error so the host renders an actionable state
            // instead of presenting a half-message as a complete
            // reply.
            listener.on_error("claude_cli: stream ended without a result event".into());
            listener.on_status(AgentStatus::Unhealthy {
                backend: "claude_cli".into(),
                reason: "no_terminal".into(),
                detail: None,
            });
            stop = StopReason::Error;
        }

        // Drop the temp config now that claude has exited; named
        // explicit so the lifetime tie is obvious to readers.
        drop(mcp_config_file);

        Outcome {
            assistant_text,
            // Claude runs its own tool loop in both modes. We
            // surface tool calls / results to the listener for
            // visibility but don't ask the orchestrator to execute
            // them, so this loop terminates after one backend turn.
            tool_calls: Vec::new(),
            stop_reason: stop,
        }
    }
}

/// Write a temporary `--mcp-config` JSON for claude. Format:
///
/// ```json
/// { "mcpServers": { "chan": { "command": "...", "args": [...] } } }
/// ```
///
/// The host-supplied `wiring.command` is split into argv[0] (the
/// binary) and the rest (its args); we forward it verbatim. The
/// auto-apply gate is owned by the MCP server side, so no extra
/// flag is appended here. The returned `NamedTempFile` must outlive
/// the claude subprocess so the path remains valid.
fn write_mcp_config(wiring: &McpWiring) -> std::io::Result<NamedTempFile> {
    let (bin, base_args) = wiring.command.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty mcp_command")
    })?;
    let args: Vec<String> = base_args.to_vec();
    let body = serde_json::json!({
        "mcpServers": {
            MCP_SERVER_KEY: {
                "command": bin,
                "args": args,
            }
        }
    });
    let mut file = NamedTempFile::with_suffix(".json")?;
    use std::io::Write;
    file.write_all(serde_json::to_string(&body)?.as_bytes())?;
    file.flush()?;
    Ok(file)
}

/// Split the chan-llm transcript into:
///   - a single concatenated system prompt (passed via
///     `--append-system-prompt`), or None when no system messages
///     are present
///   - a labelled text rendering of the rest of the turns (passed
///     on stdin as the user prompt)
///
/// Lossy for assistant turns: claude in `-p` mode treats stdin as
/// a single user message, so prior assistant turns are rendered as
/// labelled prose. v1 accepts the loss; v2 will switch to
/// `--input-format stream-json` with proper role fidelity.
fn split_system_and_prompt(messages: &[Message]) -> (Option<String>, String) {
    let mut system_chunks: Vec<&str> = Vec::new();
    let mut body = String::new();
    for m in messages {
        match m.role {
            Role::System => system_chunks.push(&m.content),
            Role::User => {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str("[user]\n");
                body.push_str(&m.content);
            }
            Role::Assistant => {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str("[assistant]\n");
                body.push_str(&m.content);
            }
            Role::Tool => {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str("[tool_result ");
                body.push_str(m.tool_call_id.as_deref().unwrap_or(""));
                body.push_str("]\n");
                body.push_str(&m.content);
            }
        }
    }
    let system = if system_chunks.is_empty() {
        None
    } else {
        Some(system_chunks.join("\n\n"))
    };
    (system, body)
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
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

fn ask_user_question_request(
    id: &str,
    input: &Json,
    parent_id: Option<String>,
) -> Option<UserRequest> {
    let raw_questions = input.get("questions")?.as_array()?;
    let mut questions = Vec::with_capacity(raw_questions.len());
    for raw in raw_questions {
        let question = raw.get("question")?.as_str()?.to_string();
        let options = raw
            .get("options")
            .and_then(Json::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| {
                        Some(UserOption {
                            label: value.get("label")?.as_str()?.to_string(),
                            description: value
                                .get("description")
                                .and_then(Json::as_str)
                                .map(str::to_owned),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        questions.push(UserQuestion {
            question,
            header: raw.get("header").and_then(Json::as_str).map(str::to_owned),
            multi_select: raw
                .get("multiSelect")
                .or_else(|| raw.get("multi_select"))
                .and_then(Json::as_bool)
                .unwrap_or(false),
            options,
        });
    }
    Some(UserRequest::Survey {
        backend: "claude_cli".into(),
        id: id.to_string(),
        questions,
        parent_id,
    })
}

// ---- stream-json wire types --------------------------------------------

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    System {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        claude_code_version: Option<String>,
        #[serde(default)]
        status: Option<String>,
    },
    Assistant {
        message: AssistantMessage,
        /// Non-null when this assistant message came from a sub-agent
        /// spawned by claude's Task tool. The frontend groups nested
        /// agent activity under the parent tool_use id.
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    User {
        message: UserMessage,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    /// Anthropic SDK partial event envelope, emitted when claude is
    /// launched with `--include-partial-messages`. We only decode
    /// `content_block_delta` -> `text_delta` for token-level UI
    /// streaming; other partial event types (`message_start`,
    /// `content_block_start`, `input_json_delta`, `message_delta`,
    /// `message_stop`, ...) pass through as `Other` and are
    /// resolved from the final `assistant` event instead.
    #[serde(rename = "stream_event")]
    Partial {
        event: PartialEvent,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    #[serde(rename = "rate_limit_event")]
    RateLimit {
        #[serde(default)]
        rate_limit_info: Json,
    },
    Result {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
        /// Tools claude refused or that the host cancelled before
        /// answering. Each entry carries the original tool_input verbatim
        /// so the frontend can render "we tried to call X with Y but it
        /// was denied/cancelled" without re-querying claude.
        #[serde(default)]
        permission_denials: Vec<PermissionDenial>,
    },
    /// system / future event types pass through silently.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PartialEvent {
    /// Boundary marker between assistant messages. The per-block
    /// partial-text tracker resets here so the next final
    /// `assistant` event compares against a fresh slate.
    MessageStart {
        #[serde(default)]
        message: Option<PartialMessage>,
    },
    ContentBlockStart {
        #[serde(default)]
        index: usize,
        content_block: PartialContentBlock,
    },
    /// Incremental delta for the content block at `index`. Indexes
    /// are positional within a single assistant message and reset
    /// at every `MessageStart`.
    ContentBlockDelta { index: usize, delta: PartialDelta },
    MessageDelta {
        #[serde(default)]
        delta: PartialMessageDelta,
    },
    #[serde(other)]
    Other,
}

#[derive(Default, Deserialize)]
struct PartialMessage {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Default, Deserialize)]
struct PartialMessageDelta {
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PartialContentBlock {
    ToolUse {
        id: String,
        name: String,
        #[serde(default, rename = "input")]
        _input: Json,
    },
    Thinking,
    #[serde(other)]
    #[default]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PartialDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        #[serde(default)]
        partial_json: String,
    },
    ThinkingDelta {
        #[serde(default)]
        thinking: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct AssistantMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct UserMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

/// Subset of a `result.permission_denials[]` entry. Reason isn't
/// surfaced by claude; the entry's mere presence is the signal.
#[derive(Deserialize)]
struct PermissionDenial {
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Json,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Json,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Json,
        #[serde(default)]
        #[allow(dead_code)]
        is_error: Option<bool>,
    },
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{StopReason, ToolCall, ToolResult};
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
        UserRequest(UserRequest),
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
        fn on_status(&self, s: AgentStatus) {
            self.0.lock().unwrap().push(Event::Status(s));
        }
        fn on_activity(&self, a: AgentActivity) {
            self.0.lock().unwrap().push(Event::Activity(a));
        }
        fn on_user_request(&self, r: UserRequest) {
            self.0.lock().unwrap().push(Event::UserRequest(r));
        }
    }

    /// Write a fake `claude` shell script that emits canned NDJSON.
    /// Returns the script path (which becomes the backend's `cmd[0]`).
    fn fake_claude(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("fake_claude.sh");
        let script = format!("#!/bin/sh\ncat <<'EOF'\n{body}\nEOF\n");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[tokio::test]
    async fn streams_assistant_text_and_forwards_tool_events() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"hello "}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"world"},{"type":"tool_use","id":"call_1","name":"Read","input":{"path":"a.md"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"call_1","content":"ok"}]}}
{"type":"result","subtype":"success","result":"hello world","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
        assert_eq!(tool_calls, vec!["Read"]);
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
    async fn streams_token_level_deltas_from_partial_events() {
        // With --include-partial-messages, claude emits each token as
        // a content_block_delta inside a stream_event envelope, then
        // sends the final assistant message with the assembled text.
        // We must emit on_delta per partial and suppress the final
        // assistant text (it would otherwise re-emit the whole turn).
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hel"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo "}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"world"}}}
{"type":"stream_event","event":{"type":"content_block_stop","index":0}}
{"type":"stream_event","event":{"type":"message_stop"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"hello world"},{"type":"tool_use","id":"call_1","name":"Read","input":{"path":"a.md"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"call_1","content":"ok"}]}}
{"type":"result","subtype":"success","result":"hello world","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
        assert_eq!(outcome.assistant_text, "hello world");
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
        // Three partial chunks, then NOT a fourth "hello world" from
        // the final assistant event.
        assert_eq!(deltas, vec!["hel", "lo ", "world"]);
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
        // Tool calls still arrive from the final assistant event so
        // the listener gets a complete `input` payload.
        assert_eq!(tool_calls, vec!["Read"]);
    }

    /// E2E against a real `claude` binary on the host. Skipped in CI
    /// (and by default locally) because it hits the live Anthropic
    /// API: `cargo test ... -- --ignored real_claude_streams_tokens`.
    /// Requires `claude` on PATH and a logged-in CLI. The cost is a
    /// few tokens (1 turn, max ~5 output tokens by the prompt).
    ///
    /// Purpose: lock in that the wire-format we decode
    /// (`stream_event` -> `content_block_delta` -> `text_delta`) is
    /// what claude actually emits, so a future upstream rename would
    /// fail here instead of silently dropping the UI typewriter
    /// effect. The exact delta count varies (claude's token boundaries
    /// shift run-to-run), so we assert on shape: multiple deltas
    /// arrived AND each individual delta is shorter than the full
    /// assistant text (proving it was streamed, not delivered as
    /// one chunk by the final `assistant` event).
    #[tokio::test]
    #[ignore]
    async fn real_claude_streams_tokens() {
        let tmp = TempDir::new().unwrap();
        let backend = ClaudeCliBackend::new(
            default_cmd(),
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
                &[Message::user(
                    "Reply with exactly the single word: streaming",
                )],
                &[],
                listener.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;

        let events = listener.0.lock().unwrap();
        let errors: Vec<&str> = events
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
            errors.is_empty(),
            "claude reported errors: {errors:?}; outcome={:?}",
            outcome.stop_reason
        );
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);
        assert!(
            !outcome.assistant_text.is_empty(),
            "assistant_text was empty; stream may not be reaching us"
        );

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
        assert!(
            deltas.len() >= 2,
            "expected token-level streaming (>=2 deltas), got {}: {deltas:?}",
            deltas.len()
        );
        // Each delta must be a strict prefix-piece of the total, not
        // the whole reply at once (which is what we'd see if partials
        // were disabled and the final assistant event fired on_delta).
        for d in &deltas {
            assert!(
                d.len() < outcome.assistant_text.len(),
                "single delta {d:?} matches the entire assistant_text \
                 ({:?}); partial streaming is not actually engaged",
                outcome.assistant_text,
            );
        }
        // Concatenated partials must equal the canonical assistant
        // text. If we ever double-counted (partials + final event),
        // this would be 2x. If we lost any, it'd be shorter.
        let joined: String = deltas.concat();
        assert_eq!(
            joined, outcome.assistant_text,
            "joined deltas != assistant_text; deltas={deltas:?}"
        );
    }

    #[tokio::test]
    async fn message_start_resets_per_block_partials_so_second_message_is_not_dropped() {
        // Regression for the bug captured in design.md section 6.2:
        // a per-run "streamed_partial_text" flag would suppress the
        // text of message N+1 if message N had streamed partials and
        // message N+1 had not. Per-message reset on `message_start`
        // is what this test pins.
        //
        // Fixture: message 1 streams its text via partials; message 2
        // ships only the final assistant event (no partials), with
        // text "from msg2". Both must appear in the deltas, neither
        // duplicated nor dropped.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"msg1 text"}}}
{"type":"assistant","message":{"content":[{"type":"text","text":"msg1 text"}]}}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m2"}}}
{"type":"assistant","message":{"content":[{"type":"text","text":"from msg2"}]}}
{"type":"result","subtype":"success","result":"from msg2","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
            vec!["msg1 text".to_string(), "from msg2".to_string()]
        );
        assert_eq!(outcome.assistant_text, "msg1 textfrom msg2");
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);
    }

    #[tokio::test]
    async fn message_start_for_next_message_before_assistant_for_current_does_not_double_emit() {
        // Regression for the user-visible duplication on claude_cli:
        // "I see the message, pause, same message again, rest of follow-
        //  up buffers". Hypothesis: the partial-tracker is keyed only
        // by content-block index and reset on any `message_start`, so
        // if claude-cli flushes the SDK's message_start for message
        // N+1 BEFORE it synthesizes the final `assistant` envelope for
        // message N, the tracker for block 0 has already been cleared
        // by the time message N's final event lands. The Assistant
        // arm then sees `already=""`, treats canonical text as un-
        // streamed, and re-emits the full block via on_delta.
        //
        // Fixture orders events so msg_start(m2) lands between the
        // partials of m1 and the assistant event for m1. A correct
        // implementation MUST still emit each message's text exactly
        // once.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"msg1 text"}}}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m2"}}}
{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"msg1 text"}]}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"msg2 text"}}}
{"type":"assistant","message":{"id":"m2","content":[{"type":"text","text":"msg2 text"}]}}
{"type":"result","subtype":"success","result":"msg2 text","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
            vec!["msg1 text".to_string(), "msg2 text".to_string()],
            "msg1 text was emitted twice: once via partials, once via the \
             final assistant event because msg2's message_start cleared \
             the tracker before msg1's assistant arrived"
        );
        // Joined deltas must equal assistant_text exactly. If we
        // double-counted, this would be 2x msg1 text.
        let joined: String = deltas.concat();
        assert_eq!(joined, outcome.assistant_text);
    }

    #[tokio::test]
    async fn second_text_block_without_partials_in_same_message_is_emitted() {
        // The partial tracker is keyed by content-block index inside
        // a single assistant message. If claude streams partials for
        // index 0 and ships index 1 only in the final assistant
        // event (no partials), index 1's text must still reach the
        // listener. The previous global-bool model dropped this case
        // silently.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"streamed"}}}
{"type":"assistant","message":{"content":[{"type":"text","text":"streamed"},{"type":"text","text":" buffered"}]}}
{"type":"result","subtype":"success","result":"streamed buffered","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
            vec!["streamed".to_string(), " buffered".to_string()]
        );
        assert_eq!(outcome.assistant_text, "streamed buffered");
    }

    #[tokio::test]
    async fn partials_canonical_drift_drops_canonical_no_doubling() {
        // Regression for the doubling bug: when claude's partial
        // text_delta events don't form an exact prefix of the
        // canonical content block (whitespace / encoding drift
        // between the two streams), the old code re-emitted the
        // full canonical text, doubling both on_delta and
        // assistant_text. The fix keeps the partials' view and
        // drops the canonical block.
        //
        // Fixture: partials stream "weekly is gone." but the
        // canonical block reports "weekly is gone" (no trailing
        // period). The two don't prefix-match; we expect a single
        // delta with the partials' text and assistant_text equal
        // to it.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"weekly is gone."}}}
{"type":"assistant","message":{"content":[{"type":"text","text":"weekly is gone"}]}}
{"type":"result","subtype":"success","result":"weekly is gone","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
        assert_eq!(deltas, vec!["weekly is gone.".to_string()]);
        assert_eq!(outcome.assistant_text, "weekly is gone.");
    }

    #[tokio::test]
    async fn stream_ended_without_result_event_is_error() {
        // A clean EOF on stdout with no `result` event means the
        // turn is incomplete (e.g. claude was killed mid-reply by
        // the OS, or the protocol drifted). Surfacing this as
        // EndOfTurn would lie to the UI; the listener should see
        // on_error and the orchestrator should see Outcome::error.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"partial"}]}}"#;
        let script = fake_claude(tmp.path(), body);
        let backend = ClaudeCliBackend::new(
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
            "expected stream-ended error, got: {errs:?}"
        );
    }

    #[tokio::test]
    async fn parse_error_emissions_are_rate_limited() {
        // PARSE_ERROR_EMIT_LIMIT distinct error frames, then the
        // overflow is counted silently and summarised once at the
        // end. Without this cap a malformed-line flood could fan
        // out into thousands of WebSocket frames.
        let tmp = TempDir::new().unwrap();
        // 12 bad JSON lines, then a valid result so the stream ends
        // cleanly without tripping the no-result error path.
        let mut body = String::new();
        for i in 0..12 {
            body.push_str(&format!("not json line {i}\n"));
        }
        body.push_str(r#"{"type":"result","subtype":"success","result":"","is_error":false}"#);
        let script = fake_claude(tmp.path(), &body);
        let backend = ClaudeCliBackend::new(
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
        // 5 detailed parse errors + 1 summary line = 6 frames.
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
        // A single very large stdout line must not be buffered past
        // NDJSON_LINE_CAP_BYTES. The cap fires during the read; the
        // backend surfaces an error and returns Outcome::error.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("flood.sh");
        // 5 MiB of 'x' on one line; the cap is 4 MiB.
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
        let backend = ClaudeCliBackend::new(
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
        // A child that never writes to stdout would wedge the loop
        // forever under the previous (uncapped) reader. With the
        // timeout in place the loop kills the child and surfaces an
        // error after the configured duration.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("silent.sh");
        std::fs::write(&path, "#!/bin/sh\nsleep 30\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let backend = ClaudeCliBackend::new(
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
        let backend = ClaudeCliBackend::new(
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
        // Either the wait() path surfaces "exit ... boom" (the
        // happy case where stdin write beats the child to the
        // exit), or the stdin write loses the race against the
        // child exiting and we surface "stdin: Broken pipe"
        // first. Both are valid "non-zero exit produced an error
        // event" outcomes. Linux schedules the latter reliably
        // under load.
        assert!(
            errs.iter()
                .any(|e| (e.contains("exit") && e.contains("boom")) || e.contains("stdin")),
            "errs={errs:?}"
        );
    }

    #[test]
    fn split_renders_roles_with_labels() {
        let msgs = vec![
            Message::system("be helpful"),
            Message::user("first"),
            Message::assistant("ok"),
            Message::user("second"),
        ];
        let (sys, body) = split_system_and_prompt(&msgs);
        assert_eq!(sys.as_deref(), Some("be helpful"));
        assert!(body.contains("[user]\nfirst"));
        assert!(body.contains("[assistant]\nok"));
        assert!(body.contains("[user]\nsecond"));
    }

    #[test]
    fn mcp_config_forwards_command_verbatim() {
        // The MCP server owns the auto-apply gate; the proxy / MCP
        // command line is forwarded as-is. The v2 chan-server proxy
        // subcommand (`chan __mcp-proxy <socket>`) doesn't accept a
        // `--auto-apply` flag and would clap-error if we appended one,
        // so it's important that we don't.
        let f = write_mcp_config(&McpWiring {
            command: vec!["chan".into(), "__mcp-proxy".into(), "/tmp/s".into()],
        })
        .unwrap();
        let body = std::fs::read_to_string(f.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let entry = &v["mcpServers"]["chan"];
        assert_eq!(entry["command"], "chan");
        let args = entry["args"].as_array().unwrap();
        assert_eq!(
            args.iter().map(|a| a.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["__mcp-proxy", "/tmp/s"]
        );
    }

    /// Generic shell-script writer: caller supplies the full
    /// `#!/bin/sh ...` body, so tests that need `sleep` between
    /// heredocs (heartbeat) or a non-zero `exit` (crash) can spell
    /// the script directly. The simple all-at-once cases keep using
    /// `fake_claude(...)` above.
    fn fake_claude_raw(dir: &std::path::Path, script: &str) -> PathBuf {
        let path = dir.join("fake_claude.sh");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn statuses(events: &[Event]) -> Vec<&AgentStatus> {
        events
            .iter()
            .filter_map(|e| {
                if let Event::Status(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect()
    }

    fn activities(events: &[Event]) -> Vec<&AgentActivity> {
        events
            .iter()
            .filter_map(|e| {
                if let Event::Activity(a) = e {
                    Some(a)
                } else {
                    None
                }
            })
            .collect()
    }

    fn user_requests(events: &[Event]) -> Vec<&UserRequest> {
        events
            .iter()
            .filter_map(|e| {
                if let Event::UserRequest(r) = e {
                    Some(r)
                } else {
                    None
                }
            })
            .collect()
    }

    async fn run_fake(
        script_path: PathBuf,
        cwd: PathBuf,
        inactivity: Duration,
    ) -> (super::Outcome, Vec<Event>) {
        let backend = ClaudeCliBackend::new(
            vec![script_path.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            cwd,
            None,
            inactivity,
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
        let events = listener.0.lock().unwrap().clone();
        (outcome, events)
    }

    #[tokio::test]
    async fn system_init_emits_ready_and_session_started() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init","session_id":"sid-1","model":"claude-opus-4-7","claude_code_version":"2.1.142"}
{"type":"result","subtype":"success","result":"","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let saw_ready = statuses(&events).iter().any(|s| {
            matches!(
                s,
                AgentStatus::Ready { session_id, model, version, .. }
                    if session_id.as_deref() == Some("sid-1")
                        && model.as_deref() == Some("claude-opus-4-7")
                        && version.as_deref() == Some("2.1.142")
            )
        });
        assert!(saw_ready, "missing Ready from system/init: {events:#?}");
        let saw_session_started = activities(&events).iter().any(|a| {
            matches!(
                a,
                AgentActivity::SessionStarted { session_id, .. }
                    if session_id.as_deref() == Some("sid-1")
            )
        });
        assert!(saw_session_started, "missing SessionStarted: {events:#?}");
    }

    #[tokio::test]
    async fn system_status_requesting_emits_thinking() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"system","subtype":"status","status":"requesting"}
{"type":"result","subtype":"success","result":"","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let saw_thinking = statuses(&events).iter().any(|s| {
            matches!(
                s,
                AgentStatus::Thinking { status, .. } if status.as_deref() == Some("requesting")
            )
        });
        assert!(saw_thinking, "missing Thinking status: {events:#?}");
    }

    #[tokio::test]
    async fn heartbeat_fires_while_child_is_quiet_but_alive() {
        // 2s inactivity ceiling, so heartbeat_interval clamps to 1s.
        // The fake child writes init, sleeps 1.5s while still alive,
        // then writes the terminal result. At least one heartbeat
        // must fire during the sleep.
        let tmp = TempDir::new().unwrap();
        let script = "#!/bin/sh
cat <<'EOF1'
{\"type\":\"system\",\"subtype\":\"init\"}
EOF1
sleep 1.5
cat <<'EOF2'
{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"\",\"is_error\":false}
EOF2
";
        let path = fake_claude_raw(tmp.path(), script);
        let (_outcome, events) =
            run_fake(path, tmp.path().to_path_buf(), Duration::from_secs(2)).await;
        let heartbeats = statuses(&events)
            .into_iter()
            .filter(|s| matches!(s, AgentStatus::Heartbeat { .. }))
            .count();
        assert!(
            heartbeats >= 1,
            "expected at least one heartbeat during 1.5s quiet window: {events:#?}"
        );
    }

    #[tokio::test]
    async fn tool_started_fires_before_canonical_tool_call() {
        // content_block_start { tool_use } must produce
        // AgentActivity::ToolStarted ahead of the final assistant
        // envelope's on_tool_call so the frontend can light up an
        // activity row before the tool args finish streaming.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}
{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call_1","name":"Read","input":{}}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"a"}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":".md\"}"}}}
{"type":"stream_event","event":{"type":"content_block_stop","index":0}}
{"type":"stream_event","event":{"type":"message_stop"}}
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"call_1","name":"Read","input":{"path":"a.md"}}]}}
{"type":"result","subtype":"success","result":"","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let tool_started_pos = events.iter().position(|e| {
            matches!(
                e,
                Event::Activity(AgentActivity::ToolStarted { id, name, .. })
                    if id == "call_1" && name == "Read"
            )
        });
        let tool_call_pos = events
            .iter()
            .position(|e| matches!(e, Event::ToolCall(n) if n == "Read"));
        let (sp, cp) = (
            tool_started_pos.expect("ToolStarted missing"),
            tool_call_pos.expect("on_tool_call missing"),
        );
        assert!(
            sp < cp,
            "ToolStarted ({sp}) must precede on_tool_call ({cp}): {events:#?}"
        );
        // And the streamed args must carry the tool id we tracked
        // from content_block_start.
        let args_with_id = activities(&events)
            .into_iter()
            .filter(
                |a| matches!(a, AgentActivity::ToolArgsDelta { id: Some(s), .. } if s == "call_1"),
            )
            .count();
        assert_eq!(
            args_with_id, 2,
            "both InputJsonDelta frames should carry tool id: {events:#?}"
        );
    }

    #[tokio::test]
    async fn ask_user_question_emits_typed_survey() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"AskUserQuestion","input":{"questions":[{"question":"Which color?","header":"Color","multiSelect":false,"options":[{"label":"Red","description":"warm"},{"label":"Blue","description":"cool"}]}]}}]}}
{"type":"result","subtype":"success","result":"","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let surveys = user_requests(&events);
        assert_eq!(surveys.len(), 1, "expected exactly one Survey: {events:#?}");
        match surveys[0] {
            UserRequest::Survey { id, questions, .. } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(questions.len(), 1);
                assert_eq!(questions[0].question, "Which color?");
                assert_eq!(questions[0].header.as_deref(), Some("Color"));
                assert!(!questions[0].multi_select);
                assert_eq!(questions[0].options.len(), 2);
                assert_eq!(questions[0].options[0].label, "Red");
                assert_eq!(questions[0].options[0].description.as_deref(), Some("warm"));
            }
        }
        // The existing on_tool_call compat path still fires.
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
        assert_eq!(tool_calls, vec!["AskUserQuestion"]);
    }

    #[tokio::test]
    async fn numbered_choice_in_plain_text_stays_a_delta() {
        // No AskUserQuestion. The model writes "Pick: 1. apples / 2.
        // oranges / 3. pears" as plain text. chan-llm must not parse
        // this into a typed UserRequest; the frontend keeps the
        // "reply with 1/2/3" affordance as a heuristic only.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"Pick one: 1. apples 2. oranges 3. pears"}]}}
{"type":"result","subtype":"success","result":"Pick one: 1. apples 2. oranges 3. pears","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        assert!(
            user_requests(&events).is_empty(),
            "numbered text must not synthesize a UserRequest: {events:#?}"
        );
        assert!(
            outcome.assistant_text.contains("1. apples"),
            "transcript should preserve numbered text"
        );
    }

    #[tokio::test]
    async fn inactivity_timeout_emits_unhealthy_status() {
        // 1s inactivity ceiling. Child writes one line and then
        // sleeps far past the timeout. The runner kills the child and
        // surfaces an AgentStatus::Unhealthy with the duration in
        // the reason.
        let tmp = TempDir::new().unwrap();
        let script = "#!/bin/sh
cat <<'EOF'
{\"type\":\"system\",\"subtype\":\"init\"}
EOF
sleep 30
";
        let path = fake_claude_raw(tmp.path(), script);
        let (outcome, events) =
            run_fake(path, tmp.path().to_path_buf(), Duration::from_secs(1)).await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let saw_unhealthy = statuses(&events).iter().any(|s| {
            matches!(
                s,
                AgentStatus::Unhealthy { reason, .. } if reason.starts_with("no_output_for_")
            )
        });
        assert!(
            saw_unhealthy,
            "missing inactivity-timeout Unhealthy: {events:#?}"
        );
    }

    #[tokio::test]
    async fn nonzero_exit_emits_unhealthy_status() {
        // Child writes init and then exits non-zero before the
        // terminal result. The exit-status branch emits an Unhealthy
        // whose reason carries the status string.
        let tmp = TempDir::new().unwrap();
        let script = "#!/bin/sh
cat <<'EOF'
{\"type\":\"system\",\"subtype\":\"init\"}
EOF
echo 'boom' 1>&2
exit 2
";
        let path = fake_claude_raw(tmp.path(), script);
        let (outcome, events) =
            run_fake(path, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let saw_unhealthy = statuses(&events).iter().any(|s| {
            matches!(
                s,
                AgentStatus::Unhealthy { reason, .. } if reason.starts_with("exit:")
            )
        });
        assert!(saw_unhealthy, "missing nonzero-exit Unhealthy: {events:#?}");
    }

    #[tokio::test]
    async fn missing_terminal_result_emits_unhealthy_status() {
        // Child writes init plus an assistant message but no
        // `result` envelope. The no-terminal branch must mark the
        // run Unhealthy("no_terminal") so the host renders an
        // actionable error rather than presenting a half reply.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]}}"#;
        let script = fake_claude(tmp.path(), body);
        let (outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        assert_eq!(outcome.stop_reason, StopReason::Error);
        let saw_unhealthy = statuses(&events)
            .iter()
            .any(|s| matches!(s, AgentStatus::Unhealthy { reason, .. } if reason == "no_terminal"));
        assert!(saw_unhealthy, "missing no-terminal Unhealthy: {events:#?}");
    }

    #[tokio::test]
    async fn parent_tool_use_id_propagates_into_activity() {
        // Sub-agent output: the outer Assistant frame carries
        // parent_tool_use_id, and that id must appear on the
        // ToolFinished frame so the frontend can nest the sub-agent
        // activity under its parent Task tool call.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"assistant","parent_tool_use_id":"parent_task_1","message":{"content":[{"type":"tool_use","id":"call_1","name":"Read","input":{"path":"x.md"}}]}}
{"type":"user","parent_tool_use_id":"parent_task_1","message":{"content":[{"type":"tool_result","tool_use_id":"call_1","content":"ok"}]}}
{"type":"result","subtype":"success","result":"","is_error":false}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let nested = activities(&events)
            .into_iter()
            .filter(|a| match a {
                AgentActivity::ToolFinished { parent_id, .. } => {
                    parent_id.as_deref() == Some("parent_task_1")
                }
                _ => false,
            })
            .count();
        assert!(
            nested >= 2,
            "ToolFinished frames must inherit parent_tool_use_id: {events:#?}"
        );
    }

    #[tokio::test]
    async fn permission_denials_emit_tool_denied_activity() {
        // The terminal `result.permission_denials[]` carries every
        // tool call claude refused (or that the host cancelled before
        // answering). Each entry maps to one AgentActivity::ToolDenied
        // so the frontend can attribute the denial to the original
        // tool_use.
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"system","subtype":"init"}
{"type":"result","subtype":"success","result":"","is_error":false,"permission_denials":[{"tool_name":"Write","tool_use_id":"toolu_w","tool_input":{"path":"secret.txt","content":"x"}},{"tool_name":"Bash","tool_use_id":"toolu_b","tool_input":{"command":"rm -rf /"}}]}"#;
        let script = fake_claude(tmp.path(), body);
        let (_outcome, events) =
            run_fake(script, tmp.path().to_path_buf(), Duration::from_secs(60)).await;
        let denied: Vec<(&str, &str)> = activities(&events)
            .into_iter()
            .filter_map(|a| match a {
                AgentActivity::ToolDenied {
                    name: Some(n),
                    id: Some(i),
                    ..
                } => Some((i.as_str(), n.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            denied,
            vec![("toolu_w", "Write"), ("toolu_b", "Bash")],
            "denials must be surfaced in order with their names/ids: {events:#?}"
        );
    }

    #[test]
    fn allowed_tools_covers_every_mcp_tool() {
        // Each `mcp__chan__*` entry maps to a real tool on the
        // chan-llm MCP server. If a new tool lands in mcp.rs without
        // being added here, claude prompts for permission on first
        // use and the v2 black-box experience breaks. Pin every
        // current tool by name so a drift surfaces as a test
        // failure at the right place.
        for tool in [
            "mcp__chan__read_file",
            "mcp__chan__write_file",
            "mcp__chan__list_files",
            "mcp__chan__search_content",
            "mcp__chan__read_image",
            "mcp__chan__graph_neighbors",
            "mcp__chan__graph_tags",
            "mcp__chan__graph_files_with_tag",
            "mcp__chan__repo_report",
        ] {
            assert!(
                ALLOWED_TOOLS.contains(tool),
                "ALLOWED_TOOLS missing {tool}: {ALLOWED_TOOLS}"
            );
        }
    }
}
