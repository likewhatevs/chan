//! ClaudeCli backend: shell-executor wrapper around the `claude` CLI.
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
//! The MCP subprocess receives `--auto-apply` only when the user
//! enabled it; otherwise `write_file` returns a "deferred" error
//! to claude (the host-approval side channel for resuming claude
//! mid-call is tracked in chan-llm issue #1).
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
//!     - `{"type":"assistant","message":{...}}` (text + tools)
//!     - `{"type":"user","message":{...}}` (tool results)
//!     - `{"type":"result","subtype":"...",...}` (end of turn)
//!
//!     Inside an assistant message, content blocks are either
//!     `{"type":"text","text":"..."}` (emit on_delta) or
//!     `{"type":"tool_use","id":"...","name":"...","input":...}`
//!     (forward to listener via on_tool_call for visibility; not
//!     executed by chan-llm). Tool results inside user messages
//!     are forwarded as on_tool_result. Both are observational in
//!     v1; the orchestration loop treats `Outcome.tool_calls` as
//!     empty so it exits after one backend turn.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value as Json;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall, ToolResult};
use crate::tools::ToolSchema;

use super::{Backend, Outcome};

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
/// and the drive path); `auto_apply_writes` mirrors `LlmConfig`'s
/// flag and decides whether the MCP subprocess is launched with
/// `--auto-apply`.
#[derive(Debug, Clone)]
pub struct McpWiring {
    pub command: Vec<String>,
    pub auto_apply_writes: bool,
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
    "mcp__chan__search_content",
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
}

impl ClaudeCliBackend {
    pub fn new(
        cmd: Vec<String>,
        extra_args: Vec<String>,
        model: Option<String>,
        cwd: PathBuf,
        mcp: Option<McpWiring>,
    ) -> Self {
        Self {
            cmd,
            extra_args,
            model,
            cwd,
            mcp,
        }
    }
}

