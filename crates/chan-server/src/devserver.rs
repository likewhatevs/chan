//! Headless multi-workspace devserver.
//!
//! `run_devserver` binds a [`WorkspaceHost`] to a real address and adds two
//! surfaces a desktop client and the `chan serve` CLI drive over it:
//!
//! - A management HTTP/JSON API under the reserved `/api/devserver/*`
//!   namespace ([`crate::devserver_api`]): list, mount, forget workspaces
//!   and open standalone terminals. Every workspace tenant mounts under a
//!   non-empty, legible prefix below `/api/`, so the management router
//!   answers first and everything else falls through to the per-tenant
//!   router.
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

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    DevserverInfo, MountedPrefix, MountedTerminal, OpenTerminalRequest, OpenWorkspaceRequest,
    SetWorkspaceOnRequest, TerminalEntry, WorkspaceEntry, DEVSERVER_API_PROTOCOL,
};
use crate::host::WorkspaceHost;
use crate::{sanitize_prefix, Error, ServeConfig};

/// Inputs the CLI resolves for `chan devserver`. The `--systemd`
/// supervision path is layered on in the CLI around this; the runtime
/// itself only needs where to bind and how to label the box.
pub struct DevserverConfig {
    /// Address to bind the public HTTP listener.
    pub addr: SocketAddr,
    /// Human label for the box (drives the client's grouping header).
    pub host_label: String,
}

/// On-disk devserver state. The bearer token is minted once and reused so a
/// reconnecting client keeps working across restarts; `workspaces` is the set
/// of registered workspaces with their mount state, so a restart comes back
/// serving exactly what was on and remembering what was toggled off.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedConfig {
    #[serde(default)]
    devserver_token: String,
    /// Registered workspaces, on and off. Replaces the old
    /// `enabled_workspaces: Vec<String>` outright (pre-release: no dual-read).
    /// Renaming the key also lets an old-format file degrade cleanly: serde
    /// ignores the now-unknown `enabled_workspaces` and keeps the token,
    /// rather than failing the whole parse and minting a fresh one.
    #[serde(default)]
    workspaces: Vec<PersistedWorkspace>,
    /// Standalone terminal tenants, persisted as first-class launcher entities
    /// so they survive a restart (re-mounted with fresh PTYs at their stable
    /// prefix). Named `terminals` for symmetry with `workspaces` (this is the
    /// internal on-disk shape, not a cross-lane wire).
    #[serde(default)]
    terminals: Vec<PersistedTerminal>,
}

/// One registered workspace as persisted: where it lives, the stable route
/// `prefix` it re-mounts at (allocated once, kept across off→on), and whether
/// it was mounted (`on`) or unmounted-but-remembered (`off`) at the last save.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedWorkspace {
    path: String,
    prefix: String,
    on: bool,
}

/// One standalone terminal tenant as persisted: the client's stable window
/// key (`label`, the `?w=<label>`), the route `prefix` it re-mounts at
/// verbatim, and the PTY's default `command` (`None` = login shell). Re-mounted
/// on restart with a FRESH PTY; the per-window pane/tab layout lives in the
/// launcher session store, not here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedTerminal {
    label: String,
    prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
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

/// The launcher session store for standalone terminals:
/// `~/.chan/devserver/terminals/`. Each persisted terminal's per-window layout
/// blob is keyed by its `?w=<label>` here, so the layout survives a devserver
/// restart. `None` when there is no home dir (the terminal then falls back to
/// the in-memory `ephemeral_sessions`).
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

/// A standalone terminal tenant as the devserver tracks it live, the source of
/// truth for `GET /api/devserver/terminals`. Keyed by its stable `prefix`. The
/// `token` is the live per-mount token (re-minted on each remount, so it is
/// NOT persisted — only [`PersistedTerminal`]'s `{label, prefix, command}` is).
struct TerminalRecord {
    label: String,
    prefix: String,
    command: Option<String>,
    token: String,
}

