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
    /// the `tunnel_guard::settings_guard` middleware reads it to
    /// refuse the settings-write routes server-side.
    pub settings_disabled: bool,
    /// Snapshot of `ServeConfig::tunnel_public`. Stricter than
    /// `settings_disabled`: only true on `--tunnel-public` runs
    /// where the gateway is NOT authenticating the viewer. Read by:
    ///
    ///   - `tunnel_guard::tunnel_public_guard`: refuses
    ///     `POST /api/llm/complete` so an anonymous visitor cannot
    ///     spend the owner's LLM tokens;
    ///   - the read-only handlers that would otherwise leak host
    ///     state (`api_get_drive`, `api_get_config`,
    ///     `api_cloud_drives`, `api_llm_status`): they strip
    ///     absolute paths, the drive registry, and assistant
    ///     readiness signals before serializing.
    pub tunnel_public: bool,
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

#[cfg(test)]
pub(crate) mod test_support {
    //! Minimal `AppState` builder for tests that exercise the
    //! middleware / handlers but don't need a real drive on disk.
    //! The `drive_cell` is intentionally left `None`: callers that
    //! try to reach into it will hit the `drive_cell missing` panic
    //! from `AppState::drive()`, which is the right failure mode
    //! (the test isn't supposed to touch the drive).
    //!
    //! The `Library` is opened against a tempfile so that
    //! `list_drives` returns an empty Vec and registry writes don't
    //! pollute the developer's `~/.chan/config.toml`.

    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Arc, Mutex, RwLock};

    use chan_drive::Library;
    use chan_llm::LlmConfig;
    use tokio::sync::{broadcast, watch};

    use super::AppState;
    use crate::self_writes::SelfWrites;
    use crate::{EditorPrefs, ServerConfig};

    /// Build an `AppState` with the two policy bools set to the
    /// requested values and everything else stubbed to defaults.
    /// The returned `AppState` is safe to wrap in `Arc` and hand to
    /// axum extractors; reading any drive-bearing field will panic
    /// (by design).
    pub fn make_test_state(settings_disabled: bool, tunnel_public: bool) -> Arc<AppState> {
        // The TempDir's path is what Library::open_at uses for any
        // later registry writes (register_drive, set_default_drive_root,
        // ...). Letting it drop here would delete the directory and
        // make those writes fail with ENOENT, which is a subtle
        // footgun for any future test that uses make_test_state and
        // mutates the registry. Leak the guard so the directory
        // outlives the test process: cheap (`#[cfg(test)]` only,
        // the process exits in seconds), avoids the footgun, and is
        // simpler than threading a lifetime through AppState.
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = Library::open_at(tmp.path().join("config.toml")).expect("open library");
        std::mem::forget(tmp);
        let (events_tx, _) = broadcast::channel::<String>(1);
        let (index_events_tx, _) = broadcast::channel::<chan_drive::WatchEvent>(1);
        // A never-tripped shutdown channel: tests don't run the
        // signal watcher, so the receiver stays parked on the
        // initial `false` value for the lifetime of the AppState.
        // Sender is leaked so the rx isn't seen as closed.
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        std::mem::forget(shutdown_tx);
        Arc::new(AppState {
            library: lib,
            drive_root: PathBuf::from("/dev/null"),
            drive_cell: Arc::new(RwLock::new(None)),
            token: None,
            prefix: Arc::new(RwLock::new(String::new())),
            settings_disabled,
            tunnel_public,
            events_tx,
            index_events_tx,
            llm_config: Arc::new(Mutex::new(LlmConfig::default())),
            server_config: Mutex::new(ServerConfig::default()),
            editor_prefs: Mutex::new(EditorPrefs::default()),
            self_writes: Arc::new(SelfWrites::new()),
            last_activity: Arc::new(AtomicU64::new(0)),
            mcp_socket_path: None,
            shutdown_rx,
        })
    }
}
