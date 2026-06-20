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

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
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
    ActiveTerminalsRejection, DevserverInfo, DevserverWindow, MountedPrefix, MountedTerminal,
    OpenTerminalRequest, OpenWorkspaceRequest, SetWorkspaceOnRequest, TerminalEntry,
    WorkspaceEntry, DEVSERVER_API_PROTOCOL,
};
use crate::{sanitize_prefix, Error, ServeConfig};
use crate::{CreateWindow, WindowRecord, WindowSet, WorkspaceHost};
// Prefix allocation lives in chan-library (the window-record assembly needs the
// stable OFF-workspace prefix); the devserver mounts at the same prefix.
use chan_library::windows::WindowRegistry;
use chan_library::{allocate_workspace_prefix, workspace_slug};

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
    /// This library's stable identity, minted once (`lib-<16hex>`) and persisted
    /// so it survives restart. Stamped on every window record (Seam W); a client
    /// merging several libraries' feeds partitions by it.
    #[serde(default)]
    library_id: String,
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
    /// This library's stable identity (`lib-<16hex>`), persisted with the token.
    library_id: String,
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
            library_id: self.library_id.clone(),
            workspaces,
            terminals,
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
    let state = Arc::new(DevserverState {
        host: host.clone(),
        addr: config.addr,
        token: token.clone(),
        library_id,
        host_label: config.host_label,
        workspaces: Mutex::new(HashMap::new()),
        terminals: Mutex::new(HashMap::new()),
        store,
    });

    // Mount the per-library SHARED terminal tenant (D-W3) before serving, so
    // devserver Terminal windows resolve to a real prefix+token.
    state
        .mount_shared_terminal_tenant()
        .await
        .context("mounting the devserver shared terminal tenant")?;

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
        .route("/api/devserver/windows", get(handle_list_windows))
        .route(
            "/api/library/windows",
            get(handle_list_library_windows).post(handle_create_library_window),
        )
        .route(
            "/api/library/windows/watch",
            get(handle_watch_library_windows),
        )
        .route(
            "/api/library/windows/:window_id",
            delete(handle_discard_library_window),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ))
        .with_state(state);
    public.merge(authed).merge(host.router())
}

/// `GET /api/library/windows`: the full library window set every client
/// reconciles to. A thin wrapper over the host's shared `assemble_window_records`,
/// which the desktop watcher and `cs window list` also call in-process, so every
/// client reads one assembly with no divergence.
async fn handle_list_library_windows(
    State(state): State<Arc<DevserverState>>,
) -> Json<Vec<WindowRecord>> {
    Json(state.host.assemble_window_records())
}

/// `GET /api/library/windows/watch`: a WebSocket that pushes the full window set
/// on connect and again on every change, so a client reconciles its surface to
/// the live library state without polling. Bearer-gated via the management
/// middleware (the `Authorization` header); a browser WebSocket cannot send that
/// header, so a browser client needs the bearer in a query parameter, while `cs`
/// and the desktop use the header.
async fn handle_watch_library_windows(
    State(state): State<Arc<DevserverState>>,
    ws: WebSocketUpgrade,
) -> Response {
    let host = state.host.clone();
    ws.on_upgrade(move |socket| watch_library_windows(socket, host))
}

/// Push a fresh window-set snapshot on connect and on every change. Sending the
/// whole set rather than a delta keeps the client's reconcile idempotent: a
/// dropped frame self-heals on the next push. The change waiter is armed
/// (`enable`d) BEFORE each snapshot so a change that lands between the snapshot
/// and the await is never missed. The loop ends when the client disconnects.
async fn watch_library_windows(mut socket: WebSocket, host: Arc<WorkspaceHost>) {
    let notify = host.library_change_notify();
    let changed = notify.notified();
    tokio::pin!(changed);
    loop {
        // Arm the change waiter BEFORE the snapshot. A `Notified` records the
        // `notify_waiters` count when it is created, so creating and `enable`-ing
        // it before the snapshot guarantees a change during the snapshot or the
        // `send().await` advances that count and wakes the `select!` below,
        // rather than being read into a snapshot the waiter was armed after. The
        // explicit `enable` also keeps this consumer's ordering identical to the
        // desktop's local watcher.
        changed.as_mut().enable();
        let set = WindowSet {
            windows: host.assemble_window_records(),
        };
        let frame = match serde_json::to_string(&set) {
            Ok(frame) => frame,
            Err(_) => break,
        };
        if socket.send(Message::Text(frame)).await.is_err() {
            break; // the client is gone
        }
        tokio::select! {
            _ = changed.as_mut() => {
                // A window-set change woke us: drop the consumed waiter and
                // re-arm a fresh one, which the next loop turn enables before
                // it reads the snapshot.
                changed.set(notify.notified());
            }
            msg = socket.recv() => match msg {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                _ => {} // ignore any other client frame
            },
        }
    }
}

