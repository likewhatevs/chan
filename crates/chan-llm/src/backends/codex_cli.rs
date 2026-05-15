//! CodexCli backend: shell-executor wrapper around `codex exec`.
//!
//! Codex is closest to the existing GeminiCli shape: `codex exec
//! --json` runs a full agent loop and emits JSONL events on stdout.
//! The backend is therefore observational for tool calls/results:
//! they are forwarded to `SessionListener` for UI visibility, but
//! `Outcome.tool_calls` stays empty because Codex already executed
//! its own loop.
//!
//! MCP-mediated mode is injected with per-invocation `-c` overrides
//! for `mcp_servers.chan.*`. That avoids editing the user's real
//! `~/.codex/config.toml` and, unlike redirecting `CODEX_HOME`,
//! preserves the user's existing Codex auth store.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value as Json;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::session::{Delta, Message, Role, SessionListener, StopReason, ToolCall, ToolResult};
use crate::tools::ToolSchema;

use super::{
    read_line_capped, sanitize_env, spawn_stderr_drainer, Backend, Outcome, StderrDrainer,
    NDJSON_LINE_CAP_BYTES, PARSE_ERROR_EMIT_LIMIT,
};

/// Default command to launch Codex. Plain `codex` so PATH wins.
pub fn default_cmd() -> Vec<String> {
    vec!["codex".to_string()]
}

/// Host-supplied wiring that switches the backend into MCP-mediated
/// mode. `command` is the full argv used to spawn the chan MCP server.
#[derive(Debug, Clone)]
pub struct McpWiring {
    pub command: Vec<String>,
}

const MCP_SERVER_KEY: &str = "chan";

#[derive(Debug)]
pub struct CodexCliBackend {
    cmd: Vec<String>,
    extra_args: Vec<String>,
    model: Option<String>,
    cwd: PathBuf,
    mcp: Option<McpWiring>,
    inactivity_timeout: Duration,
}

impl CodexCliBackend {
    pub fn new(
        cmd: Vec<String>,
        extra_args: Vec<String>,
        model: Option<String>,
        cwd: PathBuf,
        mcp: Option<McpWiring>,
        inactivity_timeout: Duration,
    ) -> Self {
        Self {
            cmd,
            extra_args,
            model,
            cwd,
            mcp,
            inactivity_timeout,
        }
    }
}

