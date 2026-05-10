//! Bridges from chan-drive's watcher and chan-llm's session listener
//! into the shared `events_tx` JSON broadcast channel.
//!
//! Both producers fan into one channel; each frame carries a `type`
//! discriminator so the frontend can route on it. Watcher events also
//! get forwarded to the indexer's raw-event channel — the indexer
//! does NOT honor the self-write dedupe, since in-app saves must
//! reindex.

use std::sync::Arc;

use chan_drive::{WatchCallback, WatchEvent};
use chan_llm::{Delta, SessionListener, StopReason, ToolCall, ToolResult};
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