/// Shared runtime state behind the management API and the discovery socket.
struct DevserverState {
    host: Arc<WorkspaceHost>,
    addr: SocketAddr,
    /// Devserver-level bearer token, distinct from per-workspace tokens.
    token: String,
    host_label: String,
    /// Registered workspaces by stable prefix, on and off.
    workspaces: Mutex<HashMap<String, WorkspaceRecord>>,
    /// Standalone terminal tenants by stable prefix, persisted so they survive
    /// a restart (re-mounted with fresh PTYs).
    terminals: Mutex<HashMap<String, TerminalRecord>>,
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
    async fn mount_at(&self, root: &Path, prefix: &str) -> Result<String, Error> {
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
        let Some((currently_on, root)) = current else {
            return Ok(None);
        };
        if currently_on == on {
            // Already in the requested state: idempotent no-op, current row.
            return Ok(self.entry_for(prefix));
        }
        if on {
            // Off → on: remount at the SAME prefix, minting a fresh token.
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
        Ok(self.entry_for(prefix))
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
        let existed = {
            let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.contains_key(prefix)
        };
        if !existed {
            return Ok(false);
        }
        // Unmount if mounted; a no-op false when the row is off (not in host).
        self.host.close_workspace(prefix)?;
        {
            let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.remove(prefix);
        }
        self.persist_state();
        Ok(true)
    }

    /// Persist the bearer token, every registered workspace (with its mount
    /// state), and every standalone terminal, so a restart comes back serving
    /// exactly what was on, remembering what was off, and re-mounting the
    /// terminals. Sorted by prefix for a stable file.
    fn persist_state(&self) {
        let workspaces: Vec<PersistedWorkspace> = {
            let map = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            let mut v: Vec<PersistedWorkspace> = map
                .values()
                .map(|record| PersistedWorkspace {
                    path: record.root.to_string_lossy().into_owned(),
                    prefix: record.prefix.clone(),
                    on: record.on,
                })
                .collect();
            v.sort_by(|a, b| a.prefix.cmp(&b.prefix));
            v
        };
        let terminals: Vec<PersistedTerminal> = {
            let map = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
            let mut v: Vec<PersistedTerminal> = map
                .values()
                .map(|record| PersistedTerminal {
                    label: record.label.clone(),
                    prefix: record.prefix.clone(),
                    command: record.command.clone(),
                })
                .collect();
            v.sort_by(|a, b| a.prefix.cmp(&b.prefix));
            v
        };
        let cfg = PersistedConfig {
            devserver_token: self.token.clone(),
            workspaces,
            terminals,
        };
        if let Err(e) = self.store.save(&cfg) {
            tracing::warn!("persisting devserver config: {e}");
        }
    }

    /// Snapshot the registered workspaces for the list endpoint, on and off,
    /// sorted by prefix for a stable listing.
    fn workspace_entries(&self) -> Vec<WorkspaceEntry> {
        let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<WorkspaceEntry> = workspaces.values().map(entry_from_record).collect();
        entries.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        entries
    }

    /// Open (mount) a standalone terminal tenant for the client window `label`,
    /// running `command` (or the login shell). Allocates a STABLE prefix from
    /// the label so the same terminal re-mounts at the same route across a
    /// restart, records it for persistence, and returns the prefix + token.
    async fn open_terminal(
        &self,
        label: String,
        command: Option<String>,
    ) -> Result<MountedTerminal, Error> {
        let prefix = allocate_terminal_prefix(&label)?;
        let hosted = self
            .host
            .open_terminal_session_with_command(
                tenant_config(self.addr, &prefix),
                command.clone(),
                devserver_terminals_dir(),
            )
            .await?;
        let token = hosted.handle.token.clone().unwrap_or_default();
        {
            let mut terminals = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
            terminals.insert(
                hosted.prefix.clone(),
                TerminalRecord {
                    label,
                    prefix: hosted.prefix.clone(),
                    command,
                    token: token.clone(),
                },
            );
        }
        self.persist_state();
        Ok(MountedTerminal {
            prefix: hosted.prefix,
            token,
        })
    }

    /// Snapshot the standalone terminals for the list endpoint, sorted by
    /// prefix. The desktop calls this on connect/reconnect to re-create the
    /// devserver's terminal windows (each keyed by its `label`/`?w=<label>`).
    fn terminal_entries(&self) -> Vec<TerminalEntry> {
        let terminals = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<TerminalEntry> = terminals
            .values()
            .map(|record| TerminalEntry {
                label: record.label.clone(),
                prefix: record.prefix.clone(),
                token: record.token.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        entries
    }

    /// Forget the standalone terminal at `prefix`: reap its PTYs + unmount the
    /// tenant, then drop it from the persisted set so it does NOT re-mount on
    /// the next restart. Returns whether a terminal was registered there. The
    /// desktop's close-for-good (vs bury) routes here.
    fn forget_terminal(&self, prefix: &str) -> Result<bool, Error> {
        let label = {
            let terminals = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
            terminals.get(prefix).map(|record| record.label.clone())
        };
        let Some(label) = label else {
            return Ok(false);
        };
        // Reap the PTYs synchronously (close_terminal_tenant), then drop the
        // record + persist so the terminal is gone for good.
        self.host.close_terminal_tenant(prefix)?;
        {
            let mut terminals = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
            terminals.remove(prefix);
        }
        // Drop the persisted per-window layout blob too (best-effort), so a
        // later terminal that reuses the label starts with a fresh layout.
        if let Some(dir) = devserver_terminals_dir() {
            let _ = crate::terminal_blob::delete(&dir, &label);
        }
        self.persist_state();
        Ok(true)
    }

    /// Re-mount a persisted terminal at its stored (stable) prefix on restart:
    /// a FRESH PTY running the recorded command; the window/tab layout restores
    /// from the launcher session store. Does NOT persist (the restart path
    /// batches one save).
    async fn remount_terminal(&self, term: &PersistedTerminal) -> Result<(), Error> {
        let hosted = self
            .host
            .open_terminal_session_with_command(
                tenant_config(self.addr, &term.prefix),
                term.command.clone(),
                devserver_terminals_dir(),
            )
            .await?;
        let token = hosted.handle.token.clone().unwrap_or_default();
        let mut terminals = self.terminals.lock().unwrap_or_else(|e| e.into_inner());
        terminals.insert(
            term.prefix.clone(),
            TerminalRecord {
                label: term.label.clone(),
                prefix: term.prefix.clone(),
                command: term.command.clone(),
                token,
            },
        );
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

    let host = Arc::new(WorkspaceHost::new(library));
    // Opt in to control-socket `chan unserve`: a hosted workspace's tenant can
    // then be unmounted by path (it does not kill the multi-tenant process).
    host.install_self();
    let state = Arc::new(DevserverState {
        host: host.clone(),
        addr: config.addr,
        token: token.clone(),
        host_label: config.host_label,
        workspaces: Mutex::new(HashMap::new()),
        terminals: Mutex::new(HashMap::new()),
        store,
    });

    // Restore the registered workspaces. `on` rows re-mount at their persisted
    // (stable) prefix; `off` rows are tracked as registered-but-unmounted so
    // the client still sees them and can toggle them on. A root that fails to
    // re-mount is downgraded to off so its row still surfaces.
    for ws in &persisted.workspaces {
        let path = PathBuf::from(&ws.path);
        if ws.on {
            if let Err(e) = state.mount_at(&path, &ws.prefix).await {
                eprintln!("chan devserver: NOTE: could not re-mount {}: {e}", ws.path);
                state.track_off(&path, &ws.prefix);
            }
        } else {
            state.track_off(&path, &ws.prefix);
        }
    }
    // Re-mount persisted standalone terminals at their stable prefix: fresh
    // PTYs, with the per-window pane/tab layout restored from the launcher
    // session store. A terminal that fails to re-mount surfaces a note.
    for term in &persisted.terminals {
        if let Err(e) = state.remount_terminal(term).await {
            eprintln!(
                "chan devserver: NOTE: could not re-mount terminal {}: {e}",
                term.label
            );
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

    crate::signal::graceful_serve(listener, app, signal_tx)
        .await
        .context("running devserver")?;
    Ok(())
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
        .route(
            "/api/devserver/terminals",
            get(handle_list_terminals).post(handle_open_terminal),
        )
        .route(
            "/api/devserver/terminals/*prefix",
            delete(handle_forget_terminal),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ))
        .with_state(state);
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
    match state.set_workspace_on(&prefix, req.on).await {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn handle_list_terminals(
    State(state): State<Arc<DevserverState>>,
) -> Json<Vec<TerminalEntry>> {
    Json(state.terminal_entries())
}

async fn handle_forget_terminal(
    State(state): State<Arc<DevserverState>>,
    AxumPath(prefix_tail): AxumPath<String>,
) -> Response {
    // The wildcard captures the prefix without its leading slash (the client
    // appends the prefix value verbatim to the route base), mirroring the
    // workspace DELETE.
    let prefix = format!("/{}", prefix_tail.trim_start_matches('/'));
    match state.forget_terminal(&prefix) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn handle_open_terminal(
    State(state): State<Arc<DevserverState>>,
    Json(req): Json<OpenTerminalRequest>,
) -> Response {
    match state.open_terminal(req.label, req.command).await {
        Ok(mounted) => Json(mounted).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Gate every management route except `info` on the devserver bearer token.
async fn require_bearer(
    State(state): State<Arc<DevserverState>>,
    req: HttpRequest<Body>,
    next: Next,
) -> Response {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match presented {
        Some(t) if bytes_eq(t.as_bytes(), state.token.as_bytes()) => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            "missing or invalid devserver bearer token",
        )
            .into_response(),
    }
}

/// Length-then-content comparison of two byte slices in time independent of
/// where they first differ, so a wrong token leaks no position information.
fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
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
        tunnel_public: false,
        verbose: false,
    }
}

/// Allocate a workspace's mount prefix: `/api/{slug}-{hash}`, where `slug`
/// is the sanitized last path segment and `hash` disambiguates over the
/// canonical root. Deterministic, so the same root always maps to the same
/// prefix (idempotent re-register and stable URLs across restarts).
fn allocate_workspace_prefix(root: &Path) -> Result<String, Error> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();
    let slug = workspace_slug(root);
    sanitize_prefix(&format!("/api/{slug}-{hash:x}")).map_err(Error::Config)
}

/// Allocate a standalone terminal's mount prefix from its window label:
/// `/api/term-{slug}-{hash}`, deterministic so the same label always maps to
/// the same prefix — the terminal re-mounts at the same route across a restart.
fn allocate_terminal_prefix(label: &str) -> Result<String, Error> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    label.hash(&mut hasher);
    let hash = hasher.finish();
    let slug = workspace_slug(Path::new(label));
    sanitize_prefix(&format!("/api/term-{slug}-{hash:x}")).map_err(Error::Config)
}

/// Sanitize a path segment into a legible `[a-z0-9-]` slug for a prefix:
/// lowercase, non-alphanumerics to `-`, collapsed and trimmed, length
/// capped, with a fallback for an empty result.
fn workspace_slug(root: &Path) -> String {
    let raw = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let mut slug: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let trimmed: String = slug.trim_matches('-').chars().take(24).collect();
    let trimmed = trimmed.trim_matches('-');
    if trimmed.is_empty() {
        "workspace".to_string()
    } else {
        trimmed.to_string()
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
    fn prefix_is_legible_unique_and_valid() {
        let a = allocate_workspace_prefix(Path::new("/tmp/notes")).unwrap();
        let b = allocate_workspace_prefix(Path::new("/tmp/notes")).unwrap();
        // Deterministic: the same root maps to the same prefix.
        assert_eq!(a, b);
        assert!(a.starts_with("/api/notes-"), "unexpected prefix: {a}");
        // Never the reserved management namespace, never empty.
        assert!(!a.starts_with("/api/devserver/"));
        assert_ne!(a, "");
        // A different root differs.
        let c = allocate_workspace_prefix(Path::new("/tmp/other")).unwrap();
        assert_ne!(a, c);
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
            workspaces: vec![
                PersistedWorkspace {
                    path: "/a".into(),
                    prefix: "/api/a-0".into(),
                    on: true,
                },
                PersistedWorkspace {
                    path: "/b".into(),
                    prefix: "/api/b-0".into(),
                    on: false,
                },
            ],
            terminals: vec![PersistedTerminal {
                label: "terminal-1".into(),
                prefix: "/api/term-terminal-1-0".into(),
                command: None,
            }],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: PersistedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.devserver_token, "tok");
        assert_eq!(back.workspaces.len(), 2);
        assert_eq!(back.workspaces[0].path, "/a");
        assert!(back.workspaces[0].on);
        assert_eq!(back.workspaces[1].prefix, "/api/b-0");
        assert!(!back.workspaces[1].on);
        assert_eq!(back.terminals.len(), 1);
        assert_eq!(back.terminals[0].label, "terminal-1");
        // Tolerant of a missing/empty file shape.
        let empty: PersistedConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.devserver_token, "");
        assert!(empty.workspaces.is_empty());
        assert!(empty.terminals.is_empty());
        // An old-format file (`enabled_workspaces: Vec<String>`) degrades to an
        // empty workspace set but KEEPS the token: the renamed key is ignored
        // rather than failing the whole parse and minting a fresh token.
        let legacy = r#"{"devserver_token":"keep","enabled_workspaces":["/x","/y"]}"#;
        let migrated: PersistedConfig = serde_json::from_str(legacy).unwrap();
        assert_eq!(migrated.devserver_token, "keep");
        assert!(migrated.workspaces.is_empty());
    }

    #[test]
    fn persisted_workspace_pins_field_names() {
        // The on-disk record field names are part of the persisted contract;
        // pin them so a rename is a visible, deliberate change.
        let ws = PersistedWorkspace {
            path: "/home/u/notes".into(),
            prefix: "/api/notes-1a2b3c".into(),
            on: true,
        };
        let v = serde_json::to_value(&ws).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "path": "/home/u/notes",
                "prefix": "/api/notes-1a2b3c",
                "on": true,
            })
        );
        assert_eq!(ws, serde_json::from_value(v).unwrap());
    }

    #[test]
    fn persisted_terminal_pins_field_names() {
        let term = PersistedTerminal {
            label: "terminal-1a2b".into(),
            prefix: "/api/term-terminal-1a2b-ff".into(),
            command: Some("ssh host".into()),
        };
        let v = serde_json::to_value(&term).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "label": "terminal-1a2b",
                "prefix": "/api/term-terminal-1a2b-ff",
                "command": "ssh host",
            })
        );
        assert_eq!(term, serde_json::from_value(v).unwrap());
        // `command` is omitted when None (login shell).
        let bare = PersistedTerminal {
            label: "t".into(),
            prefix: "/api/term-t-0".into(),
            command: None,
        };
        assert_eq!(
            serde_json::to_value(&bare).unwrap(),
            serde_json::json!({ "label": "t", "prefix": "/api/term-t-0" })
        );
    }

    #[test]
    fn terminal_prefix_is_stable_and_legible() {
        let a = allocate_terminal_prefix("terminal-1a2b").unwrap();
        let b = allocate_terminal_prefix("terminal-1a2b").unwrap();
        assert_eq!(a, b, "same label -> same prefix (stable across restart)");
        assert!(a.starts_with("/api/term-"), "unexpected: {a}");
        assert!(!a.starts_with("/api/devserver/"));
        let c = allocate_terminal_prefix("terminal-9z9z").unwrap();
        assert_ne!(a, c, "different labels differ");
    }

    #[tokio::test]
    async fn terminal_open_persists_and_remounts_at_stable_prefix() {
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        // Open a standalone terminal: mounted in the host AND persisted.
        let mounted = state
            .open_terminal("terminal-1a2b".into(), Some("printf hi".into()))
            .await
            .expect("open terminal");
        assert!(mounted.prefix.starts_with("/api/term-"));
        assert!(!mounted.token.is_empty());
        assert!(state
            .host
            .mounted_prefixes()
            .unwrap()
            .contains(&mounted.prefix));

        // Listed for the desktop's reconnect discovery, with the live token.
        let entries = state.terminal_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "terminal-1a2b");
        assert_eq!(entries[0].prefix, mounted.prefix);
        assert_eq!(entries[0].token, mounted.token);

        // Persisted with its label, stable prefix, and command.
        let persisted = state.store.load();
        assert_eq!(persisted.terminals.len(), 1);
        assert_eq!(persisted.terminals[0].label, "terminal-1a2b");
        assert_eq!(persisted.terminals[0].prefix, mounted.prefix);
        assert_eq!(persisted.terminals[0].command.as_deref(), Some("printf hi"));

        // Simulate a restart: a FRESH host re-mounts from the persisted record
        // at the SAME prefix (the terminal survives the restart).
        let restarted = test_state(home.path(), addr);
        for term in &restarted.store.load().terminals {
            restarted.remount_terminal(term).await.expect("remount");
        }
        assert!(restarted
            .host
            .mounted_prefixes()
            .unwrap()
            .contains(&mounted.prefix));
    }

    #[tokio::test]
    async fn terminal_forget_unmounts_and_drops_persistence() {
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let mounted = state
            .open_terminal("terminal-x".into(), None)
            .await
            .expect("open");
        assert_eq!(state.terminal_entries().len(), 1);

        // Forget: unmounted, dropped from the list AND the persisted config,
        // so it does not re-mount on the next restart.
        assert!(state.forget_terminal(&mounted.prefix).expect("forget"));
        assert!(state.terminal_entries().is_empty());
        assert!(!state
            .host
            .mounted_prefixes()
            .unwrap()
            .contains(&mounted.prefix));
        assert!(state.store.load().terminals.is_empty());

        // Idempotent / false for an unknown prefix.
        assert!(!state.forget_terminal(&mounted.prefix).expect("absent"));
        assert!(!state.forget_terminal("/api/term-nope-0").expect("unknown"));
    }

    #[test]
    fn store_save_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = DevserverStore::at(dir.path().join("nested").join("config.json"));
        // Missing file loads a default.
        assert_eq!(store.load().devserver_token, "");
        let cfg = PersistedConfig {
            devserver_token: "abc".into(),
            workspaces: vec![PersistedWorkspace {
                path: "/x".into(),
                prefix: "/api/x-0".into(),
                on: true,
            }],
            terminals: Vec::new(),
        };
        store.save(&cfg).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.devserver_token, "abc");
        assert_eq!(loaded.workspaces.len(), 1);
        assert_eq!(loaded.workspaces[0].path, "/x");
        assert!(loaded.workspaces[0].on);
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
        let host = Arc::new(WorkspaceHost::new(lib));
        Arc::new(DevserverState {
            host,
            addr,
            token: "test-token".into(),
            host_label: "test".into(),
            workspaces: Mutex::new(HashMap::new()),
            terminals: Mutex::new(HashMap::new()),
            store: DevserverStore::at(home.join("devserver").join("config.json")),
        })
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
    async fn forget_removes_registered_on_or_off() {
        let home = tempfile::tempdir().expect("home");
        let ws = tempfile::tempdir().expect("workspace");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);

        // Forget a mounted (on) workspace: removed + unmounted.
        let prefix = state.register_workspace(ws.path()).await.expect("mount");
        assert!(state.forget_workspace(&prefix).expect("forget on"));
        assert!(state.workspace_entries().is_empty());
        // Idempotent / false for an unknown row.
        assert!(!state.forget_workspace(&prefix).expect("forget absent"));

        // Forget an OFF workspace still reports removed — the case a
        // host-unmount-only signal would wrongly report as not-found.
        let prefix = state.register_workspace(ws.path()).await.expect("remount");
        state
            .set_workspace_on(&prefix, false)
            .await
            .unwrap()
            .unwrap();
        assert!(state.forget_workspace(&prefix).expect("forget off"));
        assert!(state.workspace_entries().is_empty());
    }

    #[tokio::test]
    async fn off_state_persists_to_store() {
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

        // The on-disk config records the workspace registered-but-off, with
        // its stable prefix, alongside the preserved bearer token. On restart,
        // `run_devserver` would `track_off` this row rather than re-mounting.
        let persisted = state.store.load();
        assert_eq!(persisted.devserver_token, "test-token");
        assert_eq!(persisted.workspaces.len(), 1);
        assert_eq!(persisted.workspaces[0].prefix, prefix);
        assert!(!persisted.workspaces[0].on);
    }
}
