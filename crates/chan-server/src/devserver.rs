//! Headless multi-workspace devserver.
//!
//! `run_devserver` binds a [`WorkspaceHost`] to a real address and adds two
//! surfaces a desktop client and the `chan serve` CLI drive over it:
//!
//! - A management HTTP/JSON API under the reserved `/api/devserver/*`
//!   namespace ([`crate::devserver_api`]): list, mount, forget workspaces
//!   and open standalone terminals. Workspace tenants mount at their PUBLIC
//!   slug `/{slug}` (top-level), so the gateway forwards
//!   `{user}.devserver.chan.app/{slug}/` unchanged and the devserver routes
//!   the tenant by it; the explicit `/api/devserver/*` and `/api/library/*`
//!   management routes match before the per-tenant fallback, and the only
//!   reserved top-level slug is `api`.
//! - A per-user Unix discovery socket ([`crate::devserver_handoff`]): a
//!   `chan serve <path>` on the same box registers its workspace with the
//!   running devserver and exits instead of binding its own server, so the
//!   devserver owns the single-writer flock.
//!
//! What was mounted survives a restart: the enabled workspace roots and the
//! devserver bearer token persist in `~/.chan/devserver/config.json` (0600).
//! Per-window pane/tab layout is NOT persisted here; each tenant is a full
//! workspace mount that already stores its own SPA session per window, so a
//! reconnecting client re-hydrates its panes from the tenant. Terminal PTY
//! contents reset (PTYs are fresh processes).

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, Request as HttpRequest, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use chan_workspace::Library;
use serde::{Deserialize, Serialize};

use crate::auth::random_token;
use crate::devserver_api::{
    ActiveTerminalsRejection, DevserverInfo, DevserverWindow, MountedPrefix, OpenWorkspaceRequest,
    SetWorkspaceOnRequest, WorkspaceEntry, DEVSERVER_API_PROTOCOL,
};
use crate::{Error, ServeConfig, WorkspaceHost};
// Prefix allocation lives in chan-library (the window-record assembly needs the
// stable OFF-workspace prefix); the devserver mounts at the same prefix.
use chan_library::windows::WindowRegistry;
use chan_library::{allocate_workspace_prefix, PersistedWorkspace, WorkspaceOverlay};

/// Inputs the CLI resolves for `chan devserver`. The `--systemd`
/// supervision path is layered on in the CLI around this; the runtime
/// itself only needs where to bind, how to label the box, and whether to
/// also dial the gateway tunnel.
pub struct DevserverConfig {
    /// Address to bind the public HTTP listener.
    pub addr: SocketAddr,
    /// Human label for the box (drives the client's grouping header).
    pub host_label: String,
    /// When set, the devserver also dials the gateway and publishes its
    /// tenant content at `{user}.devserver.chan.app/{workspace}/*`. `None`
    /// leaves it local-only (management API + discovery socket on `addr`).
    pub tunnel: Option<DevserverTunnel>,
}

/// Gateway tunnel registration for a devserver. The devserver identity is
/// resolved backend-side from the token (PAT SHA-256), so there is no name to
/// supply; the whole library rides one registration.
#[derive(Debug, Clone)]
pub struct DevserverTunnel {
    /// Tunnel endpoint URL (default `https://devserver.chan.app/v1/tunnel`).
    pub tunnel_url: String,
    /// Personal access token (`chan_pat_*`) from id.chan.app.
    pub token: String,
}

/// On-disk devserver state: the bearer token (minted once and reused so a
/// reconnecting client keeps working across restarts) and the stable library
/// identity. Workspace on/off lives in the library-owned [`WorkspaceOverlay`]
/// store (`~/.chan/devserver/workspaces.json`), not here.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedConfig {
    #[serde(default)]
    devserver_token: String,
    /// This library's stable identity, minted once (`lib-<16hex>`) and persisted
    /// so it survives restart. Stamped on every window record (Seam W); a client
    /// merging several libraries' feeds partitions by it.
    #[serde(default)]
    library_id: String,
}

/// Persistence at `~/.chan/devserver/config.json`, written atomically and
/// locked 0600 since it holds the bearer token.
struct DevserverStore {
    path: PathBuf,
}

impl DevserverStore {
    fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Read the persisted config, or a default when the file is absent or
    /// unreadable. An unreadable file degrades to a fresh token + empty set
    /// rather than refusing to start.
    fn load(&self) -> PersistedConfig {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => PersistedConfig::default(),
        }
    }

    fn save(&self, cfg: &PersistedConfig) -> std::io::Result<()> {
        let dir = match self.path.parent() {
            Some(dir) => {
                std::fs::create_dir_all(dir)?;
                dir
            }
            None => Path::new("."),
        };
        let bytes = serde_json::to_vec_pretty(cfg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = self.path.with_extension("json.tmp");
        // Write + fsync the tmp so its bytes are durable BEFORE the rename:
        // renaming un-synced data is exactly the partial-config risk on a
        // crash or power loss.
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }
        // 0600 on the tmp, before the rename, so the token file is never
        // visible at its final path with looser permissions.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &self.path)?;
        // fsync the parent directory so the new dirent survives a crash too;
        // POSIX permits the rename to be lost otherwise. Matches the
        // gold-standard `atomic_write`. Best-effort: durability hardening, not
        // a reason to fail a save the rename already committed.
        let _ = chan_workspace::fs_ops::sync_dir(dir);
        Ok(())
    }
}

/// The session store for the shared standalone-terminal tenant:
/// `~/.chan/devserver/terminals/`. Each terminal window's per-window pane/tab
/// layout blob is keyed by its `?w=<window_id>` here, so the layout survives a
/// devserver restart (with fresh PTYs). `None` when there is no home dir (the
/// tenant then falls back to the in-memory `ephemeral_sessions`).
fn devserver_terminals_dir() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".chan")
            .join("devserver")
            .join("terminals"),
    )
}

fn devserver_config_path() -> std::io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home directory"))?;
    Ok(home.join(".chan").join("devserver").join("config.json"))
}

/// Machine-readable marker the desktop control terminal scrapes from the
/// connect-script output to learn the devserver's bearer token, on every
/// connect and reconnect; the token value runs from the `=` to end of line.
/// LOCKED wire string: the desktop matches this exact prefix, so both the
/// foreground emit and the `--systemd` re-attach emit build to it.
pub const DEVSERVER_TOKEN_MARKER: &str = "CHAN_DEVSERVER_TOKEN=";

/// Read the persisted devserver bearer token from
/// `~/.chan/devserver/config.json`, or `None` when it is absent, unreadable,
/// or tokenless. The `--systemd` re-attach path prints the
/// [`DEVSERVER_TOKEN_MARKER`] from this, since a journal-follow does not
/// re-emit the running unit's original start line.
pub fn persisted_devserver_token() -> Option<String> {
    let store = DevserverStore::at(devserver_config_path().ok()?);
    let token = store.load().devserver_token;
    (!token.is_empty()).then_some(token)
}

/// A registered workspace as the devserver tracks it, the source of truth for
/// `GET /api/devserver/workspaces`. Keyed by its stable `prefix` in
/// [`DevserverState`]. Either mounted (`on`, live in the host, carrying a
/// per-mount `token`) or registered-but-unmounted (`!on`, absent from the
/// host, empty `token`). Forget drops it entirely.
struct WorkspaceRecord {
    root: PathBuf,
    prefix: String,
    label: String,
    /// Whether the workspace is mounted in the host right now.
    on: bool,
    /// Per-mount bearer token while `on`; empty while off.
    token: String,
}

/// Shared runtime state behind the management API and the discovery socket.
struct DevserverState {
    host: Arc<WorkspaceHost>,
    addr: SocketAddr,
    /// Devserver-level bearer token, distinct from per-workspace tokens.
    token: String,
    /// This library's stable identity (`lib-<16hex>`), persisted with the token.
    library_id: String,
    host_label: String,
    /// Registered workspaces by stable prefix, on and off.
    workspaces: Mutex<HashMap<String, WorkspaceRecord>>,
    store: DevserverStore,
}

