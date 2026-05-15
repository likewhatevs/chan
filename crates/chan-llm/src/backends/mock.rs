//! Mock backend for benchmarks and harness tests. Emits a canned
//! sequence of `Delta` / `ToolCall` / stop-reason events through the
//! listener with no I/O. Lets `LlmSession`-driven orchestration
//! benchmarks measure only chan-llm's own per-turn cost (no HTTP, no
//! subprocess, no real API tokens) so a regression in the
//! orchestrator, transcript handling, or listener dispatch shows up
//! cleanly.
//!
//! Gated on `#[cfg(any(test, feature = "bench"))]` so the module
//! only compiles for tests inside this crate and for the
//! `end_to_end` bench (which sets the `bench` feature via
//! `required-features` in `Cargo.toml`). Production consumers never
//! pay for it.

#![cfg(any(test, feature = "bench"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use crate::session::{Delta, Message, SessionListener, StopReason, ToolCall};
use crate::tools::ToolSchema;

use super::{Backend, Outcome};

/// One canned event the mock replays through the listener. The
/// orchestrator's response to each kind is the same as for any real
/// backend: deltas accumulate into `assistant_text`, tool calls
/// arrive on `Outcome.tool_calls` and trigger the post-turn tool
/// dispatch, and the terminal `Done` carries the stop reason.
#[derive(Debug, Clone)]
pub enum MockEvent {
    Delta(String),
    ToolCall(ToolCall),
    Done(StopReason),
}

/// A `Backend` that replays a fixed `Vec<MockEvent>` script. One
/// instance per turn; the loop holds it under an `Arc` and reuses
/// it across iterations. Cancel is honored at the per-event
/// boundary so a bench that flips the flag mid-stream gets a
/// `Cancelled` outcome promptly.
pub struct MockBackend {
    /// Event sequence to emit on every `run` call. The script is
    /// replayed verbatim each turn; benches that want per-turn
    /// variation construct a new backend with a new script.
    pub script: Vec<MockEvent>,
}

#[async_trait]
impl Backend for MockBackend {
    async fn run(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        listener: Arc<dyn SessionListener>,
        cancel: Arc<AtomicBool>,
    ) -> Outcome {
        let mut assistant_text = String::new();
        let mut tool_calls = Vec::new();
        let mut stop = StopReason::EndOfTurn;
        for ev in &self.script {
            if cancel.load(Ordering::Relaxed) {
                return Outcome::cancelled(assistant_text);
            }
            match ev {
                MockEvent::Delta(t) => {
                    listener.on_delta(Delta { text: t.clone() });
                    assistant_text.push_str(t);
                }
                MockEvent::ToolCall(c) => tool_calls.push(c.clone()),
                MockEvent::Done(r) => stop = *r,
            }
        }
        Outcome {
            assistant_text,
            tool_calls,
            stop_reason: stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::session::ToolResult;

    struct Sink(Mutex<usize>);
    impl SessionListener for Sink {
        fn on_delta(&self, _: Delta) {
            *self.0.lock().unwrap() += 1;
        }
        fn on_tool_call(&self, _: ToolCall) {}
        fn on_tool_result(&self, _: ToolResult) {}
        fn on_done(&self, _: StopReason) {}
        fn on_error(&self, _: String) {}
    }

    #[tokio::test]
    async fn replays_deltas_and_stop_reason() {
        let backend = MockBackend {
            script: vec![
                MockEvent::Delta("a".into()),
                MockEvent::Delta("b".into()),
                MockEvent::Done(StopReason::EndOfTurn),
            ],
        };
        let sink = Arc::new(Sink(Mutex::new(0)));
        let outcome = backend
            .run(
                &[],
                &[],
                sink.clone() as Arc<dyn SessionListener>,
                Arc::new(AtomicBool::new(false)),
            )
            .await;
        assert_eq!(outcome.assistant_text, "ab");
        assert_eq!(outcome.stop_reason, StopReason::EndOfTurn);
        assert!(outcome.tool_calls.is_empty());
        assert_eq!(*sink.0.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn cancel_mid_script_returns_cancelled() {
        let backend = MockBackend {
            script: vec![
                MockEvent::Delta("first".into()),
                MockEvent::Delta("second".into()),
                MockEvent::Done(StopReason::EndOfTurn),
            ],
        };
        let sink = Arc::new(Sink(Mutex::new(0)));
        let cancel = Arc::new(AtomicBool::new(true));
        let outcome = backend
            .run(&[], &[], sink as Arc<dyn SessionListener>, cancel)
            .await;
        assert_eq!(outcome.stop_reason, StopReason::Cancelled);
    }
}