#[async_trait]
impl Backend for CodexCliBackend {
    async fn run(
        &self,
        messages: &[Message],
        _tools: &[ToolSchema],
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let prompt = render_prompt(messages);

        let Some((bin, leading)) = self.cmd.split_first() else {
            listener.on_error("codex_cli: empty cmd".into());
            return Outcome::error();
        };

        let mut command = Command::new(bin);
        // Codex may authenticate from OPENAI_API_KEY or from its own
        // CODEX_HOME/auth store. HOME is part of the base allowlist,
        // and CODEX_/OPENAI_ are the only vendor prefixes forwarded.
        sanitize_env(&mut command, &["CODEX_", "OPENAI_"]);
        command.kill_on_drop(true);
        command
            .args(leading)
            .arg("exec")
            .arg("--json")
            .arg("--ephemeral")
            .arg("--skip-git-repo-check")
            .arg("--sandbox")
            .arg(if self.mcp.is_some() {
                // In mediated mode, keep Codex's native shell/file
                // mutations read-only so writes are routed through
                // chan's MCP write_file tool and host approval path.
                "read-only"
            } else {
                "workspace-write"
            })
            .arg("--cd")
            .arg(&self.cwd)
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(wiring) = self.mcp.as_ref() {
            if let Err(e) = append_mcp_overrides(&mut command, wiring) {
                listener.on_error(format!("codex_cli mcp-config: {e}"));
                return Outcome::error();
            }
        }
        if let Some(model) = self.model.as_deref() {
            command.arg("--model").arg(model);
        }
        for arg in &self.extra_args {
            command.arg(arg);
        }
        command.arg("-");

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                listener.on_error(format!("codex_cli spawn: {e}"));
                return Outcome::error();
            }
        };

        let mut stderr_drainer: Option<StderrDrainer> = spawn_stderr_drainer(child.stderr.take());
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                let stderr_snippet = match stderr_drainer.take() {
                    Some(d) => truncate(&d.finish().await, 800),
                    None => String::new(),
                };
                let _ = child.kill().await;
                let _ = child.wait().await;
                if stderr_snippet.is_empty() {
                    listener.on_error(format!("codex_cli stdin: {e}"));
                } else {
                    listener.on_error(format!("codex_cli stdin: {e}; stderr: {stderr_snippet}"));
                }
                return Outcome::error();
            }
            drop(stdin);
        }

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_error("codex_cli: stdout not piped".into());
                let _ = child.kill().await;
                return Outcome::error();
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut line_buf: Vec<u8> = Vec::new();
        let mut assistant_text = String::new();
        let mut streamed_by_item: HashMap<String, String> = HashMap::new();
        let mut stop = StopReason::EndOfTurn;
        let mut saw_terminal = false;
        let mut parse_errors_emitted = 0usize;
        let mut parse_errors_silenced = 0usize;

        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill().await;
                let _ = child.wait().await;
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                return Outcome::cancelled(assistant_text);
            }

            let read = timeout(
                self.inactivity_timeout,
                read_line_capped(&mut reader, &mut line_buf, NDJSON_LINE_CAP_BYTES),
            )
            .await;
            let got_line = match read {
                Ok(Ok(true)) => true,
                Ok(Ok(false)) => break,
                Ok(Err(e)) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    let stderr_snippet = match stderr_drainer.take() {
                        Some(d) => truncate(&d.finish().await, 800),
                        None => String::new(),
                    };
                    if stderr_snippet.is_empty() {
                        listener.on_error(format!("codex_cli stdout: {e}"));
                    } else {
                        listener
                            .on_error(format!("codex_cli stdout: {e}; stderr: {stderr_snippet}"));
                    }
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
                        "codex_cli: no output for {}s; subprocess wedged",
                        self.inactivity_timeout.as_secs(),
                    );
                    if stderr_snippet.is_empty() {
                        listener.on_error(base);
                    } else {
                        listener.on_error(format!("{base}; stderr: {stderr_snippet}"));
                    }
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
                        listener.on_error("codex_cli stdout: non-utf8 line".into());
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

            let event: Json = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(e) => {
                    if parse_errors_emitted < PARSE_ERROR_EMIT_LIMIT {
                        listener.on_error(format!(
                            "codex_cli parse: {e}; raw: {}",
                            truncate(line, 400)
                        ));
                        parse_errors_emitted += 1;
                    } else {
                        parse_errors_silenced += 1;
                    }
                    continue;
                }
            };

            let normalized = normalize_event(&event);
            match normalized {
                NormalizedEvent::AgentDelta { item_id, delta } => {
                    if !delta.is_empty() {
                        listener.on_delta(Delta {
                            text: delta.clone(),
                        });
                        assistant_text.push_str(&delta);
                        if let Some(id) = item_id {
                            streamed_by_item.entry(id).or_default().push_str(&delta);
                        }
                        if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                            listener.on_error(format!(
                                "codex_cli stream: assistant text exceeded {} bytes; aborting",
                                super::ASSISTANT_TEXT_CAP_BYTES,
                            ));
                            let _ = child.kill().await;
                            return Outcome::error();
                        }
                    }
                }
                NormalizedEvent::AgentCompleted { item_id, text } => {
                    let already = item_id
                        .as_ref()
                        .and_then(|id| streamed_by_item.remove(id))
                        .unwrap_or_default();
                    let emit = if already.is_empty() {
                        Some(text.as_str())
                    } else {
                        text.strip_prefix(already.as_str())
                            .filter(|s| !s.is_empty())
                    };
                    if let Some(s) = emit {
                        listener.on_delta(Delta {
                            text: s.to_string(),
                        });
                        assistant_text.push_str(s);
                        if assistant_text.len() > super::ASSISTANT_TEXT_CAP_BYTES {
                            listener.on_error(format!(
                                "codex_cli stream: assistant text exceeded {} bytes; aborting",
                                super::ASSISTANT_TEXT_CAP_BYTES,
                            ));
                            let _ = child.kill().await;
                            return Outcome::error();
                        }
                    }
                }
                NormalizedEvent::McpStarted { id, name, args } => {
                    listener.on_tool_call(ToolCall { id, name, args });
                }
                NormalizedEvent::McpCompleted { id, output } => {
                    listener.on_tool_result(ToolResult { id, output });
                }
                NormalizedEvent::Error(message) => {
                    listener.on_error(format!("codex_cli: {message}"));
                    stop = StopReason::Error;
                }
                NormalizedEvent::Terminal { ok } => {
                    saw_terminal = true;
                    if !ok {
                        stop = StopReason::Error;
                    }
                }
                NormalizedEvent::Other => {}
            }
        }

        let status = match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                if let Some(d) = stderr_drainer.take() {
                    let _ = d.finish().await;
                }
                listener.on_error(format!("codex_cli wait: {e}"));
                return Outcome::error();
            }
        };
        if !status.success() {
            let stderr_text = match stderr_drainer.take() {
                Some(d) => d.finish().await,
                None => String::new(),
            };
            let snippet = truncate(&stderr_text, 800);
            listener.on_error(format!("codex_cli exit {status}: {snippet}"));
            return Outcome::error();
        }
        drop(stderr_drainer);
        if parse_errors_silenced > 0 {
            listener.on_error(format!(
                "codex_cli parse: {parse_errors_silenced} additional parse errors suppressed",
            ));
        }
        if !saw_terminal && stop != StopReason::Error {
            listener.on_error("codex_cli: stream ended without a terminal event".into());
            stop = StopReason::Error;
        }

        Outcome {
            assistant_text,
            tool_calls: Vec::new(),
            stop_reason: stop,
        }
    }
}