#[async_trait]
impl Backend for ClaudeCliBackend {
    async fn run(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let (system, prompt) = split_system_and_prompt(&messages);

        let Some((bin, leading)) = self.cmd.split_first() else {
            listener.on_error("claude_cli: empty cmd".into());
            return Outcome::error();
        };

        let mut command = Command::new(bin);
        command
            .args(leading)
            .arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("text")
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

        if let Some(sys) = system.as_deref() {
            command.arg("--append-system-prompt").arg(sys);
        }
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

        // Pipe the rendered transcript on stdin. Drop stdin so
        // claude sees EOF and processes the prompt.
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                listener.on_error(format!("claude_cli stdin: {e}"));
                let _ = child.kill().await;
                return Outcome::error();
            }
            drop(stdin);
        }

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                listener.on_error("claude_cli: stdout not piped".into());
                let _ = child.kill().await;
                return Outcome::error();
            }
        };
        let stderr = child.stderr.take();

        let mut reader = BufReader::new(stdout).lines();
        let mut assistant_text = String::new();
        let mut stop = StopReason::EndOfTurn;
        let mut saw_result = false;

        loop {
            if cancel.load(Ordering::Relaxed) {
                // Kill the subprocess and return what we have. The
                // assistant_text we collected so far stays on the
                // outcome so the host can keep partial UI state.
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Outcome::cancelled(assistant_text);
            }
            let line = match reader.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => break,
                Err(e) => {
                    listener.on_error(format!("claude_cli stdout: {e}"));
                    let _ = child.kill().await;
                    return Outcome::error();
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let event: StreamEvent = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(e) => {
                    // Don't fail the whole turn on a single
                    // unrecognized line; claude may add new event
                    // shapes in future versions. Surface once and
                    // keep reading.
                    listener.on_error(format!(
                        "claude_cli parse: {e}; raw: {}",
                        truncate(&line, 400)
                    ));
                    continue;
                }
            };
            match event {
                StreamEvent::Assistant { message } => {
                    for block in message.content {
                        match block {
                            ContentBlock::Text { text } => {
                                if !text.is_empty() {
                                    listener.on_delta(Delta { text: text.clone() });
                                    assistant_text.push_str(&text);
                                }
                            }
                            ContentBlock::ToolUse { id, name, input } => {
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
                StreamEvent::User { message } => {
                    for block in message.content {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } = block
                        {
                            listener.on_tool_result(ToolResult {
                                id: tool_use_id,
                                output: content,
                            });
                        }
                    }
                }
                StreamEvent::Result {
                    subtype,
                    result,
                    is_error,
                } => {
                    saw_result = true;
                    if is_error.unwrap_or(false) {
                        let msg = result
                            .clone()
                            .unwrap_or_else(|| format!("claude exit ({subtype})"));
                        listener.on_error(format!("claude_cli result: {msg}"));
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
                                assistant_text.push_str(&text);
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
                listener.on_error(format!("claude_cli wait: {e}"));
                return Outcome::error();
            }
        };
        if !status.success() {
            let stderr_text = match stderr {
                Some(s) => read_to_string_async(s).await.unwrap_or_default(),
                None => String::new(),
            };
            let snippet = truncate(&stderr_text, 800);
            listener.on_error(format!("claude_cli exit {status}: {snippet}"));
            return Outcome::error();
        }
        if !saw_result && stop != StopReason::Error {
            // claude ended cleanly but never emitted a `result`
            // event. Treat as end_of_turn but flag the anomaly so
            // we notice if the protocol shifts.
            listener.on_error("claude_cli: stream ended without a result event".into());
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
/// binary) and the rest (its args); we tack on `--auto-apply` only
/// when the user opted in, so the MCP subprocess defers writes
/// (returns a "deferred" tool error to claude) when auto-apply is
/// off. The returned `NamedTempFile` must outlive the claude
/// subprocess so the path remains valid.
fn write_mcp_config(wiring: &McpWiring) -> std::io::Result<NamedTempFile> {
    let (bin, base_args) = wiring.command.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty mcp_command")
    })?;
    let mut args: Vec<String> = base_args.to_vec();
    if wiring.auto_apply_writes {
        args.push("--auto-apply".to_string());
    }
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

async fn read_to_string_async(mut s: tokio::process::ChildStderr) -> std::io::Result<String> {
    use tokio::io::AsyncReadExt;
    let mut out = String::new();
    s.read_to_string(&mut out).await?;
    Ok(out)
}

// ---- stream-json wire types --------------------------------------------

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    Assistant {
        message: AssistantMessage,
    },
    User {
        message: UserMessage,
    },
    Result {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
    },
    /// system / partial / future event types pass through silently.
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
    enum Event {
        Delta(String),
        ToolCall(String),
        ToolResult(String),
        Error(String),
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
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                vec![Message::user("hi")],
                Vec::new(),
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
        );
        let listener = Arc::new(Collector(Mutex::new(Vec::new())));
        let outcome = backend
            .run(
                vec![Message::user("hi")],
                Vec::new(),
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
    fn mcp_config_omits_auto_apply_when_off() {
        let f = write_mcp_config(&McpWiring {
            command: vec!["chan".into(), "__mcp".into(), "/d".into()],
            auto_apply_writes: false,
        })
        .unwrap();
        let body = std::fs::read_to_string(f.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let entry = &v["mcpServers"]["chan"];
        assert_eq!(entry["command"], "chan");
        let args = entry["args"].as_array().unwrap();
        assert_eq!(
            args.iter().map(|a| a.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["__mcp", "/d"]
        );
    }

    #[test]
    fn mcp_config_appends_auto_apply_when_on() {
        let f = write_mcp_config(&McpWiring {
            command: vec!["chan".into(), "__mcp".into(), "/d".into()],
            auto_apply_writes: true,
        })
        .unwrap();
        let body = std::fs::read_to_string(f.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let args = v["mcpServers"]["chan"]["args"].as_array().unwrap();
        assert_eq!(
            args.iter().map(|a| a.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["__mcp", "/d", "--auto-apply"]
        );
    }
}
