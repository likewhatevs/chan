//! Embedded chan-tunnel-server for chan-desktop.
//!
//! Lifecycle: explicit. The user clicks "Listen" in Drive Manager,
//! optionally specifies a port, and the backend binds the tunnel
//! listener at that point. Stays bound until the user clicks Stop
//! or the desktop exits. There is no auto-start at boot — the user
//! has to mean it.
//!
//! Topology while listening:
//!
//! ```
//! laptop (chan-desktop)                 remote host
//! ──────────────────────                ───────────
//! tunnel listener  127.0.0.1:<DPORT>  <── ssh -R <RPORT>:localhost:<DPORT>
//!                                         └── chan serve PATH
//!                                               --tunnel-url=http://127.0.0.1:<RPORT>
//!                                               --tunnel-token=<label>
//!
//! shared Arc<Registry>
//!    │
//!    ├─ supervisor task: poll list_all(), spin per-tenant listeners,
//!    │                   emit `tunneled-drive-ready` on new registrations
//!    │
//!    └─ per-tenant axum listener  127.0.0.1:<port>
//!         GET /<drive>/...  → PrependPathLayer → public_router
//!                              (sees /<label>/<drive>/...)
//! ```
//!
//! `<DPORT>` is whatever the user supplied (0 = OS-assigned). The
//! actual bound port is returned from `start_listening` and surfaced
//! in the listen dialog so the user can plug it into `ssh -R`.
//!
//! Security boundary: both listeners bind 127.0.0.1 only. The
//! tunnel listener speaks h2c (cleartext); the SSH `-R` forward
//! provides confidentiality. Any local process that can connect to
//! the desktop's tunnel port can register a drive under any label
//! (the token IS the label), matching the OS process-trust boundary
//! of a single-user desktop app.

mod public;
mod validator;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chan_tunnel_server::{serve_tunnel_listener, Registry, Validator};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use validator::LocalValidator;

/// Tauri event emitted when the tunnel listener starts or stops.
/// Payload is `{ listening: bool, port: Option<u16> }`. Frontend
/// listens to keep the "Listen" panel in sync with backend state
/// (cheap to over-emit; the frontend re-fetches anyway).
pub const TUNNEL_STATE_CHANGED: &str = "tunnel-state-changed";

/// Tauri event emitted when a remote `chan serve` finishes its
/// handshake and the per-tenant listener has bound a port for it.
/// Payload is `{ label, drive, url }`. Frontend uses this to auto-
/// launch the editor for the freshly-registered drive.
pub const TUNNELED_DRIVE_READY: &str = "tunneled-drive-ready";

/// Supervisor poll interval. The registry has no change-notify
/// channel, so we diff `list_all()` on each tick. The set is tiny
/// (one row per running remote `chan serve`) and 500 ms is well
/// below any perceptual threshold for "the drive appeared". Promote
/// to a notify channel only if this shows up in a profile.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

struct TenantListener {
    port: u16,
    cancel: CancellationToken,
}

/// Live, per-listening-session state. Created on `start_listening`
/// and dropped on `stop_listening`; the cancel token cascades to the
/// tunnel listener task, the supervisor, and (indirectly) every
/// per-tenant listener.
struct ActiveRun {
    cancel: CancellationToken,
}

pub struct TunnelState {
    registry: Arc<Registry>,
    listeners: Mutex<HashMap<String, TenantListener>>,
    /// `Some` while the tunnel listener is bound; `None` otherwise.
    /// Replaced on every Stop/Start cycle so cancel tokens never
    /// outlive their listeners.
    run: Mutex<Option<ActiveRun>>,
    /// Cached snapshot of the currently bound port for the
    /// lock-free `tunnel_port()` accessor (UI polling reads this on
    /// every render). `0` means "not listening".
    bound_port: AtomicU16,
}

