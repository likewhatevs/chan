//! GET /ws — bidirectional WebSocket pump.
//!
//! Server -> client: the global JSON-envelope broadcast (`watch`,
//! `progress`, `window_command`, ...) plus this socket's per-scope `fs`
//! frames from the `ScopeRegistry`.
//!
//! Client -> server: `sub` / `unsub` frames that add/drop this socket's
//! per-directory scope subscriptions. The socket
//! registers with the `ScopeRegistry` on connect and unregisters on any
//! exit path so a disconnect cannot leak scopes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc, watch};

use crate::bus::{ScopeRegistry, SubId};
use crate::signal::now_unix_secs;
use crate::state::AppState;
use crate::window_transfers::TransferGuard;

/// Optional window identity on the event socket (`/ws?w=<id>`): the
/// same per-window id that keys the `/api/session` blob. Tagged
/// sockets register with `WindowPresence` so `GET /api/windows` can
/// report which windows are currently connected. Absent on untagged
/// clients (tests, curl) — they simply don't appear in presence.
#[derive(Deserialize)]
pub struct WsQuery {
    w: Option<String>,
}

pub async fn ws_upgrade(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let rx = state.events_tx.subscribe();
    let last_activity = state.last_activity.clone();
    let shutdown_rx = state.shutdown_rx.clone();
    let scopes = state.scope_registry.clone();
    let presence = state.window_presence.clone();
    let transfers = state.window_transfers.clone();
    let session_registry = state.session_registry.clone();
    let session_events_tx = state.events_tx.clone();
    let window_id = q.w.map(|w| w.trim().to_string()).filter(|w| !w.is_empty());
    ws.on_upgrade(move |socket| async move {
        // RAII presence ref: held across the pump so EVERY exit path
        // (clean close, network drop, shutdown) deregisters the window.
        let _presence = window_id.as_ref().map(|id| presence.connect(id));
        // RAII transfer guard for the same `?w=` window: the pump calls
        // `set` on each `transfers` frame, and Drop clears this socket's
        // contribution on every exit path (so a reload reads inactive).
        let transfer_guard = window_id.as_ref().map(|id| transfers.register(id));
        // RAII session participation: the first socket of a window joins the
        // leader/followers session (electing the leader when it is first); the
        // guard's Drop arms the grace clock when the last socket drops. A join
        // that moves the roster (a new or revived participant) rebroadcasts.
        let _session = window_id.as_ref().map(|id| {
            let join = session_registry.join(id);
            if join.changed {
                crate::session_roster::broadcast_session_roster(
                    &session_events_tx,
                    &session_registry,
                );
            }
            join.guard
        });
        ws_pump(
            socket,
            rx,
            last_activity,
            shutdown_rx,
            scopes,
            transfer_guard,
        )
        .await;
    })
}

/// Client -> server frame. `sub`/`unsub` add/drop this socket's directory
/// scope (`dir: ""` is the workspace root); `transfers` reports this window's
/// in-flight upload/download count for the desktop close guard. Unknown frame
/// types are ignored (the client may send other shapes we don't model here).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientFrame {
    Sub {
        dir: String,
    },
    Unsub {
        dir: String,
    },
    /// `{ "type": "transfers", "active": <n> }` — this window's current
    /// in-flight transfer count. Applied to the socket's `TransferGuard`;
    /// ignored on an untagged socket (no `?w=`, hence no guard).
    Transfers {
        active: usize,
    },
}

/// Forward server -> client frames to one WebSocket client and apply this
/// socket's inbound `sub`/`unsub` frames, until either side hangs up.
///
/// Three inbound server -> client sources are merged: the global broadcast
/// (`rx`, lagged subscribers skip ahead rather than tearing down), this
/// socket's scoped `fs` outbox (`scope_rx`), and the shutdown signal. The
/// fourth `select!` arm reads client text frames and routes sub/unsub to
/// the `ScopeRegistry`. Every successful send bumps `last_activity` to keep
/// the idle-timeout window open.
///
/// The socket registers with the registry on entry and ALWAYS unregisters
/// on exit (every break path falls through to the `unregister` call), so an
/// abrupt disconnect drops all of this socket's scope subscriptions and
/// cannot leak a scope.
async fn ws_pump(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<String>,
    last_activity: Arc<AtomicU64>,
    mut shutdown_rx: watch::Receiver<bool>,
    scopes: Arc<ScopeRegistry>,
    transfer_guard: Option<TransferGuard>,
) {
    let (sub_id, scope_rx) = scopes.register();
    pump_loop(
        &mut socket,
        &mut rx,
        &last_activity,
        &mut shutdown_rx,
        &scopes,
        sub_id,
        scope_rx,
        transfer_guard.as_ref(),
    )
    .await;
    // Unconditional teardown: drops every scope this socket held.
    scopes.unregister(sub_id);
    // `transfer_guard` drops here too, clearing this socket's transfer count.
}