impl DevserverState {
    /// Register the workspace at `root` and mount it (on). Allocates the
    /// stable prefix, mounts via [`mount_at`](Self::mount_at), persists, and
    /// returns the prefix. Idempotent on the root (an already-mounted root
    /// returns its existing prefix). Used by `POST workspaces` and the
    /// discovery socket; `POST .../{prefix}/on` is the explicit-toggle sibling.
    async fn register_workspace(&self, root: &Path) -> Result<String, Error> {
        let prefix = allocate_workspace_prefix(root)?;
        let mounted = self.mount_at(root, &prefix).await?;
        self.persist_state();
        Ok(mounted)
    }

    /// Mount `root` at `prefix` in the host and record it as on. The host
    /// opens through the shared `Library`, which requires the root to be
    /// registered first; registering an already-known root is a no-op. Does
    /// NOT persist — callers batch the save. Returns the prefix actually
    /// mounted at (the host's idempotent re-register can return an existing
    /// prefix; for a fresh mount it is `prefix`).
    ///
    /// Rejects a `prefix` that collides with the reserved `/api/` namespace.
    /// The host's own collision guard rejects a `prefix` already taken by a
    /// DIFFERENT root (two workspaces with the same basename slug), surfacing
    /// the design's "slug uniqueness within a devserver".
    async fn mount_at(&self, root: &Path, prefix: &str) -> Result<String, Error> {
        if prefix == RESERVED_WORKSPACE_PREFIX {
            return Err(Error::Config(format!(
                "cannot mount a workspace at {prefix}: that path is reserved for the devserver \
                 management API (/api/*). Rename the workspace directory; its basename becomes \
                 the public slug."
            )));
        }
        self.host.library().register_workspace(root)?;
        let hosted = self
            .host
            .open_or_get_registered_workspace(root, tenant_config(self.addr, prefix))
            .await?;
        let record = WorkspaceRecord {
            root: hosted.root.clone(),
            prefix: hosted.prefix.clone(),
            label: workspace_label(&hosted.root),
            on: true,
            // Tenants are configured with `no_token: false`, so the handle
            // always carries a token; default to empty rather than panic.
            token: hosted.handle.token.clone().unwrap_or_default(),
        };
        let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
        workspaces.insert(hosted.prefix.clone(), record);
        Ok(hosted.prefix)
    }

    /// Track `root` at `prefix` as registered-but-off (remembered, not
    /// mounted, no token). Re-surfaces an off row on restart and is the off
    /// side of a toggle. Does NOT persist — callers batch the save.
    fn track_off(&self, root: &Path, prefix: &str) {
        let record = WorkspaceRecord {
            root: root.to_path_buf(),
            prefix: prefix.to_string(),
            label: workspace_label(root),
            on: false,
            token: String::new(),
        };
        let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
        workspaces.insert(prefix.to_string(), record);
    }

    /// Set whether the registered workspace at `prefix` is mounted, returning
    /// the updated row (`None` ⇒ no workspace registered there ⇒ the handler
    /// answers 404). `on:false` unmounts (releasing the per-workspace flock)
    /// but keeps the registration with an empty token; `on:true` remounts at
    /// the SAME prefix with a freshly-minted token. Idempotent in both
    /// directions. Distinct from Forget, which drops the registration.
    async fn set_workspace_on(
        &self,
        prefix: &str,
        on: bool,
    ) -> Result<Option<WorkspaceEntry>, Error> {
        // Snapshot under the lock, then release it before the mount/unmount:
        // `close_workspace` blocks on the bounded flock wait and the remount
        // awaits, and the list endpoint must stay responsive meanwhile.
        let current = {
            let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces
                .get(prefix)
                .map(|record| (record.on, record.root.clone()))
        };
        let (currently_on, root) = match current {
            Some(current) => current,
            // Not in the serving map: it may be a host-library workspace the
            // devserver has not mounted yet (D1 — every library workspace is
            // listable + toggleable). Resolve the prefix back to its library
            // root and treat it as currently off; an unknown prefix is a 404.
            None => match self.library_root_for_prefix(prefix) {
                Some(root) => (false, root),
                None => return Ok(None),
            },
        };
        if currently_on == on {
            // Already in the requested state: idempotent no-op, current row.
            // A library-only off row has no map entry, so synthesize it.
            return Ok(self
                .entry_for(prefix)
                .or_else(|| self.library_off_entry(prefix)));
        }
        if on {
            // Off → on: mount at the SAME (stable) prefix, minting a fresh
            // token. Works for a registered-off row AND a never-served library
            // workspace (mount_at registers it in the library + host).
            self.mount_at(&root, prefix).await?;
        } else {
            // On → off: unmount, release the flock, keep the registration.
            self.host.close_workspace(prefix)?;
            let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(record) = workspaces.get_mut(prefix) {
                record.on = false;
                record.token.clear();
            }
        }
        self.persist_state();
        Ok(self
            .entry_for(prefix)
            .or_else(|| self.library_off_entry(prefix)))
    }

    /// The current [`WorkspaceEntry`] for `prefix`, or `None` when no
    /// workspace is registered there.
    fn entry_for(&self, prefix: &str) -> Option<WorkspaceEntry> {
        let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
        workspaces.get(prefix).map(entry_from_record)
    }

    /// Forget the workspace at `prefix`: unmount it if on, then drop the
    /// registration entirely. Returns whether a registered workspace existed
    /// there — existence, NOT whether the host had it mounted, is the
    /// "removed" signal, since an off row is registered-but-unmounted and
    /// forgetting it must still report success. Distinct from on/off.
    fn forget_workspace(&self, prefix: &str) -> Result<bool, Error> {
        // DESTRUCTIVE under D1 (Seam B Amendment 6) — the devserver Forget is
        // `chan workspace rm`: unmount-if-running, then UNREGISTER from the host
        // library (reset Everything + bin the trash). The host library is the
        // single registry, so the workspace then disappears everywhere
        // (library, devserver listing, CLI). `set_workspace_on {on:false}` is
        // the reversible unmount; this is the removal. Resolve the root from the
        // serving record OR, for a library workspace not currently served, the
        // library itself — every library workspace is forgettable.
        let root = {
            let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.get(prefix).map(|record| record.root.clone())
        }
        .or_else(|| self.library_root_for_prefix(prefix));
        let Some(root) = root else {
            return Ok(false);
        };
        // Unmount if mounted (releases the per-workspace flock before the reset);
        // a no-op when the row is off or library-only (not in the host).
        let _ = self.host.close_workspace(prefix);
        {
            let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.remove(prefix);
        }
        // Remove from the host library: reset Everything + bin the workspace
        // trash (@@Alex "bin it!!"). Unmount above dropped the tenant's handle,
        // so the writer lock is released before the reset.
        self.host.library().unregister_workspace(&root)?;
        self.persist_state();
        Ok(true)
    }

    /// Persist devserver state across two stores: workspace on/off into the
    /// library-owned [`WorkspaceOverlay`], and the bearer token + library id into
    /// the devserver config. So a restart comes back serving exactly what was on
    /// and remembering what was off.
    fn persist_state(&self) {
        // Workspace on/off → the library-owned overlay store. The overlay sorts
        // by path on save for a stable file.
        if let Some(overlay) = self.host.workspace_overlay() {
            let rows: Vec<PersistedWorkspace> = {
                let map = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
                map.values()
                    .map(|record| PersistedWorkspace {
                        path: record.root.to_string_lossy().into_owned(),
                        on: record.on,
                    })
                    .collect()
            };
            overlay.replace(rows);
        }
        // Bearer token + library identity → the devserver config.
        let cfg = PersistedConfig {
            devserver_token: self.token.clone(),
            library_id: self.library_id.clone(),
        };
        if let Err(e) = self.store.save(&cfg) {
            tracing::warn!("persisting devserver config: {e}");
        }
    }