/// `POST /api/library/windows` `{kind, workspace_path?}`: mint a window. The
/// library assigns the id and persists the record; the registry change bridge
/// fires the watch. Returns the assembled record in the feed shape.
async fn handle_create_library_window(
    State(state): State<Arc<DevserverState>>,
    Json(req): Json<CreateWindow>,
) -> Response {
    match state.host.mint_window(req.kind, req.workspace_path) {
        Ok(record) => Json(record).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `DELETE /api/library/windows/{window_id}`: discard a window by dropping its
/// record; the change bridge fires the watch, and each client's reconcile then
/// closes the window. 404 when no window has that id.
async fn handle_discard_library_window(
    State(state): State<Arc<DevserverState>>,
    AxumPath(window_id): AxumPath<String>,
) -> Response {
    match state.host.discard_window(&window_id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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

async fn handle_list_terminals(
    State(state): State<Arc<DevserverState>>,
) -> Json<Vec<TerminalEntry>> {
    Json(state.terminal_entries())
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

/// Route the watch feed authenticates by `?t=`. A browser cannot set the
/// `Authorization` header on a WebSocket, so the watch upgrade alone accepts the
/// bearer as a query param; every other route stays header-only, since a query
/// token leaks through URL logs and the regular SPA `fetch` can set the header.
const WATCH_WS_PATH: &str = "/api/library/windows/watch";

/// Gate every management route except `info` on the devserver bearer token. The
/// token arrives in the `Authorization: Bearer` header (`cs`, the desktop, the
/// SPA `fetch`); the watch WebSocket additionally accepts it as the `?t=` query
/// param (see [`WATCH_WS_PATH`]).
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
    // The `?t=` query token is accepted only for the watch WebSocket, where the
    // browser has no other way to present it; offering it elsewhere would add a
    // URL-leakable auth path the header already covers.
    let query_token = (req.uri().path() == WATCH_WS_PATH)
        .then(|| req.uri().query().and_then(query_bearer))
        .flatten();
    let authorized = header_token.is_some_and(|t| bytes_eq(t.as_bytes(), token))
        || query_token.is_some_and(|t| bytes_eq(t.as_bytes(), token));
    if authorized {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "missing or invalid devserver bearer token",
        )
            .into_response()
    }
}

/// The `t` bearer from a URL query string (`...?t=<token>`), for a client that
/// cannot set the `Authorization` header (a browser WebSocket). Tokens are
/// alphanumeric, so the value needs no percent-decoding.
fn query_bearer(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "t").then_some(value)
    })
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

/// Mount prefix of the per-library SHARED terminal tenant (D-W3) that every
/// devserver Terminal window resolves to. Fixed (one shared tenant per library),
/// and distinct from per-label terminal prefixes (`/api/term-…`) and workspace
/// prefixes (`/api/{slug}-…`), so it never collides.
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
        tunnel_public: false,
        verbose: false,
    }
}

/// Allocate a workspace's mount prefix: `/api/{slug}-{hash}`, where `slug`
/// is the sanitized last path segment and `hash` disambiguates over the
/// canonical root. Deterministic, so the same root always maps to the same
/// prefix (idempotent re-register and stable URLs across restarts).
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
            library_id: String::new(),
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
    async fn open_terminal_post_requires_a_json_body() {
        // Contract guard for the connect 415 regression: `POST
        // /api/devserver/terminals` requires a labeled JSON body, so a bodyless
        // POST (no `Content-Type: application/json`) is a 415. The desktop always
        // sends `{label}` via `open_terminal_with_label`; this pins the contract
        // so a future bodyless caller fails loudly here, not just at runtime on a
        // fresh connect.
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

        // Bodyless POST → 415 Unsupported Media Type (a label is required).
        let bodyless = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/devserver/terminals")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bodyless.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        // Labeled JSON body → mounts the terminal (200 OK).
        let labeled = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/devserver/terminals")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"label":"terminal-deadbeef"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(labeled.status(), StatusCode::OK);
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
            library_id: String::new(),
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
        let host = Arc::new(WorkspaceHost::new(lib, crate::route_builder()));
        Arc::new(DevserverState {
            host,
            addr,
            token: "test-token".into(),
            library_id: "lib-test".into(),
            host_label: "test".into(),
            workspaces: Mutex::new(HashMap::new()),
            terminals: Mutex::new(HashMap::new()),
            store: DevserverStore::at(home.join("devserver").join("config.json")),
        })
    }

    #[tokio::test]
    async fn shared_terminal_tenant_makes_terminal_windows_resolve() {
        // D-W3: without the shared terminal tenant a devserver Terminal window
        // resolves to an empty prefix/token (and the desktop watcher hides it);
        // after mount_shared_terminal_tenant it resolves to the shared tenant's
        // prefix + a real token.
        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        state.host.install_window_registry(
            Arc::new(WindowRegistry::open(home.path().join("windows.json"))),
            "lib-test".into(),
        );

        let term = state
            .host
            .mint_window(chan_library::windows::WindowKind::Terminal, None)
            .expect("mint terminal");

        let find = |st: &Arc<DevserverState>| {
            st.host
                .assemble_window_records()
                .into_iter()
                .find(|r| r.window_id == term.window_id)
                .expect("terminal row")
        };

        // No shared terminal tenant yet → empty prefix/token (the bug).
        let before = find(&state);
        assert_eq!(
            before.prefix, "",
            "no shared terminal tenant → empty prefix"
        );
        assert_eq!(before.token, "");

        // Mount it (the D-W3 fix run_devserver performs at startup).
        state
            .mount_shared_terminal_tenant()
            .await
            .expect("mount shared terminal tenant");

        let after = find(&state);
        assert_eq!(
            after.prefix, DEVSERVER_SHARED_TERMINAL_PREFIX,
            "terminal window resolves to the shared tenant prefix",
        );
        assert!(
            !after.token.is_empty(),
            "terminal window resolves to a real token so should_show shows it",
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

        // The feed is bearer-gated.
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

        // The watch route is registered (no conflict with the discard route): a
        // plain GET is a 4xx upgrade error, not a 404.
        let watch = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/library/windows/watch")
                    .header(header::AUTHORIZATION, "Bearer test-token")
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
                    .header(header::AUTHORIZATION, "Bearer test-token")
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
                    .header(header::AUTHORIZATION, "Bearer test-token")
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
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn query_token_authorizes_only_the_watch_websocket() {
        use tower::ServiceExt;

        let home = tempfile::tempdir().expect("home");
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = test_state(home.path(), addr);
        let host = state.host.clone();
        let app = build_devserver_app(state, host);

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
