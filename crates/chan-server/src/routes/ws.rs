//! GET /ws — WebSocket pump for watcher events and LLM streaming frames.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::{broadcast, watch};

use crate::signal::now_unix_secs;
use crate::state::AppState;

pub async fn ws_upgrade(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> Response {
    let rx = state.events_tx.subscribe();
    let last_activity = state.last_activity.clone();
    let shutdown_rx = state.shutdown_rx.clone();
    ws.on_upgrade(move |socket| ws_pump(socket, rx, last_activity, shutdown_rx))
}

/// Forward pre-serialized JSON envelope frames to one WebSocket
/// client until either side hangs up. Producers (WatchBroadcast,
/// LlmBroadcastListener) build the JSON once; this pump just
/// fans out. Lagged subscribers skip ahead rather than tearing
/// down the connection.
///
/// Each successful send bumps `last_activity` so that LLM token
/// streams and watcher events keep the idle-timeout window open
/// (otherwise a long generation could be killed by `--timeout`).
/// Idle subscribers with no traffic do not bump the timer.
async fn ws_pump(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<String>,
    last_activity: Arc<AtomicU64>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            biased;
            // Server-initiated shutdown: send a Close frame so the
            // client knows this isn't a network hiccup, then return.
            // Without this branch the broadcast.recv() below would
            // block forever during a graceful shutdown, holding axum's
            // drain open until the hard deadline expires.
            _ = shutdown_rx.changed() => {
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: 1001, // going away
                        reason: "server shutdown".into(),
                    })))
                    .await;
                break;
            }
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
        }
    }
}