    /// The box's workspace list for `GET /api/devserver/workspaces`: ONE row
    /// per HOST-LIBRARY workspace (the set `chan workspace ls` shows, read live
    /// from the registry), with `on`/`prefix`/`token` from the devserver's
    /// serving state. The host library — not the devserver's own config — is
    /// the source of truth (D1): a freshly-started devserver therefore lists
    /// exactly what `chan list` shows instead of coming up empty. A library
    /// workspace the devserver is not serving is `on:false` at its stable
    /// derived prefix with no token; toggling it on mounts it (see
    /// [`set_workspace_on`](Self::set_workspace_on)). Sorted by prefix.
    fn workspace_entries(&self) -> Vec<WorkspaceEntry> {
        let by_root: HashMap<PathBuf, WorkspaceEntry> = {
            let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces
                .values()
                .map(|record| (record.root.clone(), entry_from_record(record)))
                .collect()
        };
        let mut entries: Vec<WorkspaceEntry> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for ws in self.host.library().list_workspaces() {
            seen.insert(ws.root_path.clone());
            if let Some(entry) = by_root.get(&ws.root_path) {
                entries.push(entry.clone());
            } else if let Ok(prefix) = allocate_workspace_prefix(&ws.root_path) {
                entries.push(WorkspaceEntry {
                    prefix,
                    path: ws.root_path.to_string_lossy().into_owned(),
                    label: workspace_label(&ws.root_path),
                    on: false,
                    token: String::new(),
                });
            }
        }
        // Defensive: a served workspace whose root left the library (forgotten
        // while still mounted) must still surface so a live mount never
        // silently vanishes from the list.
        for (root, entry) in &by_root {
            if !seen.contains(root) {
                entries.push(entry.clone());
            }
        }
        entries.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        entries
    }

    /// Resolve a route prefix back to a host-library workspace root for a
    /// prefix that names a library workspace the devserver is NOT serving (so
    /// it is absent from `self.workspaces`). Matches on the stable
    /// [`allocate_workspace_prefix`] mapping.
    fn library_root_for_prefix(&self, prefix: &str) -> Option<PathBuf> {
        self.host
            .library()
            .list_workspaces()
            .into_iter()
            .map(|ws| ws.root_path)
            .find(|root| allocate_workspace_prefix(root).ok().as_deref() == Some(prefix))
    }

    /// The off-state row for a library workspace the devserver is not serving
    /// (stable prefix, no token), for idempotent off-toggles and reporting.
    fn library_off_entry(&self, prefix: &str) -> Option<WorkspaceEntry> {
        let root = self.library_root_for_prefix(prefix)?;
        Some(WorkspaceEntry {
            prefix: prefix.to_string(),
            path: root.to_string_lossy().into_owned(),
            label: workspace_label(&root),
            on: false,
            token: String::new(),
        })
    }

    /// Mount the per-library SHARED terminal tenant (D-W3). `open_terminal_session`
    /// records its prefix in the host's `terminal_tenant_prefix`, which the window
    /// feed's `terminal_window_live` resolves a Terminal record's prefix+token
    /// against. The desktop does this via `embedded.rs`; the devserver never did
    /// (it only ever mounted per-LABEL terminals via the lower-level
    /// `open_terminal_session_with_command`, which does NOT set the OnceLock), so
    /// every devserver Terminal window carried an empty token and the desktop
    /// watcher's `should_show` (which requires a non-empty token) hid it —
    /// vanishing on every reconnect. `Some(dir)` persists each window's pane
    /// layout. One shared tenant per library, so this is called once at startup.
    async fn mount_shared_terminal_tenant(&self) -> Result<(), Error> {
        self.host
            .open_terminal_session(
                tenant_config(self.addr, DEVSERVER_SHARED_TERMINAL_PREFIX),
                devserver_terminals_dir(),
            )
            .await?;
        Ok(())
    }
}

/// Build the wire [`WorkspaceEntry`] for a registered workspace record: an
/// off row reports `on:false` with an empty token; an on row its live token.
fn entry_from_record(record: &WorkspaceRecord) -> WorkspaceEntry {
    WorkspaceEntry {
        prefix: record.prefix.clone(),
        path: record.root.to_string_lossy().into_owned(),
        label: record.label.clone(),
        on: record.on,
        token: record.token.clone(),
    }
}

/// Run the devserver in the foreground until the process is interrupted.
/// Loads (or mints) the persisted token, re-mounts the enabled workspaces,
/// echoes the bind+token line, binds the management + discovery surfaces,
/// and serves.
pub async fn run_devserver(library: Library, config: DevserverConfig) -> anyhow::Result<()> {
    let store =
        DevserverStore::at(devserver_config_path().context("resolving devserver config path")?);
    let mut persisted = store.load();
    if persisted.devserver_token.is_empty() {
        persisted.devserver_token = random_token();
    }
    let token = persisted.devserver_token.clone();
    // Mint a stable per-library id once (`lib-<16hex>`), persisted alongside the
    // token, so it survives restart and stamps every window record (Seam W).
    if persisted.library_id.is_empty() {
        persisted.library_id = format!("lib-{:016x}", rand::random::<u64>());
    }
    let library_id = persisted.library_id.clone();

    let host = Arc::new(WorkspaceHost::new(library, crate::route_builder()));
    // Opt in to control-socket `chan unserve`: a hosted workspace's tenant can
    // then be unmounted by path (it does not kill the multi-tenant process).
    host.install_self();
    // Install the persisted window registry (the Seam-W source of truth) beside
    // the devserver config, so the window feed has data. The window-record
    // assembly reads it; `library_id` stamps each row.
    let windows_store = devserver_config_path()
        .context("resolving devserver windows store path")?
        .with_file_name("windows.json");
    host.install_window_registry(
        Arc::new(WindowRegistry::open(windows_store)),
        library_id.clone(),
    );
    // Install the library-owned workspace on/off overlay beside the window
    // registry, so the restore below re-mounts what was on. Same shape + store
    // the desktop-local library uses (`~/.chan/workspaces.json`).
    let overlay_store = devserver_config_path()
        .context("resolving devserver workspace overlay path")?
        .with_file_name("workspaces.json");
    host.install_workspace_overlay(Arc::new(WorkspaceOverlay::open(overlay_store)));
    let state = Arc::new(DevserverState {
        host: host.clone(),
        addr: config.addr,
        token: token.clone(),
        library_id,
        host_label: config.host_label,
        workspaces: Mutex::new(HashMap::new()),
        store,
    });

    // Mount the per-library SHARED terminal tenant (D-W3) before serving, so
    // devserver Terminal windows resolve to a real prefix+token.
    state
        .mount_shared_terminal_tenant()
        .await
        .context("mounting the devserver shared terminal tenant")?;

    // The library open path: a fresh devserver (empty registry, marker unset)
    // mints exactly one terminal so a plain browser pointed at it sees a window;
    // a devserver whose terminal was closed (marker set) does NOT re-mint on
    // restart. The rule lives in the library, identical to the desktop local
    // boot. Run after mounting the shared terminal tenant so the minted window
    // resolves to a real prefix+token. Persisted-workspace restore follows, so a
    // first boot whose persisted set turns a workspace ON still mints the
    // terminal (the registry was empty at this point) — matching "open spawns one
    // terminal".
    state
        .host
        .ensure_first_open_terminal()
        .context("provisioning the devserver first-open terminal")?;

    // Restore the registered workspaces from the library-owned overlay. The
    // mount prefix is re-derived from the path (the stable slug live mounts and
    // the window feed already use), not persisted. `on` rows re-mount at it;
    // `off` rows are tracked as registered-but-unmounted so the client still
    // sees them and can toggle them on. A root that fails to re-mount is
    // downgraded to off so its row still surfaces; a path that fails to map to a
    // prefix is skipped with a note.
    let restore_rows = state
        .host
        .workspace_overlay()
        .map(|overlay| overlay.entries())
        .unwrap_or_default();
    for ws in &restore_rows {
        let path = PathBuf::from(&ws.path);
        let prefix = match allocate_workspace_prefix(&path) {
            Ok(prefix) => prefix,
            Err(e) => {
                eprintln!(
                    "chan devserver: NOTE: skipping persisted workspace {} ({e})",
                    ws.path
                );
                continue;
            }
        };
        if ws.on {
            if let Err(e) = state.mount_at(&path, &prefix).await {
                eprintln!("chan devserver: NOTE: could not re-mount {}: {e}", ws.path);
                state.track_off(&path, &prefix);
            }
        } else {
            state.track_off(&path, &prefix);
        }
    }
    // Persist once now so a newly-minted token + the restored set (with any
    // failed re-mounts downgraded to off) land even before the first call.
    state.persist_state();

    // Serve-handoff discovery. A bind failure is non-fatal: the management
    // API still works, only the `chan serve` registration path is disabled.
    let _discovery = start_discovery_listener(state.clone());

    let app = build_devserver_app(state.clone(), host.clone());
    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .with_context(|| format!("binding devserver on {}", config.addr))?;
    // Report the bound address, not the requested one, so `--port 0` prints
    // the OS-assigned port (mirrors `chan serve`). Falls back to the request
    // on the impossible local_addr() error rather than refusing to serve.
    let local_addr = listener.local_addr().unwrap_or(config.addr);
    println!("chan devserver: listening on http://{local_addr}");
    // Machine-readable token contract: the desktop control terminal scrapes
    // this exact marker from the connect-script output on every connect and
    // reconnect, as the source of truth for the bearer token. Emitted once the
    // token and bound address are known; the `--systemd` first start surfaces
    // it through the unit journal the launcher follows.
    println!("{DEVSERVER_TOKEN_MARKER}{token}");

    // Shutdown wiring mirrors `serve()`: a single watch channel fed by
    // SIGINT/SIGTERM, plus a side task that cancels every tenant's in-flight
    // reindex so the per-workspace flocks release promptly. `graceful_serve`
    // owns the signal watcher and the hard drain deadline.
    let signal_tx = Arc::new(tokio::sync::watch::channel(false).0);
    let cancel_host = host.clone();
    let mut cancel_rx = signal_tx.subscribe();
    tokio::spawn(async move {
        let _ = cancel_rx.changed().await;
        cancel_host.cancel_all_reindex();
    });

    // Tunnel mode: also hand the SAME app to chan-tunnel-client, which registers
    // ONE devserver and forwards inbound substreams into it, publishing every
    // mounted tenant behind one gateway registration. The management API rides
    // the same router, but the proxy 404s `/api/devserver/*` on the public
    // wildcard, so only tenant content is reachable through the gateway. The
    // run loop reconnects with backoff and is cancelled by the shutdown signal.
    if let Some(tunnel) = config.tunnel {
        spawn_devserver_tunnel(tunnel, app.clone(), &signal_tx);
    }

    crate::signal::graceful_serve(listener, app, signal_tx)
        .await
        .context("running devserver")?;
    Ok(())
}

