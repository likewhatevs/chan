//! Bridges from chan-drive's watcher and chan-llm's session listener
//! into the shared `events_tx` JSON broadcast channel.
//!
//! Both producers fan into one channel; each frame carries a `type`
//! discriminator so the frontend can route on it. Watcher events also
//! get forwarded to the indexer's raw-event channel — the indexer
//! does NOT honor the self-write dedupe, since in-app saves must
//! reindex.

use std::sync::Arc;

use chan_drive::{ProgressCallback, ProgressEvent, WatchCallback, WatchEvent};
use chan_llm::{
    AgentActivity, AgentStatus, Delta, SessionListener, StopReason, ToolCall, ToolResult,
    UserRequest,
};
use tokio::sync::broadcast;

use crate::self_writes::SelfWrites;

/// Construct a watcher bridge. Extracted so /api/storage/reset can
/// rebuild one cheaply when re-attaching the watcher to a fresh
/// Drive instance.
///
/// The bridge fans out every event to two consumers:
///
///   - `events_tx`: pre-serialized JSON frames forwarded to /ws
///     subscribers. Self-write echoes (the editor saving through
///     /api/markdown PUT and then seeing its own save) are
///     suppressed here so the UI doesn't show a phantom external-
///     edit toast.
///   - `index_tx`: raw `WatchEvent` for the background indexer.
///     Self-write suppression DOES NOT apply here: in-app saves
///     must reindex, otherwise search drifts every time the user
///     types. The indexer applies its own debounce.
pub fn make_watch_bridge(
    events_tx: &broadcast::Sender<String>,
    index_tx: &broadcast::Sender<WatchEvent>,
    self_writes: &Arc<SelfWrites>,
) -> Arc<dyn WatchCallback> {
    Arc::new(WatchBroadcast {
        tx: events_tx.clone(),
        index_tx: index_tx.clone(),
        self_writes: self_writes.clone(),
    })
}

struct WatchBroadcast {
    tx: broadcast::Sender<String>,
    index_tx: broadcast::Sender<WatchEvent>,
    self_writes: Arc<SelfWrites>,
}

impl WatchCallback for WatchBroadcast {
    fn on_event(&self, event: WatchEvent) {
        // Indexer always sees the event. Send-error means there are
        // no subscribers (indexer not spawned yet, or shut down);
        // safe to drop because a no-subscriber channel just keeps
        // events in the ring until one connects.
        let _ = self.index_tx.send(event.clone());
        if event_is_self_echo(&event, &self.self_writes) {
            return;
        }
        let frame = serde_json::json!({"type": "watch", "event": event});
        if let Ok(s) = serde_json::to_string(&frame) {
            let _ = self.tx.send(s);
        }
    }
}

fn event_is_self_echo(event: &WatchEvent, sw: &SelfWrites) -> bool {
    if let Some(p) = event.path.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    if let Some(p) = event.to.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    false
}

/// Bridge from chan-drive's `ProgressCallback` into the shared
/// JSON-envelope broadcast channel. Every progress tick (per-file
/// during reindex, per-batch during embedding, etc.) lands on the
/// same `/ws` stream every other producer uses, with `type` set to
/// `"progress"` so the frontend can route the frame distinctly
/// from `watch` and `llm.*`.
///
/// `Send + Sync` because `ProgressCallback` can fire from worker
/// threads inside the embedder and graph rebuilders.
pub fn make_progress_broadcast(events_tx: &broadcast::Sender<String>) -> Arc<dyn ProgressCallback> {
    Arc::new(ProgressBroadcast {
        tx: events_tx.clone(),
    })
}

struct ProgressBroadcast {
    tx: broadcast::Sender<String>,
}

impl ProgressCallback for ProgressBroadcast {
    fn on_progress(&self, event: ProgressEvent) {
        let frame = serde_json::json!({"type": "progress", "event": event});
        if let Ok(s) = serde_json::to_string(&frame) {
            // Best-effort: lagged subscribers are dropped by the
            // broadcast channel naturally; a no-subscriber send
            // returns an error we ignore for the same reason as
            // the watch bridge above.
            let _ = self.tx.send(s);
        }
    }
}

/// Bridge from chan-llm's SessionListener into the shared broadcast
/// channel. One listener instance per /api/llm/complete call; dropped
/// when the session emits `Done` or when the consumer drops the
/// `Arc` at the end of the request handler.
///
/// `session_id` is client-supplied so the frontend can correlate
/// streaming events to its in-flight assistant turn (multiple
/// turns can interleave on the same socket).
pub struct LlmBroadcastListener {
    pub tx: broadcast::Sender<String>,
    pub session_id: String,
}