fn append_mcp_overrides(command: &mut Command, wiring: &McpWiring) -> std::io::Result<()> {
    let (bin, base_args) = wiring.command.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty mcp_command")
    })?;
    command
        .arg("-c")
        .arg(format!(
            "mcp_servers.{MCP_SERVER_KEY}.command={}",
            toml_string(bin)?
        ))
        .arg("-c")
        .arg(format!(
            "mcp_servers.{MCP_SERVER_KEY}.args={}",
            toml_array(base_args)?
        ))
        .arg("-c")
        .arg(format!("mcp_servers.{MCP_SERVER_KEY}.enabled=true"))
        .arg("-c")
        .arg(format!("mcp_servers.{MCP_SERVER_KEY}.required=true"));
    Ok(())
}

fn toml_string(value: &str) -> std::io::Result<String> {
    Ok(toml::Value::String(value.to_string()).to_string())
}

fn toml_array(values: &[String]) -> std::io::Result<String> {
    Ok(toml::Value::Array(
        values
            .iter()
            .map(|value| toml::Value::String(value.clone()))
            .collect(),
    )
    .to_string())
}

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

enum NormalizedEvent {
    AgentDelta {
        item_id: Option<String>,
        delta: String,
    },
    AgentCompleted {
        item_id: Option<String>,
        text: String,
    },
    McpStarted {
        id: String,
        name: String,
        args: Json,
    },
    McpCompleted {
        id: String,
        output: Json,
    },
    Error(String),
    Terminal {
        ok: bool,
    },
    Other,
}