/// Fixed registration name sent in the tunnel `Hello` frame. The gateway
/// resolves the devserver identity from the token (PAT SHA-256) and ignores
/// this value; it is non-empty only to satisfy the client-side name check
/// (`chan_tunnel_proto::is_valid_workspace_name`). One devserver per user means
/// the registry key `(user, name)` never collides across users.
const DEVSERVER_TUNNEL_NAME: &str = "devserver";

/// True iff the tunnel dial endpoint is the production `devserver.chan.app`
/// terminator. On that path the devserver can name the public host shape
/// (`{user}.devserver.chan.app`); anywhere else (a dev gateway, a staging
/// host) the terminator owns the URL scheme, so the connect log prints
/// identity only.
fn is_production_tunnel_url(tunnel_url: &str) -> bool {
    url::Url::parse(tunnel_url)
        .map(|u| u.scheme() == "https" && u.host_str() == Some("devserver.chan.app"))
        .unwrap_or(false)
}

/// Dial the gateway tunnel on a background task that races the reconnect loop
/// against the shutdown signal. The devserver is headless, so the lifecycle
/// drainer only logs connect / disconnect / dial-failure: no QR, no
/// browser-open, and no SPA prefix swap (each tenant already serves at its own
/// public slug, so the proxy forwards the public path unchanged).
fn spawn_devserver_tunnel(
    tunnel: DevserverTunnel,
    app: Router,
    signal_tx: &Arc<tokio::sync::watch::Sender<bool>>,
) {
    let DevserverTunnel { tunnel_url, token } = tunnel;
    let mut shutdown_rx = signal_tx.subscribe();
    tokio::spawn(async move {
        let url = match url::Url::parse(&tunnel_url) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("chan devserver: invalid --tunnel-url {tunnel_url:?}: {e}");
                return;
            }
        };
        let production = is_production_tunnel_url(&tunnel_url);
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            while let Some(ev) = events_rx.recv().await {
                match ev {
                    chan_tunnel_client::TunnelEvent::Connected(reg) => {
                        if production {
                            eprintln!(
                                "chan devserver: tunnel connected; workspaces are published at \
                                 https://{user}.devserver.chan.app/<workspace>/",
                                user = reg.user,
                            );
                        } else {
                            eprintln!(
                                "chan devserver: tunnel connected as user {user}",
                                user = reg.user,
                            );
                        }
                    }
                    chan_tunnel_client::TunnelEvent::Disconnected { retry_in } => {
                        eprintln!(
                            "chan devserver: tunnel disconnected; reconnecting in {retry_in:?}"
                        );
                    }
                    chan_tunnel_client::TunnelEvent::DialFailed { error, retry_in } => {
                        eprintln!(
                            "chan devserver: tunnel dial failed: {error} (retry in {retry_in:?})"
                        );
                    }
                }
            }
        });
        let cfg = chan_tunnel_client::ClientConfig {
            tunnel_url: url,
            token,
            workspace: DEVSERVER_TUNNEL_NAME.to_string(),
            client_version: format!("chan/{}", env!("CARGO_PKG_VERSION")),
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            dial_timeout: Duration::from_secs(30),
            proxy: None,
            max_concurrent_substreams: chan_tunnel_client::ClientConfig::default()
                .max_concurrent_substreams,
            events: Some(events_tx),
        };
        // Race the run loop against shutdown: dropping the tunnel future closes
        // the yamux session immediately (no axum connection pool to drain).
        tokio::select! {
            res = chan_tunnel_client::run(cfg, app) => {
                if let Err(e) = res {
                    eprintln!("chan devserver: tunnel client exited: {e}");
                }
            }
            _ = shutdown_rx.changed() => {}
        }
    });
}

/// Build the merged router: the unauthenticated info probe, the
/// bearer-gated management routes, and the per-tenant fallback. Explicit
/// `/api/devserver/*` routes match before the host's fallback, so the
/// reserved namespace is never shadowed by a workspace prefix.
fn build_devserver_app(state: Arc<DevserverState>, host: Arc<WorkspaceHost>) -> Router {
    let public = Router::new()
        .route("/api/devserver/info", get(handle_info))
        .with_state(state.clone());
    let authed = Router::new()
        .route(
            "/api/devserver/workspaces",
            get(handle_list).post(handle_open),
        )
        .route(
            "/api/devserver/workspaces/*prefix",
            delete(handle_forget).post(handle_set_workspace_on),
        )
        .route("/api/devserver/windows", get(handle_list_windows))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ))
        .with_state(state);
    // Serve the web-launcher SPA at the library root `/` plus the `/api/library/*`
    // data surface (windows; workspaces next) as the host's root fallback —
    // without it the root 404s, since `host_dispatch` only matches
    // workspace-tenant prefixes. The `/api/library/windows*` routes used to live
    // in `authed` above; they now live in the shared launcher bundle so the
    // desktop loopback gets them too (the loopback never built this router).
    //
    // `bearer = None` (TUNNEL-TRUST) on the gateway surface: the devserver has
    // no inbound ports, and the gateway proxy is the sole auth boundary — it
    // validates the `devserver_gate` at its edge and STRIPS every client
    // credential (`?t=`, Cookie, Authorization) before forwarding, so a
    // launcher XHR reaches `/api/library/*` with zero creds. A `Some(token)`
    // gate would 401 every call. The desktop LOOPBACK passes `Some(token)`
    // instead (it has no proxy; the SPA presents the token via `?t=`).
    //
    // `serve_addr = None`: the gateway surface serves workspaces READ-ONLY.
    // `bearer=None` can't distinguish owner from grantee, so mutating routes
    // (add/on/off/rm) must not be exposed over the tunnel; owners
    // manage a headless devserver's workspaces via the bearer-gated
    // `/api/devserver/*` API + `cs`/CLI instead.
    crate::install_launcher_root_fallback(&host, None, None);
    public.merge(authed).merge(host.router())
}