#[allow(clippy::too_many_arguments)]
async fn pump_loop(
    socket: &mut WebSocket,
    rx: &mut broadcast::Receiver<String>,
    last_activity: &Arc<AtomicU64>,
    shutdown_rx: &mut watch::Receiver<bool>,
    scopes: &Arc<ScopeRegistry>,
    sub_id: SubId,
    mut scope_rx: mpsc::UnboundedReceiver<String>,
    transfer_guard: Option<&TransferGuard>,
) {
    loop {
        tokio::select! {
            biased;
            // Server-initiated shutdown: send a Close frame so the
            // client knows this isn't a network hiccup, then return.
            // Without this branch the recv arms below would block
            // forever during a graceful shutdown, holding axum's drain
            // open until the hard deadline expires.
            _ = shutdown_rx.changed() => {
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: 1001, // going away
                        reason: "server shutdown".into(),
                    })))
                    .await;
                break;
            }
            // This socket's scoped `fs` frames. Unbounded channel, so a
            // closed sender (registry torn down) ends the stream.
            scoped = scope_rx.recv() => match scoped {
                Some(frame) => {
                    if socket.send(Message::Text(frame)).await.is_err() {
                        break;
                    }
                    last_activity.store(now_unix_secs(), Ordering::Relaxed);
                }
                None => break,
            },
            recv = rx.recv() => match recv {
                Ok(frame) => {
                    if socket.send(Message::Text(frame)).await.is_err() {
                        break;
                    }
                    last_activity.store(now_unix_secs(), Ordering::Relaxed);
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            },
            // Client -> server: sub/unsub frames. A None / Err means the
            // client closed or sent garbage at the transport level; treat
            // a clean close as end-of-stream. A Close frame ends the pump;
            // Text frames route to the scope registry; other frames
            // (Binary/Ping/Pong) are ignored (axum auto-replies to Ping).
            inbound = socket.recv() => match inbound {
                Some(Ok(Message::Text(text))) => {
                    apply_client_frame(scopes, sub_id, &text, transfer_guard);
                    last_activity.store(now_unix_secs(), Ordering::Relaxed);
                }
                Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
                None => break,
            },
        }
    }
}

/// Parse one client text frame and apply it. `sub`/`unsub` route to the scope
/// registry; `transfers` updates the socket's `TransferGuard` (ignored when the
/// socket is untagged, so there is no guard). Malformed JSON or an unmodeled
/// `type` is dropped silently (the server controls the wire format; a stray
/// frame must not tear down the socket).
fn apply_client_frame(
    scopes: &ScopeRegistry,
    sub_id: SubId,
    text: &str,
    transfer_guard: Option<&TransferGuard>,
) {
    match serde_json::from_str::<ClientFrame>(text) {
        Ok(ClientFrame::Sub { dir }) => scopes.subscribe(sub_id, &dir),
        Ok(ClientFrame::Unsub { dir }) => scopes.unsubscribe(sub_id, &dir),
        Ok(ClientFrame::Transfers { active }) => {
            if let Some(guard) = transfer_guard {
                guard.set(active);
            }
        }
        Err(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the client -> server wire shape (the TS `WsClientFrame` union
    // serializes `{ "type": "sub"|"unsub", "dir": ... }` lowercase) and
    // that a parsed frame routes to the registry as the right sub/unsub.
    #[test]
    fn client_sub_unsub_frames_route_to_the_registry() {
        let reg = ScopeRegistry::new();
        let (id, _rx) = reg.register();

        apply_client_frame(&reg, id, r#"{"type":"sub","dir":"notes/recipes"}"#, None);
        assert!(reg.scope_exists("notes/recipes"));
        assert_eq!(reg.subscriber_count("notes/recipes"), 1);

        apply_client_frame(&reg, id, r#"{"type":"unsub","dir":"notes/recipes"}"#, None);
        assert!(!reg.scope_exists("notes/recipes"));

        // The workspace root scope rides the same path.
        apply_client_frame(&reg, id, r#"{"type":"sub","dir":""}"#, None);
        assert!(reg.scope_exists(""));
    }

    #[test]
    fn malformed_or_unknown_frames_are_dropped_without_panicking() {
        let reg = ScopeRegistry::new();
        let (id, _rx) = reg.register();
        // Bad JSON, an unmodeled type, and a missing field must all be
        // no-ops (a stray frame cannot tear down or corrupt the socket).
        apply_client_frame(&reg, id, "not json", None);
        apply_client_frame(&reg, id, r#"{"type":"bogus","dir":"x"}"#, None);
        apply_client_frame(&reg, id, r#"{"type":"sub"}"#, None);
        assert!(!reg.scope_exists("x"));
        assert_eq!(reg.subscriber_count(""), 0);
    }

    // Pins the `{ "type": "transfers", "active": <n> }` wire shape and that a
    // transfers frame drives the socket's TransferGuard (so the host close
    // guard reads the count). An untagged socket (no guard) ignores it.
    #[test]
    fn transfers_frame_updates_the_window_count() {
        let reg = ScopeRegistry::new();
        let (id, _rx) = reg.register();
        let transfers = Arc::new(crate::window_transfers::WindowTransfers::new());
        let guard = transfers.register("w1");

        apply_client_frame(&reg, id, r#"{"type":"transfers","active":2}"#, Some(&guard));
        assert!(transfers.window_has_active_transfer("w1"));

        apply_client_frame(&reg, id, r#"{"type":"transfers","active":0}"#, Some(&guard));
        assert!(!transfers.window_has_active_transfer("w1"));

        // A socket with no `?w=` (no guard) silently ignores the frame.
        apply_client_frame(&reg, id, r#"{"type":"transfers","active":5}"#, None);
        assert!(!transfers.window_has_active_transfer("w1"));
    }
}
