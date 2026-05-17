//! Bridges chan-drive watcher/progress callbacks into the shared
//! `events_tx` JSON broadcast channel.
//!
//! Both producers fan into one channel; each frame carries a `type`
//! discriminator so the frontend can route on it. Watcher events also
//! get forwarded to the indexer's raw-event channel — the indexer
//! does NOT honor the self-write dedupe, since in-app saves must
//! reindex.

use std::sync::Arc;

use chan_drive::{ProgressCallback, ProgressEvent, WatchCallback, WatchEvent};
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
/// `"progress"` so the frontend can route the frame distinctly.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn recv_json(rx: &mut broadcast::Receiver<String>) -> Value {
        let raw = rx.try_recv().expect("broadcast frame");
        serde_json::from_str(&raw).expect("json frame")
    }

    #[test]
    fn progress_frame_serializes() {
        let (tx, mut rx) = broadcast::channel(8);
        let sink = make_progress_broadcast(&tx);
        sink.on_progress(ProgressEvent {
            stage: chan_drive::ProgressStage::IndexFile,
            current: 1,
            total: 2,
            label: Some("a.md".into()),
            eta_secs: None,
        });

        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "progress");
        assert_eq!(frame["event"]["stage"], "IndexFile");
    }
}