fn normalize_event(event: &Json) -> NormalizedEvent {
    if let Ok(rpc) = serde_json::from_value::<RpcEvent>(event.clone()) {
        match rpc.method.as_str() {
            "item/agentMessage/delta" => {
                return NormalizedEvent::AgentDelta {
                    item_id: rpc.params.item_id,
                    delta: rpc.params.delta.unwrap_or_default(),
                };
            }
            "turn/completed" => return NormalizedEvent::Terminal { ok: true },
            "turn/failed" => return NormalizedEvent::Terminal { ok: false },
            _ => {}
        }
    }

    let Some(kind) = event.get("type").and_then(Json::as_str) else {
        return NormalizedEvent::Other;
    };
    match kind {
        "item.completed" => normalize_completed_item(event.get("item").unwrap_or(&Json::Null)),
        "item.started" => normalize_started_item(event.get("item").unwrap_or(&Json::Null)),
        "turn.completed" => NormalizedEvent::Terminal { ok: true },
        "turn.failed" => NormalizedEvent::Terminal { ok: false },
        "error" => NormalizedEvent::Error(
            event
                .get("message")
                .or_else(|| event.get("error").and_then(|e| e.get("message")))
                .and_then(Json::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        ),
        _ => NormalizedEvent::Other,
    }
}

fn normalize_started_item(item: &Json) -> NormalizedEvent {
    if item_type(item) != Some("mcp_tool_call") {
        return NormalizedEvent::Other;
    }
    let id = item_id(item);
    let server = item.get("server").and_then(Json::as_str).unwrap_or("");
    let tool = item
        .get("tool")
        .or_else(|| item.get("tool_name"))
        .and_then(Json::as_str)
        .unwrap_or("mcp_tool_call");
    let name = if server.is_empty() {
        tool.to_string()
    } else {
        format!("{server}::{tool}")
    };
    let args = item
        .get("arguments")
        .or_else(|| item.get("args"))
        .or_else(|| item.get("input"))
        .cloned()
        .unwrap_or(Json::Null);
    NormalizedEvent::McpStarted { id, name, args }
}

fn normalize_completed_item(item: &Json) -> NormalizedEvent {
    match item_type(item) {
        Some("agent_message") | Some("assistant_message") => NormalizedEvent::AgentCompleted {
            item_id: item.get("id").and_then(Json::as_str).map(str::to_owned),
            text: item
                .get("text")
                .or_else(|| item.get("message"))
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
        },
        Some("mcp_tool_call") => {
            let output = item
                .get("output")
                .or_else(|| item.get("result"))
                .or_else(|| item.get("error"))
                .cloned()
                .unwrap_or_else(|| item.clone());
            NormalizedEvent::McpCompleted {
                id: item_id(item),
                output,
            }
        }
        Some("error") => NormalizedEvent::Error(
            item.get("message")
                .and_then(Json::as_str)
                .unwrap_or("item error")
                .to_string(),
        ),
        _ => NormalizedEvent::Other,
    }
}

fn item_type(item: &Json) -> Option<&str> {
    item.get("type")
        .or_else(|| item.get("item_type"))
        .and_then(Json::as_str)
}

fn item_id(item: &Json) -> String {
    item.get("id")
        .and_then(Json::as_str)
        .unwrap_or("codex_mcp_call")
        .to_string()
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

#[derive(Deserialize)]
struct RpcEvent {
    method: String,
    #[serde(default)]
    params: RpcParams,
}

#[derive(Default, Deserialize)]
struct RpcParams {
    #[serde(default, rename = "itemId")]
    item_id: Option<String>,
    #[serde(default)]
    delta: Option<String>,
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
        Error,
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
            let _ = e;
            self.0.lock().unwrap().push(Event::Error);
        }
    }

    fn fake_codex(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("fake_codex.sh");
        let script = format!("#!/bin/sh\ncat >/dev/null\ncat <<'EOF'\n{body}\nEOF\n");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[tokio::test]
    async fn streams_rpc_agent_deltas_and_dedupes_completed_message() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"method":"item/agentMessage/delta","params":{"itemId":"msg_1","delta":"hello "}}
{"method":"item/agentMessage/delta","params":{"itemId":"msg_1","delta":"world"}}
{"type":"item.completed","item":{"id":"msg_1","type":"agent_message","text":"hello world"}}
{"type":"turn.completed","turn":{"status":"completed"}}"#;
        let script = fake_codex(tmp.path(), body);
        let backend = CodexCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
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
            .filter_map(|e| match e {
                Event::Delta(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, vec!["hello ", "world"]);
    }

    #[tokio::test]
    async fn forwards_mcp_tool_events() {
        let tmp = TempDir::new().unwrap();
        let body = r#"{"type":"item.started","item":{"id":"call_1","type":"mcp_tool_call","server":"chan","tool":"read_file","arguments":{"path":"a.md"}}}
{"type":"item.completed","item":{"id":"call_1","type":"mcp_tool_call","server":"chan","tool":"read_file","output":"ok"}}
{"type":"item.completed","item":{"id":"msg_1","type":"agent_message","text":"done"}}
{"type":"turn.completed","turn":{"status":"completed"}}"#;
        let script = fake_codex(tmp.path(), body);
        let backend = CodexCliBackend::new(
            vec![script.to_string_lossy().into_owned()],
            Vec::new(),
            None,
            tmp.path().to_path_buf(),
            None,
            Duration::from_secs(60),
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
        assert_eq!(outcome.assistant_text, "done");

        let events = listener.0.lock().unwrap();
        let tool_calls: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::ToolCall(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(tool_calls, vec!["chan::read_file"]);
        let tool_results: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::ToolResult(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(tool_results, vec!["call_1"]);
    }

    #[test]
    fn mcp_overrides_are_toml_literals() {
        assert_eq!(toml_string("/bin/chan").unwrap(), "\"/bin/chan\"");
        assert_eq!(
            toml_array(&["__mcp".into(), "/tmp/drive".into()]).unwrap(),
            "[\"__mcp\", \"/tmp/drive\"]"
        );
    }
}
