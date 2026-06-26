//! Pushing the session roster onto `/ws` and reaping disconnected
//! participants.
//!
//! The session registry ([`chan_library::session_presence`]) is the source of
//! truth for who leads and who follows one tenant. Two pieces connect it to the
//! clients: [`broadcast_session_roster`] serializes a snapshot into a
//! `session_roster` frame on the shared `events_tx` (every `/ws` socket gets it
//! and the SPA marks itself by `window_id`), and [`spawn_session_reaper`] is the
//! per-tenant task that advances the grace clock so a disconnected leader is
//! eventually replaced even when no new frame arrives to drive it.

use std::sync::Arc;
use std::time::Instant;

use chan_library::session_presence::{SessionRegistry, SessionSnapshot};
use serde::Serialize;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

/// The `/ws` frame carrying a full session snapshot. Mirrors the
/// `terminal_roster` frame shape: a `type` tag plus the flattened payload, so
/// the SPA's `onWatchEvent` switch routes it by `type`.
#[derive(Serialize)]
struct SessionRosterFrame<'a> {
    #[serde(rename = "type")]
    frame_type: &'static str,
    #[serde(flatten)]
    snapshot: &'a SessionSnapshot,
}

/// Serialize the registry's current snapshot and broadcast it to every `/ws`
/// socket. A no-op if serialization fails or there are no subscribers (a
/// dropped frame on an empty session is harmless; the next change re-sends).
pub fn broadcast_session_roster(events_tx: &broadcast::Sender<String>, registry: &SessionRegistry) {
    let snapshot = registry.snapshot(Instant::now());
    let frame = SessionRosterFrame {
        frame_type: "session_roster",
        snapshot: &snapshot,
    };
    if let Ok(raw) = serde_json::to_string(&frame) {
        let _ = events_tx.send(raw);
    }
}

/// Spawn the per-tenant session reaper. It reaps due participants, broadcasts a
/// fresh roster whenever the snapshot moves, then sleeps exactly until the next
/// grace transition -- woken early when a participant disconnects (which arms a
/// new deadline) and stopped on tenant shutdown. Mirrors
/// [`crate::routes::terminal::spawn_roster_broadcaster`].
pub fn spawn_session_reaper(
    registry: Arc<SessionRegistry>,
    events_tx: broadcast::Sender<String>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let outcome = registry.reap_due(Instant::now());
            if outcome.changed {
                broadcast_session_roster(&events_tx, &registry);
            }
            // Sleep until the soonest transition, or forever when no
            // participant is in its grace window; either way the wake or the
            // shutdown arm interrupts the sleep.
            let timer = async {
                match outcome.next_deadline {
                    Some(deadline) => {
                        tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await
                    }
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                _ = shutdown_rx.changed() => break,
                _ = registry.reaper_wake().notified() => {}
                _ = timer => {}
            }
        }
    })
}
