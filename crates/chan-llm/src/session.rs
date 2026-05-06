// LlmSession: the public handle the assistant operates through.
//
// Designed callback-first so uniffi can wrap it cleanly later. The
// caller (chan-server, a future native shell, the CLI) implements
// `SessionListener` and hands an `Arc` to `LlmSession::send`. The
// session drives the HTTP stream on its internal tokio runtime and
// dispatches into the listener as deltas, tool calls, and the final
// stop reason arrive.
//
// Async stays inside. Public methods don't return `Future`; they
// kick off background work and return immediately. This is the same
// pattern `chan_core::Drive::watch` uses, for the same reason: a
// foreign-language consumer shouldn't have to negotiate an async
// runtime across the FFI boundary.
//
// Backends are stubs in this initial commit. `send` immediately
// dispatches `on_error(NotImplemented)` to the listener and returns.
// The shape is locked though; consumers can wire to the API today
// and the real backend ports drop in without surface changes.

use std::sync::Arc;

use chan_core::Drive;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::backends::BackendKind;
use crate::config::LlmConfig;
use crate::error::LlmError;
use crate::tools::ToolContext;

/// Streaming text delta. Backends emit these as they receive
/// SSE / streaming JSON chunks from the upstream model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub text: String,
}

/// One tool call the assistant proposes during generation. The host
/// decides whether to execute it (via `tools::execute`) and reports
/// the result back via the next `on_tool_result` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Backend-assigned id, surfaced so multiple parallel tool
    /// calls can be matched to their results.
    pub id: String,
    pub name: String,
    pub args: Json,
}

/// Result of executing a tool the assistant requested. The host
/// runs the tool (via `tools::execute`) and sends this back to the
/// session so the assistant can continue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub output: Json,
}

/// Why a session stopped. Mirrors the lowest-common-denominator of
/// the three backends; we map vendor-specific reasons into these
/// at translation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndOfTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    Error,
}

/// What the consumer implements. Implementations live in:
///
///   - chan-server  (forwards events over WebSocket to the web UI)
///   - native shells (Swift / Kotlin via uniffi callback objects)
///   - tests (a `Vec<Event>` collector)
///
/// `Send + Sync` because events arrive on the runtime's worker
/// threads.
pub trait SessionListener: Send + Sync {
    fn on_delta(&self, delta: Delta);
    fn on_tool_call(&self, call: ToolCall);
    fn on_tool_result(&self, result: ToolResult);
    fn on_done(&self, reason: StopReason);
    fn on_error(&self, error: String);
}

/// One conversation worth of state. Cheap to clone (Arc inside).
pub struct LlmSession {
    drive: Arc<Drive>,
    config: LlmConfig,
}

impl LlmSession {
    pub fn new(drive: Arc<Drive>, config: LlmConfig) -> Self {
        Self { drive, config }
    }

    /// Returns the active backend kind, falling back to the
    /// config's default if the caller hasn't pinned one yet.
    pub fn backend(&self) -> Option<BackendKind> {
        self.config.backend
    }

    /// Snapshot of the tool context this session will hand to
    /// dispatched tool calls. Exposed so the host can run the same
    /// tool sandbox out-of-band (e.g. for a "preview this tool's
    /// effect" UI).
    pub fn tool_context(&self) -> ToolContext {
        ToolContext::new(self.drive.clone(), self.config.auto_apply_writes)
    }

    /// Kick off a turn. The user's message goes to the configured
    /// backend; deltas, tool calls, and the stop reason flow into
    /// the listener. Returns immediately after spawning the
    /// background work.
    ///
    /// Backends are stubs at this commit. The listener will receive
    /// a single `on_error("backend not implemented yet: ...")`.
    pub fn send(&self, _user_message: String, listener: Arc<dyn SessionListener>) {
        let backend = self.config.backend;
        // Dispatch synchronously here since the stub doesn't actually
        // do any I/O. When real backends land they'll spawn onto an
        // internal tokio runtime and stream events as they arrive;
        // the public signature stays the same.
        match backend {
            None => {
                listener
                    .on_error(LlmError::NotImplemented("no backend configured".into()).to_string());
                listener.on_done(StopReason::Error);
            }
            Some(kind) => {
                listener.on_error(
                    LlmError::NotImplemented(format!(
                        "backend `{}` not implemented yet; ports in follow-up",
                        kind.name(),
                    ))
                    .to_string(),
                );
                listener.on_done(StopReason::Error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_core::Library;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Test listener that collects every event for assertion.
    struct Collector(Mutex<Vec<Event>>);

    // Variants only inspected via `matches!`; clippy 1.95 flags the
    // payload fields as never-read. Allow on this test-only enum
    // rather than dropping the payloads (we'll want them once the
    // backends actually emit deltas).
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
        session.send("hi".into(), collector.clone());
        let events = collector.0.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::Error(_)));
        assert!(matches!(events[1], Event::Done(StopReason::Error)));
    }

    #[test]
    fn send_with_stub_backend_emits_not_implemented() {
        let (_cfg, _root, drive) = fixture();
        let config = LlmConfig {
            backend: Some(BackendKind::Anthropic),
            ..Default::default()
        };
        let session = LlmSession::new(drive, config);
        let collector = Arc::new(Collector(Mutex::new(Vec::new())));
        session.send("hi".into(), collector.clone());
        let events = collector.0.lock().unwrap();
        match &events[0] {
            Event::Error(msg) => {
                assert!(msg.contains("anthropic"), "got: {msg}");
                assert!(msg.contains("not implemented"));
            }
            _ => panic!("expected Error first"),
        }
    }
}
