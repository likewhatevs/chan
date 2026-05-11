//! Server-wide state shared across handlers.
//!
//! `AppState` is the immutable boot bundle every route reaches into.
//! `DriveCell` wraps the live `Arc<Drive>` plus its watcher and indexer
//! so `/api/storage/reset` can swap them wholesale without restarting
//! the process.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, RwLock};

use chan_drive::{Drive, Library, WatchEvent, WatchHandle};
use chan_llm::LlmConfig;
use tokio::sync::{broadcast, watch};

use crate::indexer;
use crate::self_writes::SelfWrites;
use crate::{EditorPrefs, ServerConfig};

/// Server state shared across all handlers.
pub struct AppState {
    pub library: Library,
    /// Drive root resolved at boot. Stays stable for the server's
    /// lifetime even when `drive_cell` is swapped during a reset
    /// (the swap reopens against the same root).
    pub drive_root: PathBuf,
    /// Live drive + its watcher, behind an RwLock so /api/storage/
    /// reset can drop and reopen them without restarting the
    /// process. Always `Some` outside the brief swap window inside
    /// reset itself; handlers reach the inner Arc<Drive> via
    /// `state.drive()` which clones it under a read lock.
    pub drive_cell: Arc<RwLock<Option<DriveCell>>>,
    pub token: Option<String>,
    /// Canonical URL prefix the SPA prepends to fetch and WebSocket
    /// URLs, injected into the shell as `<meta name="chan-prefix">`.
    /// Mutable so tunnel mode can swap in the registration prefix
    /// (`/{user}/{drive}`) on Connected; the local-serve path sets
    /// it once at build time from `ServeConfig::prefix` and never
    /// touches it again. Empty when no prefix.
    ///
    /// Note: this is the SPA-facing prefix only; the axum router is
    /// already nested under `ServeConfig::prefix` at build time, so
    /// changing this value does not re-route handlers. In tunnel
    /// mode the public gateway strips the prefix before forwarding,
    /// which is why the router stays mounted at root.
    pub prefix: Arc<RwLock<String>>,
    /// Snapshot of `ServeConfig::settings_disabled`. Immutable for
    /// the server's lifetime: hardcoded to true on every tunnel run,
    /// always false on local serve. `serve_static` reads it to
    /// inject the `<meta name="chan-settings-disabled">` tag, and
    /// the settings-area write handlers consult it via
    /// `error::err_settings_locked` so the API can't be poked
    /// around the greyed-out button. Read-side endpoints are left
    /// open so the UI can still populate values in view mode.
    pub settings_disabled: bool,
    /// Last activity timestamp (unix seconds). Bumped by HTTP
    /// middleware on every request, by `ws_upgrade` on connect,
    /// and by `ws_pump` on every successful frame send. The idle
    /// watcher task compares this against `now` to decide when to
    /// trigger a graceful shutdown. Always present; the watcher
    /// task only runs when `--timeout` is set.
    pub last_activity: Arc<AtomicU64>,
    /// Pre-serialized JSON-envelope frames: `{"type": "watch",
    /// "event": ...}`, `{"type": "llm.delta", "session_id": ...,
    /// "text": ...}`, etc. One channel; the `type` field tells
    /// the frontend what to do.
    pub events_tx: broadcast::Sender<String>,
    /// Raw watcher events feeding the background indexer. Lives at
    /// AppState scope (not just inside DriveCell) so the bridge
    /// constructor at /api/storage/reset time can reuse the same
    /// channel without resubscribing the indexer to a fresh one.
    pub index_events_tx: broadcast::Sender<WatchEvent>,
    /// Loaded at boot; mutable for future PATCH /api/llm/config
    /// (backend selection, auto_apply_writes toggle). Currently
    /// only read by the status route and the complete handler.
    pub llm_config: Arc<Mutex<LlmConfig>>,
    /// chan-server's own preferences (attachments_dir,
    /// answers_dir, etc). Mutable via PATCH /api/server/config;
    /// reads route through the get handler.
    pub server_config: Mutex<ServerConfig>,
    /// Editor preferences: fonts / theme / pane widths / line
    /// spacing / date format. Persisted to
    /// `<config>/chan/preferences.toml`; mutated through the
    /// /api/config PATCH path.
    pub editor_prefs: Mutex<EditorPrefs>,
    /// Recently-written paths for the watcher dedupe. Every server-
    /// side write notes its target here; WatchBroadcast checks the
    /// queue before forwarding so an editor save doesn't bounce
    /// back as an "external edit" event.
    pub self_writes: Arc<SelfWrites>,
    /// Path to the Unix-domain socket where the in-process MCP
    /// server is exposed for agent subprocesses (claude / gemini).
    /// `None` when the bridge couldn't bind (read-only tmpdir,
    /// exotic platforms); the agent backends fall back to v1
    /// black-box mode in that case. The bridge handle that owns
    /// the socket file lives on `AppArtifacts` so it gets dropped
    /// (and the file unlinked) when serve() unwinds.
    pub mcp_socket_path: Option<PathBuf>,
    /// Process-wide shutdown signal. Fires once SIGINT/SIGTERM or
    /// the idle-timeout watcher trip. Long-lived handlers (e.g.
    /// `/ws`) observe this to close their sockets promptly so axum's
    /// graceful drain returns in milliseconds instead of holding
    /// open until the hard deadline.
    pub shutdown_rx: watch::Receiver<bool>,
}

/// Drive + its notify watcher. Replaced wholesale by /api/storage/
/// reset: drop the cell, run chan-drive's reset_drive, reopen, store
/// a fresh cell. The watch_handle is `Option` only because reset
/// must take it out before dropping the inner Drive (the watcher
/// holds a callback that references the same broadcast channel; we
/// keep it tidy by dropping the handle first).
pub struct DriveCell {
    pub drive: Arc<Drive>,
    pub watch_handle: Option<WatchHandle>,
    /// Background indexer for the live drive. Replaced wholesale
    /// on /api/storage/reset (the new drive needs a fresh indexer
    /// pinned to its `Arc<Drive>`). Drop = abort = workers stop.
    pub indexer: Arc<indexer::Indexer>,
}

impl AppState {
    /// Snapshot the current drive Arc. Acquires the RwLock read
    /// guard for the duration of the clone (microseconds). The
    /// returned Arc keeps the drive alive even if a reset swaps
    /// the cell out a moment later, so callers don't need to hold
    /// the lock through their I/O.
    ///
    /// Panics if called while the cell is in the brief
    /// "between drop and reopen" state inside reset itself; the
    /// reset path holds the write lock end-to-end so handlers can
    /// never observe `None` (they wait on the read lock).
    pub fn drive(&self) -> Arc<Drive> {
        self.drive_cell
            .read()
            .expect("drive cell poisoned")
            .as_ref()
            .expect("drive cell missing outside reset window")
            .drive
            .clone()
    }

    /// Snapshot the live indexer Arc. Same RwLock pattern as
    /// `drive()`: held only for the duration of the clone.
    pub fn indexer(&self) -> Arc<indexer::Indexer> {
        self.drive_cell
            .read()
            .expect("drive cell poisoned")
            .as_ref()
            .expect("drive cell missing outside reset window")
            .indexer
            .clone()
    }
}