impl TunnelState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            registry: Registry::new(),
            listeners: Mutex::new(HashMap::new()),
            run: Mutex::new(None),
            bound_port: AtomicU16::new(0),
        })
    }

    /// Currently-bound tunnel listener port, or `None` if the
    /// listener is not active. Used by the IPC status command.
    pub fn tunnel_port(&self) -> Option<u16> {
        match self.bound_port.load(Ordering::Acquire) {
            0 => None,
            p => Some(p),
        }
    }

    pub fn is_listening(&self) -> bool {
        self.run.lock().unwrap().is_some()
    }

    /// Snapshot every registered tunnel paired with the tenant
    /// listener's URL. A row with empty `url` is a tunnel that has
    /// just registered but whose per-tenant listener hasn't bound
    /// yet; the next supervisor tick fills it.
    pub fn snapshot(&self) -> Vec<TunneledDrive> {
        let listeners = self.listeners.lock().unwrap();
        let mut rows: Vec<TunneledDrive> = self
            .registry
            .list_all()
            .into_iter()
            .map(|t| TunneledDrive {
                label: t.user.to_string(),
                drive: t.drive.to_string(),
                public: t.public,
                peer_addr: t.peer_addr.map(|p| p.to_string()),
                connected_at: t.connected_at.to_rfc3339(),
                url: listeners
                    .get(t.user.as_ref())
                    .map(|l| format!("http://127.0.0.1:{}/{}/", l.port, t.drive))
                    .unwrap_or_default(),
            })
            .collect();
        rows.sort_by(|a, b| (&a.label, &a.drive).cmp(&(&b.label, &b.drive)));
        rows
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TunneledDrive {
    pub label: String,
    pub drive: String,
    pub public: bool,
    pub peer_addr: Option<String>,
    pub connected_at: String,
    pub url: String,
}

/// Start the tunnel listener at `preferred_port` (0 = OS-assigned)
/// and the per-tenant supervisor. Returns the actually-bound port.
/// Errors propagate to the IPC caller so the UI can show "port in
/// use" or similar to the user.
///
/// No-op (idempotent) if the listener is already running, returning
/// the existing port. Toggling On while already on must not crash.
pub async fn start_listening(
    app: AppHandle,
    state: Arc<TunnelState>,
    preferred_port: u16,
) -> Result<u16, String> {
    if let Some(p) = state.tunnel_port() {
        return Ok(p);
    }

    let listener = TcpListener::bind(("127.0.0.1", preferred_port))
        .await
        .map_err(|e| format!("binding 127.0.0.1:{preferred_port}: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("reading local_addr: {e}"))?
        .port();

    let cancel = CancellationToken::new();
    {
        let mut run = state.run.lock().unwrap();
        *run = Some(ActiveRun {
            cancel: cancel.clone(),
        });
    }
    state.bound_port.store(port, Ordering::Release);
    tracing::info!(port, "tunnel listener bound on 127.0.0.1");

    let validator: Arc<dyn Validator> = Arc::new(LocalValidator);
    let registry_for_listener = state.registry.clone();
    let cancel_listener = cancel.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_listener.cancelled() => {}
            res = serve_tunnel_listener(
                listener,
                validator,
                registry_for_listener,
                /* max_drives_per_user = unlimited */ 0,
            ) => {
                if let Err(e) = res {
                    tracing::warn!(error = %e, "tunnel listener accept loop exited");
                }
            }
        }
    });

    let app_for_super = app.clone();
    let state_for_super = state.clone();
    let cancel_super = cancel.clone();
    tokio::spawn(async move { supervisor(app_for_super, state_for_super, cancel_super).await });

    let _ = app.emit(
        TUNNEL_STATE_CHANGED,
        serde_json::json!({ "listening": true, "port": port }),
    );
    Ok(port)
}

/// Tear down the tunnel listener, the supervisor, every per-tenant
/// listener, and clear the registry by dropping its handles
/// indirectly (yamux connections close when their drivers exit).
/// Idempotent: a no-op when nothing is listening.
pub fn stop_listening(app: &AppHandle, state: &Arc<TunnelState>) {
    let run = state.run.lock().unwrap().take();
    if let Some(run) = run {
        run.cancel.cancel();
    }
    {
        let mut listeners = state.listeners.lock().unwrap();
        for (_, l) in listeners.drain() {
            l.cancel.cancel();
        }
    }
    state.bound_port.store(0, Ordering::Release);
    // Tear down every tunneled drive webview. The per-tenant
    // listeners are already cancelled above, so any open window
    // would either show a connection error on its next request or
    // sit on a cached page; neither is useful, and Stop is the
    // user explicitly retiring all tunneled state.
    crate::serve::close_all_tunneled_drive_windows(app);
    let _ = app.emit(
        TUNNEL_STATE_CHANGED,
        serde_json::json!({ "listening": false, "port": serde_json::Value::Null }),
    );
}

/// App-exit hook. Same effect as `stop_listening` but does not emit
/// the state-changed event (the frontend is on its way out).
pub fn shutdown(state: &Arc<TunnelState>) {
    let run = state.run.lock().unwrap().take();
    if let Some(run) = run {
        run.cancel.cancel();
    }
    let mut listeners = state.listeners.lock().unwrap();
    for (_, l) in listeners.drain() {
        l.cancel.cancel();
    }
    state.bound_port.store(0, Ordering::Release);
}