/// Bind the per-user discovery socket whose registration handler mounts the
/// requested workspace. `None` (and a note) when the socket cannot bind, so
/// the management API still serves.
fn start_discovery_listener(
    state: Arc<DevserverState>,
) -> Option<crate::devserver_handoff::ListenerHandle> {
    let socket_path = crate::devserver_handoff::well_known_devserver_socket_path()?;
    let result = crate::devserver_handoff::start_listener(socket_path, move |req| {
        let state = state.clone();
        async move {
            match req {
                crate::devserver_handoff::Request::RegisterWorkspace { workspace_path, .. } => {
                    match state.register_workspace(Path::new(&workspace_path)).await {
                        Ok(prefix) => crate::devserver_handoff::Response::Registered {
                            devserver_version: crate::devserver_handoff::CHAN_VERSION.to_string(),
                            prefix,
                        },
                        Err(e) => crate::devserver_handoff::Response::Error {
                            message: e.to_string(),
                        },
                    }
                }
            }
        }
    });
    match result {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!(
                "chan devserver: NOTE: discovery socket unavailable ({e}); \
                 serve-handoff registration is disabled"
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Management handlers.
// ---------------------------------------------------------------------------

async fn handle_info(State(state): State<Arc<DevserverState>>) -> Json<DevserverInfo> {
    Json(DevserverInfo {
        devserver_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol: DEVSERVER_API_PROTOCOL,
        host_label: state.host_label.clone(),
    })
}

async fn handle_list(State(state): State<Arc<DevserverState>>) -> Json<Vec<WorkspaceEntry>> {
    Json(state.workspace_entries())
}

async fn handle_open(
    State(state): State<Arc<DevserverState>>,
    Json(req): Json<OpenWorkspaceRequest>,
) -> Response {
    match state.register_workspace(Path::new(&req.path)).await {
        Ok(prefix) => Json(MountedPrefix { prefix }).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn handle_forget(
    State(state): State<Arc<DevserverState>>,
    AxumPath(prefix_tail): AxumPath<String>,
) -> Response {
    // The wildcard captures the prefix without its leading slash (the
    // client appends the prefix value verbatim to the route base).
    let prefix = format!("/{}", prefix_tail.trim_start_matches('/'));
    match state.forget_workspace(&prefix) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Set whether the registered workspace addressed by the route is mounted.
/// The catch-all captures `<prefix>/on` (the client appends the prefix
/// verbatim then `/on`, mirroring the `DELETE` convention — an axum catch-all
/// can't carry a fixed `/on` suffix, so the suffix rides inside the capture);
/// we recover the prefix by stripping the trailing `/on`. A capture that is
/// not `<prefix>/on` is not this endpoint and 404s. The body is
/// [`SetWorkspaceOnRequest`]; the response is the updated [`WorkspaceEntry`]
/// (404 when the prefix is not a registered workspace).
async fn handle_set_workspace_on(
    State(state): State<Arc<DevserverState>>,
    AxumPath(captured): AxumPath<String>,
    Json(req): Json<SetWorkspaceOnRequest>,
) -> Response {
    let Some(prefix_tail) = captured.trim_start_matches('/').strip_suffix("/on") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let prefix = format!("/{}", prefix_tail.trim_start_matches('/'));
    // Confirm-before-off: unmounting a workspace kills the terminals running in
    // it, so a reversible off with live terminals is refused (the response
    // carries the count) until the client re-issues with `force`. The check is
    // server-side because `cs` and the launcher can trigger the off too, not
    // just the desktop's own confirm dialog.
    if !req.on && !req.force {
        let active = state.host.tenant_terminal_session_count(&prefix);
        if active > 0 {
            return (
                StatusCode::CONFLICT,
                Json(ActiveTerminalsRejection {
                    active_terminals: active,
                }),
            )
                .into_response();
        }
    }
    match state.set_workspace_on(&prefix, req.on).await {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /api/devserver/windows` (L10): every PERSISTED window across all
/// tenants, for the desktop's menu-reopen of closed devserver windows. Folds
/// the host's per-tenant window enumeration into `DevserverWindow` rows,
/// stamping each with its tenant's per-mount token; the desktop filters
/// `saved && !connected`. Persisted-only: a discard reaped the blob + PTYs, so
/// only windows with a live blob (`saved`) surface.
async fn handle_list_windows(
    State(_state): State<Arc<DevserverState>>,
) -> Json<Vec<DevserverWindow>> {
    // Superseded by the library window feed `GET /api/library/windows` (Seam W),
    // which the desktop watcher and `cs window list` reconcile to. The
    // per-tenant enumeration that backed this endpoint is gone with the host
    // move; this returns empty during the transition until the feed lands and
    // this endpoint is retired.
    Json(Vec::new())
}

/// Gate every `/api/devserver/*` management route except `info` on the devserver
/// bearer token. The token arrives in the `Authorization: Bearer` header (`cs`,
/// the desktop). The management surface is header-only: it has no WebSocket
/// route, so there is no `?t=` query-token path here (the launcher's watch WS
/// lives in [`crate::routes::launcher_router`], which owns its own `?t=` rule).
async fn require_bearer(
    State(state): State<Arc<DevserverState>>,
    req: HttpRequest<Body>,
    next: Next,
) -> Response {
    let header_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let token = state.token.as_bytes();
    if header_token.is_some_and(|t| bytes_eq(t.as_bytes(), token)) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "missing or invalid devserver bearer token",
        )
            .into_response()
    }
}

/// Length-then-content comparison of two byte slices in time independent of
/// where they first differ, so a wrong token leaks no position information.
/// `pub(crate)` so the launcher bundle ([`crate::routes::launcher_router`])
/// reuses the one vetted constant-time compare for its own bearer gate.
pub(crate) fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Prefix + config helpers.
// ---------------------------------------------------------------------------

/// Top-level prefix reserved for the devserver's own `/api/` namespace (the
/// management API and the terminal tenants). A workspace whose basename
/// sanitizes to `api` would mount at `/api` and shadow that namespace, so
/// [`mount_at`](DevserverState::mount_at) rejects it. Workspace tenants mount at
/// their public slug `/{slug}` (top-level); only `/api` collides.
const RESERVED_WORKSPACE_PREFIX: &str = "/api";

/// Mount prefix of the per-library SHARED terminal tenant (D-W3) that every
/// devserver Terminal window resolves to. Fixed (one shared tenant per library),
/// and distinct from per-label terminal prefixes (`/api/term-…`) and workspace
/// prefixes (the top-level public slug `/{slug}`), so it never collides.
const DEVSERVER_SHARED_TERMINAL_PREFIX: &str = "/api/terminal";

/// Per-tenant serve config: each workspace gets its own bearer token (so
/// `no_token` is false), no browser, no idle timeout.
fn tenant_config(addr: SocketAddr, prefix: &str) -> ServeConfig {
    ServeConfig {
        addr,
        no_token: false,
        prefix: prefix.to_string(),
        idle_timeout: None,
        open_browser: false,
        search_aggression: None,
        settings_disabled: false,
        verbose: false,
    }
}

/// Display label for a workspace: its last path segment, or the full path
/// when there is no file name.
fn workspace_label(root: &Path) -> String {
    root.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_library::workspace_slug;

    #[test]
    fn token_marker_is_the_locked_wire_string() {
        // LOCKED contract: the desktop control terminal scrapes this exact
        // prefix from the connect-script output. Both the foreground emit and
        // the `--systemd` re-attach emit build to it, so pin it here — an
        // accidental edit breaks reconnect.
        assert_eq!(DEVSERVER_TOKEN_MARKER, "CHAN_DEVSERVER_TOKEN=");
    }

    #[tokio::test]
    async fn port_zero_bind_resolves_to_a_concrete_port() {
        // The ready line reports `listener.local_addr()`, not the requested
        // addr, so `chan devserver --port 0` prints the OS-assigned port (the
        // shape `chan serve` reports) instead of `:0`.
        let requested: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = tokio::net::TcpListener::bind(requested).await.unwrap();
        let local_addr = listener.local_addr().unwrap_or(requested);
        assert_eq!(local_addr.ip(), requested.ip());
        assert_ne!(
            local_addr.port(),
            0,
            "the OS assigns a concrete port for :0"
        );
    }

    #[test]
    fn slug_sanitizes_and_falls_back() {
        assert_eq!(workspace_slug(Path::new("/home/u/My Notes")), "my-notes");
        assert_eq!(workspace_slug(Path::new("/home/u/notes.d")), "notes-d");
        assert_eq!(workspace_slug(Path::new("/home/u/__")), "workspace");
        assert_eq!(workspace_slug(Path::new("/")), "workspace");
    }

    #[test]
    fn workspace_prefix_is_the_public_slug() {
        let a = allocate_workspace_prefix(Path::new("/tmp/notes")).unwrap();
        let b = allocate_workspace_prefix(Path::new("/tmp/notes")).unwrap();
        // Deterministic: the same root maps to the same prefix.
        assert_eq!(a, b);
        // The prefix IS the public slug the gateway forwards (`/{slug}`), not
        // the old opaque `/api/{slug}-{hash}`. Top-level, never under the
        // reserved `/api/` management+terminal namespace, never empty.
        assert_eq!(a, "/notes");
        assert!(!a.starts_with("/api"));
        assert_ne!(a, "");
        // A different basename differs.
        let c = allocate_workspace_prefix(Path::new("/tmp/other")).unwrap();
        assert_ne!(a, c);
        // Same basename under a different parent COLLIDES (same slug): the
        // devserver rejects the second at mount time (slug uniqueness).
        let d = allocate_workspace_prefix(Path::new("/tmp/sub/notes")).unwrap();
        assert_eq!(a, d);
    }

    #[tokio::test]
    async fn mount_uses_public_slug_and_rejects_slug_collision_and_reserved() {
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        // A workspace named "notes" mounts at its PUBLIC slug "/notes" (the path
        // the gateway forwards), not an opaque /api/{slug}-{hash}.
        let parent = tempfile::tempdir().expect("parent");
        let notes = parent.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("n.md"), "# N\n").unwrap();
        let prefix = state.register_workspace(&notes).await.expect("mount");
        assert_eq!(prefix, "/notes");
        assert!(state
            .host
            .mounted_prefixes()
            .unwrap()
            .contains(&"/notes".to_string()));

        // A SECOND workspace with the same basename collides on the slug and is
        // rejected at mount time (slug uniqueness within a devserver).
        let other = tempfile::tempdir().expect("other");
        let notes2 = other.path().join("notes");
        std::fs::create_dir_all(&notes2).unwrap();
        std::fs::write(notes2.join("n.md"), "# N2\n").unwrap();
        let err = state.register_workspace(&notes2).await.unwrap_err();
        assert!(err.to_string().contains("already mounted"), "{err}");

        // A workspace whose basename sanitizes to the reserved "api" slug is
        // rejected: it would mount at /api and shadow the management namespace.
        let api_parent = tempfile::tempdir().expect("api parent");
        let api_dir = api_parent.path().join("api");
        std::fs::create_dir_all(&api_dir).unwrap();
        std::fs::write(api_dir.join("a.md"), "# A\n").unwrap();
        let err = state.register_workspace(&api_dir).await.unwrap_err();
        assert!(err.to_string().contains("reserved"), "{err}");
    }

    #[test]
    fn bytes_eq_is_length_and_content_sensitive() {
        assert!(bytes_eq(b"secret", b"secret"));
        assert!(!bytes_eq(b"secret", b"secre"));
        assert!(!bytes_eq(b"secret", b"secreT"));
        assert!(bytes_eq(b"", b""));
    }

    #[test]
    fn persisted_config_round_trips() {
        let cfg = PersistedConfig {
            devserver_token: "tok".into(),
            library_id: "lib-abc".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: PersistedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.devserver_token, "tok");
        assert_eq!(back.library_id, "lib-abc");
        // Tolerant of a missing/empty file shape.
        let empty: PersistedConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.devserver_token, "");
        // Old-format keys (`enabled_workspaces`, `workspaces`, `terminals`)
        // degrade cleanly: workspace on/off lives in the overlay store now and
        // the per-label terminal subsystem is gone, so unknown keys are ignored
        // rather than failing the whole parse and minting a fresh token.
        let legacy =
            r#"{"devserver_token":"keep","workspaces":[{"path":"/x","on":true}],"terminals":[]}"#;
        let migrated: PersistedConfig = serde_json::from_str(legacy).unwrap();
        assert_eq!(migrated.devserver_token, "keep");
    }

    #[tokio::test]
    async fn off_without_live_terminals_is_not_blocked() {
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        std::fs::write(ws.path().join("a.md"), "# A\n").expect("seed");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

        // An unforced off of a workspace with no live terminals clears the
        // confirm-before-off guard (count is 0) and unmounts: 200, not 409. (The
        // 409 path needs a live PTY in the tenant, which the host's
        // `tenant_terminal_session_count` test covers.)
        let off = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri(format!("/api/devserver/workspaces{prefix}/on"))
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"on":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(off.status(), StatusCode::OK);
    }

    #[test]
    fn store_save_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = DevserverStore::at(dir.path().join("nested").join("config.json"));
        // Missing file loads a default.
        assert_eq!(store.load().devserver_token, "");
        let cfg = PersistedConfig {
            devserver_token: "abc".into(),
            library_id: "lib-xyz".into(),
        };
        store.save(&cfg).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.devserver_token, "abc");
        assert_eq!(loaded.library_id, "lib-xyz");
        // The atomic tmp+rename leaves no tmpfile behind after a save.
        let tmp = dir.path().join("nested").join("config.json.tmp");
        assert!(!tmp.exists(), "leftover tmpfile: {}", tmp.display());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("nested").join("config.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "config must be 0600");
        }
    }

    /// Build a `DevserverState` over a sandbox dir for the on/off
    /// state-machine tests: a fresh `Library`, an empty host, and a devserver
    /// store under `home`.
    fn test_state(home: &Path, addr: SocketAddr) -> Arc<DevserverState> {
        let lib = Library::open_at(home.join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, crate::route_builder()));
        // Install the workspace overlay so persist_state has somewhere to write
        // the on/off rows (run_devserver installs it beside the window registry).
        host.install_workspace_overlay(Arc::new(WorkspaceOverlay::open(
            home.join("devserver").join("workspaces.json"),
        )));
        Arc::new(DevserverState {
            host,
            addr,
            token: "test-token".into(),
            library_id: "lib-test".into(),
            host_label: "test".into(),
            workspaces: Mutex::new(HashMap::new()),
            store: DevserverStore::at(home.join("devserver").join("config.json")),
        })
    }

    #[tokio::test]
    async fn shared_terminal_tenant_makes_terminal_windows_resolve() {
        // The real devserver open path: mount the shared terminal tenant, then
        // run the library's first-open rule (what `run_devserver` does at
        // startup). The minted terminal window must resolve to the shared
        // tenant's prefix + a real token, so the desktop watcher's should_show
        // (non-empty token) shows it rather than hiding it on every reconnect.
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        state.host.install_window_registry(
            Arc::new(WindowRegistry::open(home.path().join("windows.json"))),
            "lib-test".into(),
        );

        // Mount the shared terminal tenant (the D-W3 mount run_devserver does
        // before provisioning the first-open terminal), then provision it.
        state
            .mount_shared_terminal_tenant()
            .await
            .expect("mount shared terminal tenant");
        let term = state
            .host
            .ensure_first_open_terminal()
            .expect("first open")
            .expect("fresh devserver mints exactly one terminal");

        let records = state.host.assemble_window_records();
        assert_eq!(records.len(), 1, "exactly one window after first open");
        let after = records
            .into_iter()
            .find(|r| r.window_id == term.window_id)
            .expect("terminal row");
        assert_eq!(
            after.kind,
            chan_library::windows::WindowKind::Terminal,
            "first-open window is a terminal",
        );
        assert_eq!(
            after.prefix, DEVSERVER_SHARED_TERMINAL_PREFIX,
            "terminal window resolves to the shared tenant prefix",
        );
        assert!(
            !after.token.is_empty(),
            "terminal window resolves to a real token so should_show shows it",
        );

        // The marker is now set: a second open (a restart whose terminal was
        // never closed) mints nothing extra.
        assert!(state
            .host
            .ensure_first_open_terminal()
            .expect("re-open")
            .is_none());
        assert_eq!(
            state.host.assemble_window_records().len(),
            1,
            "no re-mint on a second open",
        );
    }

    #[tokio::test]
    async fn workspace_on_off_toggle_round_trip() {
        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        std::fs::write(ws.path().join("a.md"), "# A\n").expect("seed");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        // Mount it on: one listed row, on, carrying a token.
        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        let entries = state.workspace_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].on);
        assert!(!entries[0].token.is_empty(), "on row carries a token");
        let token_on = entries[0].token.clone();

        // Toggle off: unmounted in the host, still registered, empty token,
        // SAME prefix.
        let row = state
            .set_workspace_on(&prefix, false)
            .await
            .expect("toggle off")
            .expect("row present");
        assert!(!row.on);
        assert!(row.token.is_empty(), "off row drops its token");
        assert_eq!(row.prefix, prefix, "prefix stays stable across off");
        assert_eq!(state.workspace_entries().len(), 1, "off row still listed");
        assert!(
            state.host.mounted_prefixes().unwrap().is_empty(),
            "off workspace is unmounted in the host"
        );

        // Idempotent off.
        let row = state
            .set_workspace_on(&prefix, false)
            .await
            .unwrap()
            .unwrap();
        assert!(!row.on);

        // Toggle on: remounted at the SAME prefix. chan's per-workspace token
        // is persisted, so the on row carries that SAME stable token (the off
        // row merely hid it on the wire). The client rebuilds the tenant URL
        // from whatever the on row carries — a stable token keeps the URL
        // bookmarkable across off→on, which is the behavior we want.
        let row = state
            .set_workspace_on(&prefix, true)
            .await
            .expect("toggle on")
            .expect("row present");
        assert!(row.on);
        assert_eq!(row.prefix, prefix);
        assert!(!row.token.is_empty(), "on row carries the workspace token");
        assert_eq!(
            row.token, token_on,
            "per-workspace token is stable across off→on (persisted, not per-mount)"
        );
        assert_eq!(state.host.mounted_prefixes().unwrap(), vec![prefix.clone()]);

        // An unknown prefix is a 404 (None), not an error.
        assert!(state
            .set_workspace_on("/api/nope-0", true)
            .await
            .expect("no error")
            .is_none());
    }

    #[tokio::test]
    async fn lists_full_host_library_and_toggles_unserved_workspaces_on() {
        // D1: GET /workspaces lists ONE row per HOST-LIBRARY workspace (what
        // `chan list` shows), not just the devserver's served subset — so a
        // fresh devserver is not empty. An unserved library workspace is off at
        // its stable prefix; `{prefix}/on` mounts it even though it was never
        // registered on the devserver.
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        // Two workspaces registered in the HOST LIBRARY directly (as `chan add`
        // would), with NEITHER mounted on the devserver.
        let ws_a = tempfile::tempdir().expect("a");
        let ws_b = tempfile::tempdir().expect("b");
        std::fs::write(ws_a.path().join("a.md"), "# A\n").unwrap();
        std::fs::write(ws_b.path().join("b.md"), "# B\n").unwrap();
        state
            .host
            .library()
            .register_workspace(ws_a.path())
            .unwrap();
        state
            .host
            .library()
            .register_workspace(ws_b.path())
            .unwrap();

        // The devserver surfaces BOTH — the full library — off, no token.
        let entries = state.workspace_entries();
        assert_eq!(
            entries.len(),
            2,
            "lists the full host library, not the served subset"
        );
        assert!(
            entries.iter().all(|e| !e.on),
            "unserved library workspaces are off"
        );
        assert!(
            entries.iter().all(|e| e.token.is_empty()),
            "off rows carry no token"
        );
        assert!(
            state.host.mounted_prefixes().unwrap().is_empty(),
            "nothing mounted yet"
        );

        // Toggle A on by its stable prefix — never registered on the devserver,
        // yet this mounts it (D1: every library workspace is toggleable).
        let prefix_a = allocate_workspace_prefix(ws_a.path()).expect("prefix");
        let row = state
            .set_workspace_on(&prefix_a, true)
            .await
            .expect("toggle on")
            .expect("library workspace is a known prefix");
        assert!(row.on);
        assert_eq!(row.prefix, prefix_a);
        assert!(!row.token.is_empty(), "an on row carries a token");

        // Still two rows; exactly A is on.
        let entries = state.workspace_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries.iter().filter(|e| e.on).count(), 1);
        assert!(entries.iter().find(|e| e.prefix == prefix_a).unwrap().on);

        // An unknown prefix (no library workspace, no serving record) is a 404.
        assert!(state
            .set_workspace_on("/api/ghost-0", true)
            .await
            .expect("no error")
            .is_none());
    }

    #[tokio::test]
    async fn forget_is_destructive_and_removes_from_the_host_library() {
        // Seam B Amendment 6: the devserver Forget is DESTRUCTIVE — it is
        // `chan workspace rm` (unmount-if-on + unregister from the host library
        // + bin the trash). The host library is the single registry, so the
        // workspace then disappears from the listing too. (`set_workspace_on
        // {on:false}` is the reversible unmount; this is the removal.)
        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        std::fs::write(ws.path().join("a.md"), "# A\n").unwrap();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        assert_eq!(state.workspace_entries().len(), 1);

        assert!(state.forget_workspace(&prefix).expect("forget"));
        assert!(
            state.host.mounted_prefixes().unwrap().is_empty(),
            "forget unmounts the workspace in the host"
        );
        // Destructive: unregistered from the host library, so gone from the
        // listing — one registry, one removal.
        assert!(
            state.workspace_entries().is_empty(),
            "forgotten workspace is removed from the library listing"
        );
        assert!(
            state.host.library().list_workspaces().is_empty(),
            "forgotten workspace is unregistered from the host library"
        );

        // Idempotent: forgetting an unknown / already-removed prefix is false.
        assert!(!state.forget_workspace(&prefix).expect("already removed"));
    }

    #[tokio::test]
    async fn off_state_persists_to_overlay() {
        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        state
            .set_workspace_on(&prefix, false)
            .await
            .unwrap()
            .unwrap();

        // The library-owned overlay records the workspace registered-but-off (by
        // path, the prefix re-derived at restore). On restart, `run_devserver`
        // reads the overlay and `track_off`s this row rather than re-mounting.
        let rows = state
            .host
            .workspace_overlay()
            .expect("overlay installed")
            .entries();
        assert_eq!(rows.len(), 1);
        // The host canonicalizes the registered root, so compare against the
        // canonical path.
        let canonical = ws.path().canonicalize().expect("canonicalize workspace");
        assert_eq!(rows[0].path, canonical.to_string_lossy());
        assert!(!rows[0].on);
    }

    #[tokio::test]
    async fn library_windows_feed_lists_mints_and_discards() {
        use axum::body::to_bytes;
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        // The real devserver installs a window registry in run_devserver; do the
        // same here so mint/discard have a store.
        state.host.install_window_registry(
            Arc::new(chan_library::windows::WindowRegistry::open(
                home.path().join("windows.json"),
            )),
            "local".to_string(),
        );
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

        // On the devserver (gateway surface) the feed is TUNNEL-TRUSTED: the
        // launcher bundle installs with `bearer = None`, because the gateway
        // proxy strips every client credential and is the sole auth boundary. So
        // the feed serves WITHOUT a bearer (the loopback's `?t=` gate is covered
        // by `launcher_router_bearer_gates_data_routes`).
        let listed = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);

        // The watch route is registered (no conflict with the discard route): a
        // plain GET is a 4xx upgrade error, not a 404.
        let watch = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows/watch")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(watch.status(), StatusCode::NOT_FOUND);

        // Mint a terminal window: 200 with the assembled record (a w- id, stamped
        // with the library id).
        let minted = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/library/windows")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"kind":"terminal"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(minted.status(), StatusCode::OK);
        let body = to_bytes(minted.into_body(), 64 * 1024).await.unwrap();
        let record: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let window_id = record["window_id"].as_str().unwrap().to_string();
        assert!(window_id.starts_with("w-"));
        assert_eq!(record["kind"], "terminal");
        assert_eq!(record["library_id"], "local");

        // Discard it: 204; an unknown id is 404.
        let discarded = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("DELETE")
                    .uri(format!("/api/library/windows/{window_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(discarded.status(), StatusCode::NO_CONTENT);
        let missing = app
            .oneshot(
                HttpRequest::builder()
                    .method("DELETE")
                    .uri("/api/library/windows/w-nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn library_workspaces_lists_registered_with_on_state() {
        use axum::body::to_bytes;
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        // Register + mount one workspace so it lists as on.
        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

        // Tunnel-trust on the devserver surface: no bearer needed.
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/workspaces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let rows: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let rows = rows.as_array().expect("array of workspaces");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        // workspace_id is the route prefix without its leading slash.
        assert_eq!(row["workspace_id"], prefix.trim_start_matches('/'));
        assert_eq!(row["on"], true);
        // Path is the canonical workspace root; label is its basename.
        let basename = ws.path().file_name().unwrap().to_str().unwrap();
        assert!(row["path"].as_str().unwrap().ends_with(basename));
        assert!(!row["label"].as_str().unwrap().is_empty());
    }

    /// The `on` state of a workspace `id` from the launcher list, or `None` when
    /// no row matches (forgotten).
    async fn launcher_workspace_on(app: &axum::Router, id: &str) -> Option<bool> {
        use axum::body::to_bytes;
        use tower::ServiceExt;
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/workspaces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let rows: serde_json::Value = serde_json::from_slice(&body).unwrap();
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["workspace_id"] == id)
            .map(|r| r["on"].as_bool().unwrap())
    }

    #[tokio::test]
    async fn library_workspaces_crud_is_loopback_only() {
        use axum::body::to_bytes;
        use std::sync::OnceLock;
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let host = state.host.clone();

        // Read-only surface (serve_addr = None): a mutating call is refused 403,
        // so a grantee can never escalate to mutation.
        let readonly = crate::routes::launcher_router(host.clone(), None, None);
        let refused = readonly
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/library/workspaces/anything/off")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);

        // Loopback surface: serve_addr filled post-bind enables the full CRUD.
        let cell = Arc::new(OnceLock::new());
        cell.set(addr).unwrap();
        let app = crate::routes::launcher_router(host.clone(), None, Some(cell));

        // add: register + mount the folder; 200 with the new row (on).
        let body = format!(r#"{{"path":{:?}}}"#, ws.path().to_string_lossy());
        let added = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/library/workspaces")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(added.status(), StatusCode::OK);
        let bytes = to_bytes(added.into_body(), 64 * 1024).await.unwrap();
        let row: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = row["workspace_id"].as_str().unwrap().to_string();
        assert_eq!(row["on"], true);
        assert_eq!(launcher_workspace_on(&app, &id).await, Some(true));

        // off: unmount, keep the registration (still listed, now off).
        let off = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri(format!("/api/library/workspaces/{id}/off"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(off.status(), StatusCode::NO_CONTENT);
        assert_eq!(launcher_workspace_on(&app, &id).await, Some(false));

        // on: remount at the same stable id.
        let on = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri(format!("/api/library/workspaces/{id}/on"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(on.status(), StatusCode::NO_CONTENT);
        assert_eq!(launcher_workspace_on(&app, &id).await, Some(true));

        // rm: unregister; the workspace disappears from the list.
        let removed = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("DELETE")
                    .uri(format!("/api/library/workspaces/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(removed.status(), StatusCode::NO_CONTENT);
        assert_eq!(launcher_workspace_on(&app, &id).await, None);
    }

    #[tokio::test]
    async fn launcher_mounts_at_library_root() {
        use axum::body::to_bytes;
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

        // Root `/` is served by the installed launcher root fallback — public
        // (no bearer). Without the fallback `host_dispatch` 404s the root with
        // an empty body; the launcher always names itself: a 200 SPA shell when
        // the bundle is built, or a 404 whose body names the missing bundle when
        // it isn't (the gate's `cargo test` runs before any frontend build, so
        // build.rs's `create_dir_all` leaves an empty embed there). Either proves
        // the fallback is wired; a non-wired root would be the bare host 404.
        let root = app
            .clone()
            .oneshot(HttpRequest::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = root.status();
        let body = to_bytes(root.into_body(), 1 << 20).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        let launcher_built = text.contains("Chan Launcher");
        assert!(
            launcher_built || text.contains("launcher bundle not built"),
            "root `/` must be served by the launcher fallback (status {status}, body starts: {:.120})",
            text,
        );

        // When the bundle is present (dev tree / a properly built release), the
        // shell is a 200 HTML doc and its hashed module script resolves under `/`
        // (vite `base: "./"` makes `./assets/..` land at the library root).
        if launcher_built {
            assert_eq!(status, StatusCode::OK);
            assert!(text.contains(r#"id="app""#));
            let asset = text
                .split_once("src=\"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .map(|(src, _)| src.trim_start_matches("./").to_string())
                .expect("index references a module script");
            let asset_resp = app
                .clone()
                .oneshot(
                    HttpRequest::builder()
                        .uri(format!("/{asset}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                asset_resp.status(),
                StatusCode::OK,
                "launcher asset {asset} must resolve"
            );
        }

        // An `/api` miss stays a real 404 (never the SPA HTML), so the
        // launcher's `/api/library/*` calls get JSON-style errors.
        let api_miss = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/not-a-route")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(api_miss.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn launcher_router_bearer_gates_data_routes() {
        use tower::ServiceExt;

        // The LOOPBACK surface installs the launcher bundle with `Some(token)`
        // (the desktop per-window token). Drive `launcher_router` directly with a
        // token to pin the bearer semantics: header for every route, `?t=` for the
        // watch WS only. (The devserver/gateway surface uses `None` — tunnel-trust
        // — verified in `library_windows_feed_lists_mints_and_discards`.)
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let host = state.host.clone();
        let app = crate::routes::launcher_router(host, Some("test-token"), None);

        // No credential: rejected.
        let unauth = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

        // Valid `Authorization` header: allowed.
        let with_header = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(with_header.status(), StatusCode::OK);

        // A regular route does NOT accept `?t=`: the header is required (a query
        // token leaks via URL logs, and the SPA fetch can set the header).
        let regular = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows?t=test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(regular.status(), StatusCode::UNAUTHORIZED);

        // The watch WebSocket accepts `?t=`: a valid token passes the bearer
        // gate, so the response is the WebSocket upgrade error (no upgrade
        // headers in this plain request), NOT a 401.
        let watch_ok = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows/watch?t=test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(watch_ok.status(), StatusCode::UNAUTHORIZED);

        // A wrong `?t=` on the watch route is still rejected.
        let watch_bad = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows/watch?t=nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(watch_bad.status(), StatusCode::UNAUTHORIZED);
    }

    /// The watch pump arms its change waiter (`enable`) BEFORE it takes the
    /// snapshot. A `Notify::Notified` records the `notify_waiters` count when it
    /// is created and compares it on first poll, so a change is observed only
    /// when the waiter was created before that change. This pins the ordering
    /// the pump depends on against a real `notify_waiters` (the same primitive
    /// `library_change_notify` fires): armed-before-change wakes, armed-after
    /// blocks.
    #[tokio::test]
    async fn watch_waiter_must_be_armed_before_the_change() {
        use tokio::sync::Notify;

        let notify = Notify::new();

        // Armed before the stand-in "snapshot + send": a change in that window
        // wakes the await, so it returns at once.
        let armed = notify.notified();
        tokio::pin!(armed);
        armed.as_mut().enable();
        notify.notify_waiters();
        tokio::time::timeout(std::time::Duration::from_millis(200), armed.as_mut())
            .await
            .expect("a waiter armed before the change observes it");

        // Armed after the change: the waiter captures the already-advanced count,
        // so the change is behind it and the await blocks. This is what moving
        // the waiter past the snapshot would do, hence arm-before-snapshot.
        notify.notify_waiters();
        let late = notify.notified();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), late)
                .await
                .is_err(),
            "a waiter armed after the change blocks until the next one"
        );
    }
}
