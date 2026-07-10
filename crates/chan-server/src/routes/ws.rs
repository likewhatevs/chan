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

/// The target window id of a `window_command` broadcast frame, or `None` for
/// any other frame (which is genuinely broadcast to every socket).
///
/// window_command frames serialize compactly with fields in declaration order
/// as `{"type":"window_command","window_id":"<id>",...}` (see
/// `WindowCommandFrame` in `control_socket`), so the id reads off that fixed
/// prefix without parsing the rest of the command -- for `clipboard_write`
/// that tail is a multi-MB base64 payload we must not re-parse on every
/// connection. A format drift just makes this return `None`, so the frame is
/// forwarded and the SPA's own `window_id` gate still filters it: it fails safe.
fn window_command_target(frame: &str) -> Option<&str> {
    const PREFIX: &str = "{\"type\":\"window_command\",\"window_id\":\"";
    let rest = frame.strip_prefix(PREFIX)?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

pub async fn ws_upgrade(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
    // A `/ws` that arrived over the devserver's gateway tunnel carries the
    // `TunnelOrigin` request-extension marker; the loopback bind (and an
    // `ssh -L` forward to it) never does, nor does the desktop embedded server.
    // The `Option` extractor yields `None` on absence rather than 500ing, so
    // absence means a local-origin socket. This is the session-role seam: a
    // local socket reads Leader, a tunnel socket reads Follower.
    origin: Option<axum::Extension<crate::TunnelOrigin>>,
    ws: WebSocketUpgrade,
) -> Response {
    let local = origin.is_none();
    // Gateway identity for a tunnel participant: the proxy's per-request
    // assertion carries the caller's name/email claims; map them into
    // presence here so the roster renders `Display Name <email>` without
    // chan-library depending on chan-tunnel-proto. Absent claims (an older
    // gateway, or a local socket) leave the participant on its generated
    // default name.
    let identity = origin.as_ref().and_then(|ext| {
        let caller = ext.0.caller.as_ref()?;
        if caller.name.is_none() && caller.email.is_none() {
            return None;
        }
        Some(crate::session_presence::ParticipantIdentity {
            display_name: caller.name.clone(),
            email: caller.email.clone(),
        })
    });
    let rx = state.events_tx.subscribe();
    let last_activity = state.last_activity.clone();
    let shutdown_rx = state.shutdown_rx.clone();
    let scopes = state.scope_registry.clone();
    let presence = state.window_presence.clone();
    let transfers = state.window_transfers.clone();
    let session_registry = state.session_registry.clone();
    let session_events_tx = state.events_tx.clone();
    let window_id = q.w.map(|w| w.trim().to_string()).filter(|w| !w.is_empty());
    ws.on_upgrade(move |mut socket| async move {
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
            let join = session_registry.join(id, local, identity);
            if join.changed {
                crate::session_roster::broadcast_session_roster(
                    &session_events_tx,
                    &session_registry,
                );
            }
            join.guard
        });
        // Per-socket roster snapshot on connect, for tagged AND untagged sockets.
        // The broadcast above fires only when the join MOVES the roster, so a
        // reload (the socket-overlap window reports changed=false) would leave
        // this fresh socket with no roster until some unrelated change -- the
        // starvation that strands isLeader()/roster UI. Sending the current
        // snapshot straight to this socket guarantees it a first frame, and an
        // untagged observer (no `?w=`, no join) learns the roster the same way.
        if let Some(frame) = crate::session_roster::serialize_session_roster(&session_registry) {
            if socket.send(Message::text(frame)).await.is_err() {
                return;
            }
        }
        ws_pump(
            socket,
            rx,
            last_activity,
            shutdown_rx,
            scopes,
            transfer_guard,
            window_id,
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
    window_id: Option<String>,
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
        window_id.as_deref(),
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
    window_id: Option<&str>,
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
                    if socket.send(Message::text(frame)).await.is_err() {
                        break;
                    }
                    last_activity.store(now_unix_secs(), Ordering::Relaxed);
                }
                None => break,
            },
            recv = rx.recv() => match recv {
                Ok(frame) => {
                    // A window_command is addressed to ONE window: forward it
                    // only to the socket serving that window (an untagged
                    // socket is never a target). This keeps request_ids and
                    // clipboard payloads off other windows' sockets server-side,
                    // hardening the reply-hijack surface beyond the SPA's gate.
                    // All other frame types stay broadcast to every socket.
                    if let Some(target) = window_command_target(&frame) {
                        if window_id != Some(target) {
                            continue;
                        }
                    }
                    if socket.send(Message::text(frame)).await.is_err() {
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
    fn window_command_target_extracts_the_addressed_window() {
        // A window_command frame yields its target window_id; the pump forwards
        // it only to that window's socket.
        let frame = r#"{"type":"window_command","window_id":"workspace-aa-0","command":"clipboard_write","request_id":"r1","mime":"image/png","data_b64":"AAAA"}"#;
        assert_eq!(window_command_target(frame), Some("workspace-aa-0"));

        // Non-window_command frames are broadcast (return None), so the pump
        // forwards them to every socket unchanged.
        assert_eq!(
            window_command_target(r#"{"type":"progress","pct":10}"#),
            None
        );
        assert_eq!(
            window_command_target(r#"{"type":"session_roster","rows":[]}"#),
            None
        );
        assert_eq!(window_command_target("not json"), None);
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