async fn supervisor(app: AppHandle, state: Arc<TunnelState>, cancel: CancellationToken) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Tracked separately from `listeners` so we also emit
    // `serves-changed` when new drives appear under an existing
    // label (no listener change, but the snapshot did).
    let mut last_pairs: HashSet<(String, String)> = HashSet::new();
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {}
        }

        let all = state.registry.list_all();
        let live_labels: HashSet<String> = all.iter().map(|t| t.user.to_string()).collect();
        let live_pairs: HashSet<(String, String)> = all
            .iter()
            .map(|t| (t.user.to_string(), t.drive.to_string()))
            .collect();

        let mut changed = false;

        let to_start: Vec<String> = {
            let listeners = state.listeners.lock().unwrap();
            live_labels
                .iter()
                .filter(|l| !listeners.contains_key(l.as_str()))
                .cloned()
                .collect()
        };
        for label in to_start {
            match public::spawn_tenant_listener(label.clone(), state.registry.clone()).await {
                Ok((port, cancel)) => {
                    state
                        .listeners
                        .lock()
                        .unwrap()
                        .insert(label.clone(), TenantListener { port, cancel });
                    tracing::info!(label = %label, port, "per-tenant listener up");
                    changed = true;
                }
                Err(e) => {
                    tracing::warn!(label = %label, error = %e, "tenant listener bind failed");
                }
            }
        }

        let to_stop: Vec<String> = {
            let listeners = state.listeners.lock().unwrap();
            listeners
                .keys()
                .filter(|l| !live_labels.contains(l.as_str()))
                .cloned()
                .collect()
        };
        if !to_stop.is_empty() {
            let mut listeners = state.listeners.lock().unwrap();
            for label in to_stop {
                if let Some(l) = listeners.remove(&label) {
                    l.cancel.cancel();
                    tracing::info!(label = %label, "per-tenant listener down");
                    changed = true;
                }
            }
        }

        if live_pairs != last_pairs {
            // Auto-launch every drive that wasn't present last
            // tick: the user already opted in by clicking Listen,
            // and the supervisor doesn't see the registration until
            // the per-tenant listener has bound a port (so the URL
            // we open is immediately reachable).
            let newly_added: Vec<(String, String)> =
                live_pairs.difference(&last_pairs).cloned().collect();
            if !newly_added.is_empty() {
                // Snapshot the listener map: we drop the lock
                // before spawning windows (run_on_main_thread can
                // block briefly waiting for the main thread).
                let urls: Vec<(String, String, String)> = {
                    let listeners = state.listeners.lock().unwrap();
                    newly_added
                        .into_iter()
                        .filter_map(|(label, drive)| {
                            let port = listeners.get(label.as_str())?.port;
                            let url = format!("http://127.0.0.1:{port}/{drive}/");
                            Some((label, drive, url))
                        })
                        .collect()
                };
                for (label, drive, url) in urls {
                    // Open the same kind of Tauri webview window
                    // we give local drives — same key-bridge JS,
                    // same zoom polyfill, same drag-drop handling.
                    // Each Launch click also spawns a fresh window;
                    // this first-registration open is just one of
                    // them. Closing a window never affects the
                    // remote chan-serve lifecycle.
                    crate::serve::spawn_tunneled_drive_window(&app, &label, &drive, &url);
                    // Still emit the event so the drive table /
                    // header chip can react (refresh, badge, etc.)
                    // without parsing logs.
                    let _ = app.emit(
                        TUNNELED_DRIVE_READY,
                        serde_json::json!({
                            "label": label,
                            "drive": drive,
                            "url": url,
                        }),
                    );
                }
            }
            // Pairs that disappeared between ticks: the remote
            // disconnected (yamux close, registry eviction, etc.).
            // Close every webview window the user had open for that
            // drive — keeping a stale window around would just show
            // a "tunnel disconnected" error once the per-tenant
            // listener tears down too.
            let removed: Vec<(String, String)> =
                last_pairs.difference(&live_pairs).cloned().collect();
            for (label, drive) in removed {
                crate::serve::close_tunneled_drive_windows(&app, &label, &drive);
            }
            last_pairs = live_pairs;
            changed = true;
        }

        if changed {
            let _ = app.emit("serves-changed", ());
        }
    }
}
