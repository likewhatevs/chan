// LlmSession: the public handle the assistant operates through.
//
// Designed callback-first so uniffi can wrap it cleanly later. The
// caller (chan-server, a future native shell, the CLI) implements
// `SessionListener` and hands an `Arc` to `LlmSession::send`. The
// session drives the HTTP stream on an internal tokio runtime and
// dispatches into the listener as deltas, tool calls, and the
// final stop reason arrive.
//
// Async stays inside. Public methods don't return `Future`; they
// kick off background work and return immediately. This is the
// same pattern `chan_core::Drive::watch` uses, for the same reason:
// a foreign-language consumer shouldn't have to negotiate an async
// runtime across the FFI boundary.
//
// Tool-call orchestration: this commit is text-only. When the
// follow-up commit lands tool round-trips, `send` will spawn a
// loop that drives backend.run() -> on_tool_call -> wait for
// host-supplied result -> next backend.run() until a non-tool
// stop reason. The loop lives in this module; backends just
// translate one HTTP exchange.

use std::sync::{Arc, OnceLock};

use chan_core::Drive;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tokio::runtime::Runtime;

use crate::backends::{self, BackendKind};
use crate::config::LlmConfig;
use crate::error::LlmError;
use crate::tools::ToolContext;

/// Conversation roles. The taxonomy mirrors OpenAI / Anthropic
/// conventions so backends don't have to invent their own; each
/// translates these to its wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// One turn in the conversation. The host (chan-server, native
/// shell) owns the history and passes the full transcript on each
/// `send` call. Stateless on the chan-llm side keeps the FFI
/// surface simple: no hidden state to synchronize across the
/// boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Set on `Role::Tool` messages to identify which tool call
    /// this is the result of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Set on `Role::Assistant` messages that include tool calls.
    /// Mirrors the previous turn's tool calls so the assistant
    /// can reference them across the conversation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: Vec::new(),
        }
    }
}

/// Streaming text delta. Backends emit these as they receive
/// SSE / streaming JSON chunks from the upstream model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub text: String,
}

/// One tool call the assistant proposes during generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Backend-assigned id. Surfaced so multiple parallel tool
    /// calls can be matched to their results.
    pub id: String,
    pub name: String,
    pub args: Json,
}

/// Result of executing a tool the assistant requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub output: Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndOfTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    Error,
}

/// What the consumer implements. `Send + Sync` because events
/// arrive on the runtime's worker threads.
pub trait SessionListener: Send + Sync {
    fn on_delta(&self, delta: Delta);
    fn on_tool_call(&self, call: ToolCall);
    fn on_tool_result(&self, result: ToolResult);
    fn on_done(&self, reason: StopReason);
    fn on_error(&self, error: String);
}

pub struct LlmSession {
    drive: Arc<Drive>,
    config: LlmConfig,
}

impl LlmSession {
    pub fn new(drive: Arc<Drive>, config: LlmConfig) -> Self {
        Self { drive, config }
    }

    pub fn backend(&self) -> Option<BackendKind> {
        self.config.backend
    }

    pub fn tool_context(&self) -> ToolContext {
        ToolContext::new(self.drive.clone(), self.config.auto_apply_writes)
    }

    /// Kick off a turn. The host passes the full conversation
    /// transcript; chan-llm prepends the system prompt + tool
    /// descriptions automatically. Returns immediately after
    /// spawning the background task; events flow into the
    /// listener.
    ///
    /// Today: text-only. Tool calls in a follow-up commit; this
    /// build's `on_tool_call` hook on the listener is unused by
    /// any backend.
    pub fn send(&self, messages: Vec<Message>, listener: Arc<dyn SessionListener>) {
        let Some(kind) = self.config.backend else {
            listener.on_error(LlmError::MissingApiKey("no backend configured".into()).to_string());
            listener.on_done(StopReason::Error);
            return;
        };

        let backend = match backends::build(kind, &self.config) {
            Ok(b) => b,
            Err(e) => {
                listener.on_error(e.to_string());
                listener.on_done(StopReason::Error);
                return;
            }
        };

        // Prepend the chan-wide system prompt unless the host
        // already provided one (a few editor surfaces want a
        // chat-only variant; passing the system message
        // explicitly overrides the default).
        let mut prepared: Vec<Message> = Vec::with_capacity(messages.len() + 1);
        if !messages.first().is_some_and(|m| m.role == Role::System) {
            prepared.push(Message::system(crate::prompts::SYSTEM_PROMPT));
        }
        prepared.extend(messages);

        let listener_for_task = listener.clone();
        spawn(async move {
            backend.run(prepared, listener_for_task.clone()).await;
        });
    }
}

/// Spawn a future onto whichever runtime is appropriate. When
/// chan-llm is called from a tokio context (chan-server's runtime),
/// we use that. Otherwise (a CLI / native shell with no ambient
/// runtime) we fall back to a process-shared runtime created on
/// first use.
fn spawn<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(future);
    } else {
        shared_runtime().spawn(future);
    }
}

fn shared_runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build chan-llm shared tokio runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_core::Library;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct Collector(Mutex<Vec<Event>>);

    #[allow(dead_code)]
    enum Event {
        Delta(String),
        ToolCall(String),
        ToolResult(String),
        Done(StopReason),
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
        fn on_done(&self, r: StopReason) {
            self.0.lock().unwrap().push(Event::Done(r));
        }
        fn on_error(&self, e: String) {
            self.0.lock().unwrap().push(Event::Error(e));
        }
    }

    fn fixture() -> (TempDir, TempDir, Arc<Drive>) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        (cfg, drive_dir, drive)
    }

    #[test]
    fn send_with_no_backend_emits_error_and_done() {
        let (_cfg, _root, drive) = fixture();
        let session = LlmSession::new(drive, LlmConfig::default());
        let collector = Arc::new(Collector(Mutex::new(Vec::new())));
        session.send(vec![Message::user("hi")], collector.clone());
        let events = collector.0.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::Error(_)));
        assert!(matches!(events[1], Event::Done(StopReason::Error)));
    }

    #[test]
    fn message_constructors() {
        let u = Message::user("hi");
        assert_eq!(u.role, Role::User);
        let s = Message::system("you are chan");
        assert_eq!(s.role, Role::System);
        let a = Message::assistant("ok");
        assert_eq!(a.role, Role::Assistant);
        let t = Message::tool("call-1", "result");
        assert_eq!(t.role, Role::Tool);
        assert_eq!(t.tool_call_id.as_deref(), Some("call-1"));
    }
}