impl LlmBroadcastListener {
    fn send(&self, ty: &str, body: serde_json::Value) {
        let mut frame = serde_json::Map::new();
        frame.insert("type".into(), ty.into());
        frame.insert("session_id".into(), self.session_id.clone().into());
        if let serde_json::Value::Object(map) = body {
            for (k, v) in map {
                frame.insert(k, v);
            }
        }
        if let Ok(s) = serde_json::to_string(&serde_json::Value::Object(frame)) {
            let _ = self.tx.send(s);
        }
    }
}

impl SessionListener for LlmBroadcastListener {
    fn on_status(&self, status: AgentStatus) {
        self.send("llm.status", serde_json::json!({"status": status}));
    }
    fn on_activity(&self, activity: AgentActivity) {
        self.send("llm.activity", serde_json::json!({"activity": activity}));
    }
    fn on_user_request(&self, request: UserRequest) {
        self.send("llm.user_request", serde_json::json!({"request": request}));
    }
    fn on_delta(&self, d: Delta) {
        self.send("llm.delta", serde_json::json!({"text": d.text}));
    }
    fn on_tool_call(&self, c: ToolCall) {
        self.send("llm.tool_call", serde_json::json!({"call": c}));
    }
    fn on_tool_result(&self, r: ToolResult) {
        self.send("llm.tool_result", serde_json::json!({"result": r}));
    }
    fn on_done(&self, r: StopReason) {
        self.send("llm.done", serde_json::json!({"reason": r}));
    }
    fn on_error(&self, e: String) {
        self.send("llm.error", serde_json::json!({"error": e}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn listener() -> (LlmBroadcastListener, broadcast::Receiver<String>) {
        let (tx, rx) = broadcast::channel(8);
        (
            LlmBroadcastListener {
                tx,
                session_id: "A".to_string(),
            },
            rx,
        )
    }

    fn recv_json(rx: &mut broadcast::Receiver<String>) -> Value {
        let raw = rx.try_recv().expect("broadcast frame");
        serde_json::from_str(&raw).expect("json frame")
    }

    #[test]
    fn status_frame_serializes_with_session_id() {
        let (listener, mut rx) = listener();
        let status = AgentStatus::Heartbeat {
            backend: "claude_cli".into(),
            idle_ms: 1500,
        };

        listener.on_status(status.clone());

        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.status");
        assert_eq!(frame["session_id"], "A");
        assert_eq!(
            frame["status"],
            serde_json::to_value(status).expect("status value")
        );
    }

    #[test]
    fn activity_frame_serializes_with_session_id() {
        let (listener, mut rx) = listener();
        let activity = AgentActivity::ToolStarted {
            backend: "claude_cli".into(),
            id: "toolu_1".into(),
            name: "read_file".into(),
            parent_id: Some("msg_1".into()),
        };

        listener.on_activity(activity.clone());

        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.activity");
        assert_eq!(frame["session_id"], "A");
        assert_eq!(
            frame["activity"],
            serde_json::to_value(activity).expect("activity value")
        );
    }

    #[test]
    fn user_request_frame_serializes_with_session_id() {
        let (listener, mut rx) = listener();
        let request = UserRequest::Survey {
            backend: "claude_cli".into(),
            id: "survey_1".into(),
            questions: vec![chan_llm::UserQuestion {
                question: "Proceed?".into(),
                header: Some("Confirm".into()),
                multi_select: false,
                options: vec![chan_llm::UserOption {
                    label: "Yes".into(),
                    description: Some("Continue".into()),
                }],
            }],
            parent_id: Some("msg_1".into()),
        };

        listener.on_user_request(request.clone());

        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.user_request");
        assert_eq!(frame["session_id"], "A");
        assert_eq!(
            frame["request"],
            serde_json::to_value(request).expect("request value")
        );
    }

    #[test]
    fn existing_llm_frames_keep_shape() {
        let (listener, mut rx) = listener();

        listener.on_delta(Delta { text: "hi".into() });
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.delta");
        assert_eq!(frame["session_id"], "A");
        assert_eq!(frame["text"], "hi");

        let call = ToolCall {
            id: "call_1".into(),
            name: "read_file".into(),
            args: serde_json::json!({"path": "a.md"}),
        };
        listener.on_tool_call(call.clone());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.tool_call");
        assert_eq!(
            frame["call"],
            serde_json::to_value(call).expect("call value")
        );

        let result = ToolResult {
            id: "call_1".into(),
            output: serde_json::json!({"ok": true}),
        };
        listener.on_tool_result(result.clone());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.tool_result");
        assert_eq!(
            frame["result"],
            serde_json::to_value(result).expect("result value")
        );

        listener.on_done(StopReason::EndOfTurn);
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.done");
        assert_eq!(frame["reason"], "end_of_turn");

        listener.on_error("boom".into());
        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "llm.error");
        assert_eq!(frame["error"], "boom");
    }
}
