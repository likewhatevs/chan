//! Multi-workspace host runtime.
//!
//! `WorkspaceHost` is the in-process owner that chan-desktop can embed
//! instead of spawning one `chan serve` child per local workspace. Each
//! mounted workspace still gets its own `AppState`, watcher, indexer,
//! MCP bridge, control socket, terminal registry, and route prefix.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock, Weak};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use chan_workspace::{Library, Workspace};
use tokio::sync::Notify;
use tower::ServiceExt;

use crate::desktop_window_ops::DesktopBridge;
use crate::tenant::{HostControl, TenantArtifacts, TenantBuilder, UnserveMode};
use crate::terminal_sessions::CloseReason;
use crate::windows::{PersistedWindow, WindowKind, WindowRecord, WindowRegistry};
use crate::{allocate_workspace_prefix, sanitize_prefix, Error, ServeConfig, ServeHandle};

/// One workspace mounted into a [`WorkspaceHost`].
#[derive(Debug, Clone)]
pub struct HostedWorkspace {
    /// Workspace root for diagnostics and desktop state correlation.
    pub root: PathBuf,
    /// Canonical route prefix where the workspace is mounted.
    pub prefix: String,
    /// Launch handle for browser/webview clients.
    pub handle: ServeHandle,
}

/// In-process multi-workspace host.
///
/// This is intentionally a thin owner around the existing per-workspace
/// server runtime. It does not share route state across workspaces:
/// mounting two workspaces builds two independent `AppState` instances
/// and dispatches by URL prefix.
pub struct WorkspaceHost {
    library: Library,
    workspaces: RwLock<HashMap<String, HostedWorkspaceRuntime>>,
    /// Desktop integration shared by every tenant this host mounts: the
    /// window-ops channel and the title map. `DesktopBridge::default()`
    /// (no channel, empty map) when the embedder is not chan-desktop.
    desktop: DesktopBridge,
    /// Serializes idempotent re-registration so two callers racing the same
    /// root resolve to one mount. The winner holds the per-workspace flock
    /// from its `Library::open_workspace` but only lands in `workspaces`
    /// after `build_app`; without this gate a concurrent loser would see
    /// neither the flock-free map nor the not-yet-inserted winner and fail
    /// its own open. Held across the open's `.await`, so it is a tokio
    /// mutex; registration is infrequent, so serializing it is cheap.
    register_lock: tokio::sync::Mutex<()>,
    /// The route layer's tenant constructor, inverted so the host builds tenants
    /// without depending on chan-server. chan-server's `RouteLayer` implements
    /// it; `open_*` call through it.
    builder: Arc<dyn TenantBuilder>,
    /// The host's own `Arc`, downgraded, registered by
    /// [`install_self`](Self::install_self). Lets a per-tenant control socket
    /// reach back for a `chan unserve` of a hosted path (unmount that tenant).
    /// Empty until an embedder opts in; a host that never does answers
    /// `Unserve` with an "unsupported" message (correct for chan-desktop,
    /// which tears workspaces down in-process).
    self_weak: OnceLock<Weak<dyn HostControl>>,
    /// The library's persisted window registry — the source of truth for which
    /// windows exist (D5 ids). Installed once via
    /// [`install_window_registry`](Self::install_window_registry); the window
    /// feed (`assemble_window_records`) reads it. Empty on a host that never
    /// installs one (its window set is empty).
    window_registry: OnceLock<Arc<WindowRegistry>>,
    /// This library's identity: `"local"` for the baked-in local-disk library,
    /// `lib-<hex>` for a devserver. Stamped on every window record. Set with the
    /// registry; defaults to `"local"` when unset.
    library_id: OnceLock<String>,
    /// Route prefix of this library's shared terminal tenant — the one
    /// standalone-terminal tenant mounted via [`open_terminal_session`](
    /// Self::open_terminal_session) that every terminal window attaches to.
    /// Recorded on first mount so [`window_live_state`](Self::window_live_state)
    /// resolves a terminal window's `(prefix, token)` to it (the terminal
    /// analogue of a workspace tenant). Unset until the tenant mounts.
    terminal_tenant_prefix: OnceLock<String>,
    /// Fires on any change that affects the window set — registry mint/discard,
    /// `WindowPresence` connect/disconnect, tenant on/off — so the watch feed
    /// pushes a fresh snapshot. The aggregate every client's reconcile awaits.
    library_change_notify: Arc<Notify>,
}

struct HostedWorkspaceRuntime {
    root: PathBuf,
    /// Launch handle captured at mount time (addr, prefix, token). Lets the
    /// host hand back the existing mount on an idempotent re-register and
    /// list every tenant without rebuilding one.
    handle: ServeHandle,
    artifacts: TenantArtifacts,
}

impl HostedWorkspaceRuntime {
    fn router(&self) -> Router {
        self.artifacts.app.clone()
    }

    /// Signal shutdown and tear the workspace cell down. Returns a `Weak`
    /// to the workspace plus its lock dir so a caller that needs the
    /// per-workspace flock released before it returns (an in-process close
    /// then reopen) can wait for the last strong `Arc` to drop AND the flock
    /// to free. `None` when the cell was already cleared (a second call, e.g.
    /// Drop after an explicit close, or a terminal tenant with no workspace).
    fn shutdown(&self) -> Option<(Weak<Workspace>, PathBuf)> {
        let _ = self.artifacts.shutdown_tx.send(true);
        self.artifacts.cell.clear()
    }
}

impl Drop for HostedWorkspaceRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl WorkspaceHost {
    /// Create an empty host backed by the caller's `Library`, with no
    /// desktop attached (window-lifecycle ops refuse; the title map stays
    /// empty). The standalone and test path.
    pub fn new(library: Library, builder: Arc<dyn TenantBuilder>) -> Self {
        Self::with_desktop_bridge(library, DesktopBridge::default(), builder)
    }

    /// Create a host whose tenants share `desktop` — chan-desktop passes a
    /// bridge carrying the window-ops channel and the title map so
    /// `cs window <op>` reaches the Tauri app and `cs window list` shows
    /// real titles. `builder` is the route layer's tenant constructor
    /// (chan-server's `RouteLayer`).
    pub fn with_desktop_bridge(
        library: Library,
        desktop: DesktopBridge,
        builder: Arc<dyn TenantBuilder>,
    ) -> Self {
        Self {
            library,
            workspaces: RwLock::new(HashMap::new()),
            desktop,
            register_lock: tokio::sync::Mutex::new(()),
            builder,
            self_weak: OnceLock::new(),
            window_registry: OnceLock::new(),
            library_id: OnceLock::new(),
            terminal_tenant_prefix: OnceLock::new(),
            library_change_notify: Arc::new(Notify::new()),
        }
    }

    /// Install this library's persisted window registry + its identity
    /// (`"local"` / `lib-<hex>`). Idempotent set-once; the window feed reads the
    /// registry, and `library_id` is stamped on every record. The embedder
    /// (devserver / desktop) builds the registry at its store path and calls
    /// this once after wrapping the host in an `Arc`.
    pub fn install_window_registry(&self, registry: Arc<WindowRegistry>, library_id: String) {
        // Bridge the registry's own change signal (it fires on every
        // create/remove) into the aggregate library notify, so the watch feed
        // pushes on a mint/discard without the caller having to fire it. Skipped
        // outside a tokio runtime (unit tests don't run the feed); the task
        // lives for the host's lifetime.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let reg_notify = registry.change_notify();
            let lib_notify = Arc::clone(&self.library_change_notify);
            handle.spawn(async move {
                // Re-arm the registry waiter before fanning out so a change
                // during the fan-out is not dropped (`Notify` keeps no permit
                // for `notify_waiters`).
                let notified = reg_notify.notified();
                tokio::pin!(notified);
                loop {
                    notified.as_mut().await;
                    notified.set(reg_notify.notified());
                    lib_notify.notify_waiters();
                }
            });
        }
        let _ = self.window_registry.set(registry);
        let _ = self.library_id.set(library_id);
    }

    /// Fire the aggregate window-set change signal that the watch feed awaits.
    /// Called when a tenant mounts/unmounts (a workspace window's liveness in
    /// [`assemble_window_records`](Self::assemble_window_records) shifts);
    /// registry mint/discard fire it via the bridge in
    /// [`install_window_registry`](Self::install_window_registry).
    fn notify_window_change(&self) {
        self.library_change_notify.notify_waiters();
    }

    /// This library's persisted window registry, once installed.
    pub fn window_registry(&self) -> Option<&Arc<WindowRegistry>> {
        self.window_registry.get()
    }

    /// This library's identity (`"local"` until a devserver installs its own).
    pub fn library_id(&self) -> &str {
        self.library_id.get().map(String::as_str).unwrap_or("local")
    }

    /// The aggregate change signal the window-set watch feed awaits: fires on
    /// registry mint/discard, presence connect/disconnect, and tenant on/off.
    pub fn library_change_notify(&self) -> Arc<Notify> {
        self.library_change_notify.clone()
    }

    /// Register the host's own `Arc` so per-tenant control sockets can reach it
    /// for a `chan unserve` of a hosted path. Idempotent; an embedder that
    /// wants control-socket unserve of hosted workspaces calls this once after
    /// wrapping the host in an `Arc` (the devserver does). A host that never
    /// calls it answers `Unserve` with an "unsupported" message — correct for
    /// chan-desktop, which tears workspaces down in-process, not over the
    /// control socket.
    pub fn install_self(self: &Arc<Self>) {
        // Unsize the concrete `Weak<WorkspaceHost>` to `Weak<dyn HostControl>`
        // (WorkspaceHost impls HostControl) so the control socket reaches the
        // host without naming the concrete type. Downgrade concretely first,
        // then coerce — inferring the trait object from `set`'s type would make
        // `downgrade` expect `&Arc<dyn HostControl>` and fail.
        let weak_self: Weak<WorkspaceHost> = Arc::downgrade(self);
        let _ = self.self_weak.set(weak_self);
    }

    /// The unserve mode tenants built by this host carry: `Host(weak)` once
    /// [`install_self`](Self::install_self) ran, else `Unsupported`.
    fn unserve_mode(&self) -> UnserveMode {
        match self.self_weak.get() {
            Some(weak) => UnserveMode::Host(weak.clone()),
            None => UnserveMode::Unsupported,
        }
    }

    /// Return the shared workspace registry handle.
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// The desktop bridge shared across this host's tenants. chan-desktop
    /// uses it to write window titles as it builds/destroys webviews.
    pub fn desktop_bridge(&self) -> &DesktopBridge {
        &self.desktop
    }

    /// Open a registered workspace path and mount it under
    /// `config.prefix`.
    ///
    /// The path must already be registered with this host's
    /// `Library`. Desktop first-launch code can create/register the
    /// workspace before calling this method; the CLI compatibility path
    /// keeps its existing auto-create behavior outside the host.
    pub async fn open_registered_workspace(
        &self,
        root: impl AsRef<Path>,
        config: ServeConfig,
    ) -> Result<HostedWorkspace, Error> {
        let workspace = self.library.open_workspace(root.as_ref())?;
        self.open_workspace(workspace, config).await
    }

    /// Mount the workspace at `root` under `config.prefix`, or return the
    /// existing mount when that root is already mounted.
    ///
    /// Idempotent on the workspace ROOT: a root already mounted (under any
    /// prefix) returns its existing [`HostedWorkspace`] without re-opening
    /// it, so the per-workspace single-writer flock the running tenant
    /// holds is never contended (a second `Library::open_workspace` on a
    /// mounted root would fail `WorkspaceAlreadyOpen` anyway). A different
    /// root that collides on `config.prefix` is still an error.
    ///
    /// Race-safe via the host's registration lock: callers racing the same
    /// root serialize, so the first mounts and the rest observe that mount
    /// in the pre-check and return it. A distinct root that collides on
    /// `config.prefix` falls through to `open_registered_workspace` and its
    /// duplicate-prefix error.
    pub async fn open_or_get_registered_workspace(
        &self,
        root: impl AsRef<Path>,
        config: ServeConfig,
    ) -> Result<HostedWorkspace, Error> {
        let root = root.as_ref();
        let _registering = self.register_lock.lock().await;
        if let Some(existing) = self.hosted_for_root(root)? {
            return Ok(existing);
        }
        self.open_registered_workspace(root, config).await
    }

    /// The existing mount for `root`, matched by canonical form, or `None`
    /// when no tenant owns that path. One read lock; the returned
    /// [`HostedWorkspace`] is rebuilt from the handle captured at mount.
    fn hosted_for_root(&self, root: &Path) -> Result<Option<HostedWorkspace>, Error> {
        let target = canonical_key(root);
        let workspaces = self
            .workspaces
            .read()
            .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
        Ok(workspaces
            .values()
            .find(|runtime| canonical_key(&runtime.root) == target)
            .map(hosted_from_runtime))
    }

    /// Mount an already-open workspace under `config.prefix`.
    pub async fn open_workspace(
        &self,
        workspace: Arc<Workspace>,
        mut config: ServeConfig,
    ) -> Result<HostedWorkspace, Error> {
        config.prefix = sanitize_prefix(&config.prefix).map_err(Error::Config)?;
        let prefix = config.prefix.clone();
        let root = workspace.root().to_path_buf();

        {
            let workspaces = self
                .workspaces
                .read()
                .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
            if workspaces.contains_key(&prefix) {
                return Err(Error::Config(format!(
                    "workspace prefix already mounted: {}",
                    display_prefix(&prefix)
                )));
            }
            if workspaces.values().any(|runtime| runtime.root == root) {
                return Err(Error::Config(format!(
                    "workspace already mounted: {}",
                    root.display()
                )));
            }
        }

        let artifacts = self
            .builder
            .build_workspace(
                self.library.clone(),
                workspace,
                &config,
                self.desktop.clone(),
                self.unserve_mode(),
            )
            .await?;
        // Presence transitions (a window's first socket connecting / last one
        // dropping) shift its `connected` flag with no registry change, so feed
        // the tenant's presence the aggregate signal the watch awaits.
        artifacts
            .window_presence
            .install_change_notify(self.library_change_notify.clone());
        let handle = ServeHandle {
            addr: config.addr,
            prefix: prefix.clone(),
            token: artifacts.token.clone(),
        };
        let hosted = HostedWorkspace {
            root: root.clone(),
            prefix: prefix.clone(),
            handle: handle.clone(),
        };
        let runtime = HostedWorkspaceRuntime {
            root,
            handle,
            artifacts,
        };

        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
        if workspaces.contains_key(&prefix) {
            return Err(Error::Config(format!(
                "workspace prefix already mounted: {}",
                display_prefix(&prefix)
            )));
        }
        workspaces.insert(prefix, runtime);
        drop(workspaces);
        self.notify_window_change();
        Ok(hosted)
    }

    /// Mount a workspace-less "terminal-only" tenant whose terminals run
    /// the user's default interactive shell. Shorthand for
    /// [`open_terminal_session_with_command`](Self::open_terminal_session_with_command)
    /// with no command.
    pub async fn open_terminal_session(
        &self,
        config: ServeConfig,
    ) -> Result<HostedWorkspace, Error> {
        // This is THE library's shared terminal tenant (every standalone
        // terminal window attaches here, sharing its prefix+token). Record its
        // sanitized prefix — matching the `workspaces` map key — so
        // `window_live_state(Terminal)` resolves terminal windows to it. The
        // tenant's PTY/layout is non-persistent (lives in `ephemeral_sessions`,
        // so shells restart on relaunch); the terminal WINDOWS persist as
        // registry rows. Set-once: the shared tenant mounts once per library.
        let prefix = sanitize_prefix(&config.prefix).map_err(Error::Config)?;
        let hosted = self
            .open_terminal_session_with_command(config, None, None)
            .await?;
        let _ = self.terminal_tenant_prefix.set(prefix);
        Ok(hosted)
    }

    /// Mount a workspace-less "terminal-only" tenant under
    /// `config.prefix`, optionally running `command` on its PTY.
    ///
    /// Mirrors [`open_workspace`](Self::open_workspace) but backs the
    /// mount with [`build_terminal_app`] instead of `build_app`: no
    /// `Arc<Workspace>`, no watcher / indexer / MCP bridge / control
    /// socket. The slim tenant serves only the terminal + window-session
    /// routes plus the SPA shell, so a standalone terminal window
    /// (desktop webview in `?kind=terminal` mode) gets a PTY surface
    /// without a workspace behind it.
    ///
    /// `command` is one shell command line, run through the login shell so
    /// an interactive script (host-key / password prompts) gets a real
    /// PTY; `None` keeps the default shell. It is the tenant default, so
    /// every terminal opened in this tenant that names no command of its
    /// own runs it (a single-purpose terminal window, e.g. one that runs a
    /// connect script).
    ///
    /// The tenant lands in the SAME `workspaces` map as workspace mounts
    /// and is reached by the same `host_dispatch` prefix routing, so the
    /// duplicate-prefix guard and `close_workspace` apply uniformly. The
    /// returned [`HostedWorkspace::root`] is the PTY cwd (the user's home
    /// dir) since there is no workspace root; `handle.launch_url()`
    /// resolves against `config.addr`/`prefix`/token exactly like a
    /// workspace mount.
    pub async fn open_terminal_session_with_command(
        &self,
        mut config: ServeConfig,
        command: Option<String>,
        session_dir: Option<PathBuf>,
    ) -> Result<HostedWorkspace, Error> {
        config.prefix = sanitize_prefix(&config.prefix).map_err(Error::Config)?;
        let prefix = config.prefix.clone();

        // Duplicate-prefix guard only: unlike a workspace mount there is
        // no filesystem root to collide on, so two terminal tenants are
        // free to share the home-dir PTY cwd. The check mirrors
        // `open_workspace` so a prefix already serving a workspace can't
        // be shadowed.
        {
            let workspaces = self
                .workspaces
                .read()
                .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
            if workspaces.contains_key(&prefix) {
                return Err(Error::Config(format!(
                    "workspace prefix already mounted: {}",
                    display_prefix(&prefix)
                )));
            }
        }

        // The builder applies `command` as the tenant's default before the SPA
        // can open the first terminal.
        let artifacts = self
            .builder
            .build_terminal(
                self.library.clone(),
                &config,
                self.desktop.clone(),
                self.unserve_mode(),
                command,
                session_dir,
            )
            .await?;
        // Feed the tenant's presence the aggregate change signal (see
        // `open_workspace`); a terminal window's `connected` is presence-driven.
        artifacts
            .window_presence
            .install_change_notify(self.library_change_notify.clone());
        // Root reported for diagnostics / desktop correlation: the PTY
        // cwd is the user's home dir, so surface that. Falls back to "/"
        // to match `build_terminal_app`'s registry root resolution.
        let root = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let handle = ServeHandle {
            addr: config.addr,
            prefix: prefix.clone(),
            token: artifacts.token.clone(),
        };
        let hosted = HostedWorkspace {
            root: root.clone(),
            prefix: prefix.clone(),
            handle: handle.clone(),
        };
        let runtime = HostedWorkspaceRuntime {
            root,
            handle,
            artifacts,
        };

        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
        if workspaces.contains_key(&prefix) {
            return Err(Error::Config(format!(
                "workspace prefix already mounted: {}",
                display_prefix(&prefix)
            )));
        }
        workspaces.insert(prefix, runtime);
        drop(workspaces);
        self.notify_window_change();
        Ok(hosted)
    }

    /// True when the tenant mounted at `prefix` has at least one live
    /// terminal session bound to `window_id` (the desktop window label
    /// the SPA forwards from its `?w=` query param).
    ///
    /// chan-desktop's close handler asks this before letting a
    /// standalone terminal window really close: a window whose shells
    /// are still alive gets hidden ("buried") instead, so the PTYs stay
    /// reachable. Sync and cheap (one read lock + a roster snapshot);
    /// safe to call from the Tauri event-loop thread. `false` when no
    /// tenant is mounted at `prefix`.
    pub fn tenant_has_window_sessions(&self, prefix: &str, window_id: &str) -> bool {
        let Ok(prefix) = sanitize_prefix(prefix) else {
            return false;
        };
        let Ok(workspaces) = self.workspaces.read() else {
            return false;
        };
        let Some(runtime) = workspaces.get(&prefix) else {
            return false;
        };
        runtime
            .artifacts
            .terminal_sessions
            .roster()
            .iter()
            .any(|entry| entry.window_id.as_deref() == Some(window_id))
    }

    /// How many live terminal sessions the tenant mounted at `prefix` is
    /// running, or `0` when nothing is mounted there. The reversible
    /// workspace-off path reads this to refuse an unmount that would kill
    /// running terminals unless the caller forces it. Sync and cheap (one read
    /// lock + a roster snapshot), like
    /// [`tenant_has_window_sessions`](Self::tenant_has_window_sessions).
    pub fn tenant_terminal_session_count(&self, prefix: &str) -> usize {
        let Ok(prefix) = sanitize_prefix(prefix) else {
            return 0;
        };
        let Ok(workspaces) = self.workspaces.read() else {
            return 0;
        };
        workspaces
            .get(&prefix)
            .map(|runtime| runtime.artifacts.terminal_sessions.roster().len())
            .unwrap_or(0)
    }

    /// The full library window set: every window across every tenant, as the
    /// authoritative records the launcher, `cs window list`, and the desktop
    /// watcher reconcile to. Joins each persisted registry row with its serving
    /// tenant's live state: a workspace window carries its mounted tenant's
    /// prefix and token (or, when the workspace is off, a stable derived prefix
    /// with no token), and `connected` reflects a live `/ws` socket for that
    /// window id. Empty when no registry is installed (a host that never opened
    /// one has no windows).
    pub fn assemble_window_records(&self) -> Vec<WindowRecord> {
        let Some(registry) = self.window_registry() else {
            return Vec::new();
        };
        let library_id = self.library_id();
        registry
            .snapshot()
            .into_iter()
            .map(|row| {
                let (prefix, token, connected) = self.window_live_state(&row);
                row.to_record(library_id.to_string(), prefix, token, connected)
            })
            .collect()
    }

    /// Mint a window: persist a new registry row and return its assembled
    /// [`WindowRecord`] (the same shape the feed serves, so a `POST` handler
    /// returns it directly). The registry's create fires the watch via the
    /// bridge; this also fires it directly so the push does not hinge on the
    /// bridge task's scheduling. The tenant side (ensuring a serving tenant for
    /// the new window) is layered on with the D-W3 desktop wiring.
    pub fn mint_window(
        &self,
        kind: WindowKind,
        workspace_path: Option<String>,
    ) -> Result<WindowRecord, Error> {
        let registry = self
            .window_registry()
            .ok_or_else(|| Error::Config("window registry not installed".into()))?;
        let row = registry.create(kind, workspace_path);
        self.notify_window_change();
        let library_id = self.library_id().to_string();
        let (prefix, token, connected) = self.window_live_state(&row);
        Ok(row.to_record(library_id, prefix, token, connected))
    }

    /// Discard a window: drop its registry row, reap its terminal sessions, and
    /// fire the watch. Returns whether a row existed (a `DELETE` handler maps
    /// `false` to 404). The reap is the L5 "discard ⇒ reap" contract: it frees
    /// the fds a busy detached session would otherwise keep alive. A terminal
    /// window's sessions reap once @@Desktop's D-W3 terminal tenant is wired;
    /// until then only that tenant is absent, so the reap is simply a no-op for
    /// terminal windows (workspace windows reap their panes today).
    pub fn discard_window(&self, window_id: &str) -> Result<bool, Error> {
        let registry = self
            .window_registry()
            .ok_or_else(|| Error::Config("window registry not installed".into()))?;
        let removed = registry.remove(window_id);
        if removed {
            self.reap_window_sessions(window_id);
            self.notify_window_change();
        }
        Ok(removed)
    }

    /// Reap every terminal session a discarded `window_id` owns, across all
    /// mounted tenants. The id is library-unique, so only its owning tenant
    /// reaps anything; the rest are no-ops. The tenant registries are cloned out
    /// under the lock and reaped after releasing it, so a PTY teardown never
    /// blocks a concurrent tenant mount/unmount. Returns the count reaped.
    fn reap_window_sessions(&self, window_id: &str) -> usize {
        let registries: Vec<Arc<crate::terminal_sessions::Registry>> = match self.workspaces.read()
        {
            Ok(workspaces) => workspaces
                .values()
                .map(|runtime| runtime.artifacts.terminal_sessions.clone())
                .collect(),
            Err(_) => return 0,
        };
        registries
            .iter()
            .map(|sessions| sessions.forget_window(window_id))
            .sum()
    }

    /// Resolve a persisted window's live `(prefix, token, connected)` from its
    /// serving tenant. A terminal window resolves to the library's shared
    /// terminal tenant (via [`terminal_window_live`](Self::terminal_window_live)),
    /// a workspace window through
    /// [`workspace_window_live`](Self::workspace_window_live).
    fn window_live_state(&self, row: &PersistedWindow) -> (String, String, bool) {
        match row.kind {
            WindowKind::Workspace => {
                self.workspace_window_live(row.workspace_path.as_deref(), &row.window_id)
            }
            WindowKind::Terminal => self.terminal_window_live(&row.window_id),
        }
    }

    /// The `(prefix, token, connected)` for a terminal window: the library's
    /// shared terminal tenant, once mounted. Every terminal window attaches to
    /// the one tenant, so they all share its prefix+token; `connected` reflects
    /// this `window_id`'s live `/ws` presence. Empty until the tenant is mounted
    /// — boot ordering mounts it before the watcher reconciles persisted
    /// terminal windows, so they resolve and reopen on relaunch.
    fn terminal_window_live(&self, window_id: &str) -> (String, String, bool) {
        let Some(prefix) = self.terminal_tenant_prefix.get() else {
            return (String::new(), String::new(), false);
        };
        if let Ok(workspaces) = self.workspaces.read() {
            if let Some(runtime) = workspaces.get(prefix) {
                let connected = runtime
                    .artifacts
                    .window_presence
                    .connected_ids()
                    .iter()
                    .any(|id| id == window_id);
                return (
                    runtime.handle.prefix.clone(),
                    runtime.handle.token.clone().unwrap_or_default(),
                    connected,
                );
            }
        }
        (String::new(), String::new(), false)
    }

    /// The `(prefix, token, connected)` for a workspace window. A mounted
    /// workspace carries its live tenant's prefix and token plus the window's
    /// `/ws` presence; an off workspace carries its stable derived prefix with
    /// no token, since the client turns it on before attaching.
    fn workspace_window_live(
        &self,
        workspace_path: Option<&str>,
        window_id: &str,
    ) -> (String, String, bool) {
        let Some(path) = workspace_path else {
            return (String::new(), String::new(), false);
        };
        let path = Path::new(path);
        let target = canonical_key(path);
        if let Ok(workspaces) = self.workspaces.read() {
            if let Some(runtime) = workspaces
                .values()
                .find(|runtime| canonical_key(&runtime.root) == target)
            {
                let connected = runtime
                    .artifacts
                    .window_presence
                    .connected_ids()
                    .iter()
                    .any(|id| id == window_id);
                return (
                    runtime.handle.prefix.clone(),
                    runtime.handle.token.clone().unwrap_or_default(),
                    connected,
                );
            }
        }
        let prefix = allocate_workspace_prefix(path).unwrap_or_default();
        (prefix, String::new(), false)
    }

    /// Raw replay-ring PTY bytes for the terminal tenant mounted at
    /// `prefix` (empty when none is mounted there). Reaches into that
    /// tenant's terminal registry like [`tenant_has_window_sessions`](
    /// Self::tenant_has_window_sessions). Lets a desktop read a CONTROL
    /// TERMINAL's output to scrape a token a connect script printed, in the
    /// case where that output never reaches the desktop another way.
    pub fn terminal_tenant_scrollback(&self, prefix: &str) -> Vec<u8> {
        let Ok(prefix) = sanitize_prefix(prefix) else {
            return Vec::new();
        };
        let Ok(workspaces) = self.workspaces.read() else {
            return Vec::new();
        };
        workspaces
            .get(&prefix)
            .map(|runtime| runtime.artifacts.terminal_sessions.all_scrollback())
            .unwrap_or_default()
    }

    /// The exit code of the terminal tenant (mounted at `prefix`)'s connect
    /// script once its PTY has exited, or `None` while it runs / when no tenant
    /// is mounted there. Sibling to [`terminal_tenant_scrollback`](
    /// Self::terminal_tenant_scrollback): the desktop polls BOTH while scraping
    /// a control terminal — a token in the scrollback means connected, a
    /// `Some(code)` here means the script died, so the scrape can stop at once
    /// (instead of waiting out the full timeout) and a tab closed mid-connect
    /// can survey on a real failure instead of stranding an empty window.
    pub fn terminal_tenant_last_exit(&self, prefix: &str) -> Option<u32> {
        let prefix = sanitize_prefix(prefix).ok()?;
        let workspaces = self.workspaces.read().ok()?;
        workspaces
            .get(&prefix)
            .and_then(|runtime| runtime.artifacts.terminal_sessions.last_exit_code())
    }

    /// Close the mounted workspace whose root matches `root` (by canonical
    /// form), returning whether one was found and closed. The control-socket
    /// `Unserve` handler uses this to unmount a single hosted tenant by path
    /// without disturbing the rest of the host. A terminal tenant (no
    /// workspace root) never matches a real workspace root.
    pub fn close_workspace_for_root(&self, root: &Path) -> Result<bool, Error> {
        let target = canonical_key(root);
        let prefix = {
            let workspaces = self
                .workspaces
                .read()
                .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
            workspaces
                .values()
                .find(|runtime| canonical_key(&runtime.root) == target)
                .map(|runtime| runtime.handle.prefix.clone())
        };
        match prefix {
            Some(prefix) => self.close_workspace(&prefix),
            None => Ok(false),
        }
    }

    /// Close the workspace mounted at `prefix`.
    ///
    /// Returns `Ok(false)` when no workspace is mounted there. Closing
    /// sends the shared shutdown signal before dropping the runtime,
    /// so active WebSockets and terminal sessions get a clean exit
    /// path.
    ///
    /// This does NOT synchronously reap the tenant's PTYs: the shutdown
    /// signal lets the per-tenant prune task close them on its own schedule.
    /// That is fine for a workspace tenant (the devserver only mounts
    /// workspaces through this), but a terminal-only tenant whose PTY must
    /// stop at once — a control terminal running a connect script — should be
    /// closed with [`close_terminal_tenant`](Self::close_terminal_tenant).
    pub fn close_workspace(&self, prefix: &str) -> Result<bool, Error> {
        let prefix = sanitize_prefix(prefix).map_err(Error::Config)?;
        let runtime = {
            let mut workspaces = self
                .workspaces
                .write()
                .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
            workspaces.remove(&prefix)
        };
        let Some(runtime) = runtime else {
            return Ok(false);
        };
        // Tear down explicitly (rather than leaving it to Drop) so we hold a
        // `Weak` to the workspace and can wait for the per-workspace flock to
        // release before returning. Without this an in-process close then
        // immediate reopen of the same root races teardown and trips
        // `WorkspaceAlreadyOpen`. Drop re-runs shutdown on the now-cleared
        // cell, which is a no-op.
        let released = runtime.shutdown();
        drop(runtime);
        if let Some((weak, lock_dir)) = released {
            wait_for_workspace_release(&weak, &lock_dir);
        }
        self.notify_window_change();
        Ok(true)
    }

    /// Close the terminal-only tenant mounted at `prefix`, reaping its PTYs.
    ///
    /// Returns `Ok(false)` when nothing is mounted there. chan-desktop calls
    /// this on Disconnect AND Forget of a scripted devserver: destroying the
    /// webview window alone leaves the control terminal's connect script
    /// RUNNING on the host, because the tenant (mounted via
    /// [`open_terminal_session_with_command`](Self::open_terminal_session_with_command))
    /// outlives the window that drove it.
    ///
    /// This explicitly `close_all`s the tenant's terminal registry so every
    /// PTY child is sent its `Kill` synchronously — the script process is
    /// gone by the time this returns — rather than leaning on the per-tenant
    /// prune task to later observe the shutdown signal. The shared shutdown
    /// signal then stops the accept loops and background tasks before the
    /// runtime drops. The flock-release tail mirrors
    /// [`close_workspace`](Self::close_workspace) so pointing this at a
    /// workspace tenant by mistake still tears it down race-free; a terminal
    /// tenant has no workspace cell, so that wait is skipped.
    pub fn close_terminal_tenant(&self, prefix: &str) -> Result<bool, Error> {
        let prefix = sanitize_prefix(prefix).map_err(Error::Config)?;
        let runtime = {
            let mut workspaces = self
                .workspaces
                .write()
                .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
            workspaces.remove(&prefix)
        };
        let Some(runtime) = runtime else {
            return Ok(false);
        };
        // Reap every PTY now: an explicit Disconnect/Forget must stop the
        // connect script's process at once, not whenever the prune task next
        // wakes on the shutdown signal. `close_all` drains the session map
        // and Kills each child; it is idempotent, so the shutdown-driven
        // prune that follows is a no-op.
        runtime
            .artifacts
            .terminal_sessions
            .close_all(CloseReason::Shutdown);
        let released = runtime.shutdown();
        drop(runtime);
        if let Some((weak, lock_dir)) = released {
            wait_for_workspace_release(&weak, &lock_dir);
        }
        self.notify_window_change();
        Ok(true)
    }

    /// Cancel any in-flight reindex on every mounted tenant.
    ///
    /// On shutdown the devserver calls this so each tenant's blocking reindex
    /// drops its `Arc<Workspace>` at the next per-file cancel check, releasing
    /// the per-workspace flock promptly instead of waiting for the rebuild to
    /// run to completion. Mirrors the single-tenant `indexer.cancel()` in
    /// [`clear_workspace_cell`]. Read-only over the map and best-effort per
    /// tenant: a poisoned cell or a terminal tenant (no workspace cell) is
    /// skipped.
    pub fn cancel_all_reindex(&self) {
        let Ok(workspaces) = self.workspaces.read() else {
            return;
        };
        for runtime in workspaces.values() {
            runtime.artifacts.cell.cancel_reindex();
        }
    }

    /// Snapshot the mounted prefixes.
    pub fn mounted_prefixes(&self) -> Result<Vec<String>, Error> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
        let mut prefixes: Vec<String> = workspaces.keys().cloned().collect();
        prefixes.sort();
        Ok(prefixes)
    }

    /// Build a dynamic router for all mounted workspaces.
    ///
    /// The returned router consults the host map on every request, so
    /// later `open_*` and `close_workspace` calls are visible without
    /// rebuilding the outer axum app.
    pub fn router(self: Arc<Self>) -> Router {
        Router::new().fallback(host_dispatch).with_state(self)
    }

    /// Return the live `Arc<Workspace>` for a mounted workspace whose root
    /// matches `root`, or `None` when no mounted runtime owns that
    /// path.
    ///
    /// Desktop feature toggles need the SAME handle the runtime
    /// holds: a second `Library::open_workspace` for a mounted path
    /// returns `WorkspaceAlreadyOpen` because `Workspace::open` keeps a
    /// lifetime flock. Comparison is by canonical form so a
    /// symlinked or non-normalized caller path still matches the
    /// canonical root the runtime stored at mount time. Lock
    /// poisoning and a drained workspace cell both read as "not live"
    /// (mirrors `AppState::try_workspace`); the caller then falls back
    /// to a transient open against the registry.
    pub fn live_workspace(&self, root: &Path) -> Option<Arc<Workspace>> {
        let target = canonical_key(root);
        let workspaces = self.workspaces.read().ok()?;
        let runtime = workspaces
            .values()
            .find(|runtime| canonical_key(&runtime.root) == target)?;
        runtime.artifacts.cell.workspace()
    }

    fn router_for_path(&self, path: &str) -> Result<Option<Router>, Error> {
        let workspaces = self
            .workspaces
            .read()
            .map_err(|_| Error::Config("workspace host lock poisoned".into()))?;
        Ok(workspaces
            .iter()
            .filter(|(prefix, _)| path_matches_prefix(path, prefix))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, runtime)| runtime.router()))
    }
}

/// The control socket reaches the host through `Weak<dyn HostControl>` (the
/// `install_self` back-reference), so it never names the concrete host type.
impl HostControl for WorkspaceHost {
    fn close_workspace_for_root(&self, root: &Path) -> Result<bool, Error> {
        self.close_workspace_for_root(root)
    }

    fn assemble_window_records(&self) -> Vec<WindowRecord> {
        self.assemble_window_records()
    }
}

async fn host_dispatch(State(host): State<Arc<WorkspaceHost>>, req: Request<Body>) -> Response {
    let Some(router) = (match host.router_for_path(req.uri().path()) {
        Ok(router) => router,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    match router.oneshot(req).await {
        Ok(response) => response,
        Err(e) => match e {},
    }
}

fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Rebuild the public [`HostedWorkspace`] view of a mounted runtime from
/// the handle captured at mount time, for the idempotent re-register
/// return and any tenant listing.
fn hosted_from_runtime(runtime: &HostedWorkspaceRuntime) -> HostedWorkspace {
    HostedWorkspace {
        root: runtime.root.clone(),
        prefix: runtime.handle.prefix.clone(),
        handle: runtime.handle.clone(),
    }
}

/// Canonical-form key for matching a caller path against a mounted
/// runtime's root. Falls back to the input path when the filesystem
/// can't canonicalize (workspace root missing or asleep), so the match
/// still works on the exact request path. Mirrors the private
/// `canonical_key` in `chan_workspace::library`.
fn canonical_key(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

fn display_prefix(prefix: &str) -> &str {
    if prefix.is_empty() {
        "/"
    } else {
        prefix
    }
}

// The workspace-cell teardown (cancel indexer, drop watcher + workspace, return
// the flock-release info) lives in the route layer's `WorkspaceCellHandle::clear`
// impl, which owns the concrete cell; the host reaches it via `artifacts.cell`.

/// Block (bounded) until the last strong `Arc<Workspace>` drops after
/// teardown, which releases the per-workspace flock. The straggler is an
/// in-flight reindex on the blocking pool: `clear_workspace_cell` set the
/// indexer's cancel flag, and the reindex drops its `Arc` at its next
/// per-file cancel check, on a separate blocking-pool thread that makes
/// progress regardless of this wait. Close is an infrequent teardown and
/// the wait is typically a few milliseconds. Bounded so a wedged reindex
/// cannot hang close: past the deadline the caller sees the same
/// lingering-flock behavior it would have had without the wait.
fn wait_for_workspace_release(weak: &Weak<Workspace>, lock_dir: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    // Two conditions, not one: the last strong `Arc` must drop, AND the
    // per-workspace flock must actually release. An `Arc`'s strong count hits
    // zero *before* `Workspace::drop` runs the `_lock` field's drop, so
    // `strong_count()==0` alone leaves a window where an immediate reopen
    // races the in-flight Drop and trips `WorkspaceLocked` with no live
    // competitor (the on→off→on wedge). `lock::is_free` try-acquires the flock
    // (and releases it), proving the prior holder's Drop completed.
    while weak.strong_count() > 0 || !chan_workspace::lock::is_free(lock_dir) {
        if Instant::now() >= deadline {
            tracing::warn!("close_workspace: workspace flock still held 5s after teardown");
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant::WorkspaceCellHandle;
    use crate::terminal_sessions::CreateOptions;
    use axum::body::to_bytes;
    use portable_pty::PtySize;

    fn serve_config(prefix: &str) -> ServeConfig {
        ServeConfig {
            addr: ([127, 0, 0, 1], 0).into(),
            no_token: true,
            prefix: prefix.to_string(),
            idle_timeout: None,
            open_browser: false,
            search_aggression: None,
            verbose: false,
            settings_disabled: false,
            tunnel_public: false,
        }
    }

    async fn response_json(response: Response) -> serde_json::Value {
        let (parts, body) = response.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let bytes = to_bytes(body, 1024 * 1024).await.expect("read body");
        serde_json::from_slice(&bytes).expect("json response")
    }

    /// A minimal `TenantBuilder` standing in for chan-server's route layer:
    /// a tiny router (`/api/workspace` echoes the root, `/api/build-info` 200,
    /// nothing else → 404), a real terminal registry, and a fake cell that
    /// holds the real `Arc<Workspace>` so the flock-release lifecycle behaves
    /// like the route layer's. Lets the host's dispatch + teardown be unit-
    /// tested in isolation.
    struct FakeBuilder;

    fn fake_builder() -> Arc<dyn TenantBuilder> {
        Arc::new(FakeBuilder)
    }

    fn fake_registry() -> Arc<crate::terminal_sessions::Registry> {
        Arc::new(crate::terminal_sessions::Registry::new(
            crate::terminal_sessions::RegistryConfig {
                workspace_root: PathBuf::from("/"),
                mcp_socket_path: None,
                control_socket_path: None,
                terminal: crate::config::TerminalConfig::default(),
            },
        ))
    }

    fn fake_artifacts(app: Router, cell: Arc<dyn WorkspaceCellHandle>) -> TenantArtifacts {
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        TenantArtifacts {
            app,
            token: None,
            terminal_sessions: fake_registry(),
            shutdown_tx: Arc::new(shutdown_tx),
            prefix: Arc::new(RwLock::new(String::new())),
            window_presence: Arc::new(crate::window_presence::WindowPresence::new()),
            cell,
            keepalive: Box::new(()),
        }
    }

    fn nest(prefix: &str, inner: Router) -> Router {
        if prefix.is_empty() {
            inner
        } else {
            Router::new().nest(prefix, inner)
        }
    }

    struct FakeWorkspaceCell(std::sync::Mutex<Option<Arc<Workspace>>>);
    impl WorkspaceCellHandle for FakeWorkspaceCell {
        fn workspace(&self) -> Option<Arc<Workspace>> {
            self.0.lock().ok()?.clone()
        }
        fn cancel_reindex(&self) {}
        fn clear(&self) -> Option<(Weak<Workspace>, PathBuf)> {
            let ws = self.0.lock().ok()?.take()?;
            let lock_dir = ws.paths().lock.clone();
            let weak = Arc::downgrade(&ws);
            drop(ws);
            Some((weak, lock_dir))
        }
    }

    struct FakeTerminalCell;
    impl WorkspaceCellHandle for FakeTerminalCell {
        fn workspace(&self) -> Option<Arc<Workspace>> {
            None
        }
        fn cancel_reindex(&self) {}
        fn clear(&self) -> Option<(Weak<Workspace>, PathBuf)> {
            None
        }
    }

    #[async_trait::async_trait]
    impl TenantBuilder for FakeBuilder {
        async fn build_workspace(
            &self,
            _library: Library,
            workspace: Arc<Workspace>,
            config: &ServeConfig,
            _desktop: DesktopBridge,
            _unserve: UnserveMode,
        ) -> Result<TenantArtifacts, Error> {
            let root = workspace.root().to_string_lossy().to_string();
            let inner = Router::new()
                .route(
                    "/api/workspace",
                    axum::routing::get(move || {
                        let root = root.clone();
                        async move { axum::Json(serde_json::json!({ "root": root })) }
                    }),
                )
                .route(
                    "/api/build-info",
                    axum::routing::get(|| async { StatusCode::OK }),
                );
            let cell: Arc<dyn WorkspaceCellHandle> =
                Arc::new(FakeWorkspaceCell(std::sync::Mutex::new(Some(workspace))));
            Ok(fake_artifacts(nest(&config.prefix, inner), cell))
        }

        async fn build_terminal(
            &self,
            _library: Library,
            config: &ServeConfig,
            _desktop: DesktopBridge,
            _unserve: UnserveMode,
            command: Option<String>,
            _session_dir: Option<PathBuf>,
        ) -> Result<TenantArtifacts, Error> {
            let inner = Router::new().route(
                "/api/build-info",
                axum::routing::get(|| async { StatusCode::OK }),
            );
            let artifacts = fake_artifacts(nest(&config.prefix, inner), Arc::new(FakeTerminalCell));
            artifacts.terminal_sessions.set_default_command(command);
            Ok(artifacts)
        }
    }

    #[tokio::test]
    async fn host_routes_requests_to_the_matching_workspace_prefix() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root_a = tempfile::tempdir().expect("workspace a");
        let root_b = tempfile::tempdir().expect("workspace b");
        std::fs::write(root_a.path().join("a.md"), "# A\n").expect("write a");
        std::fs::write(root_b.path().join("b.md"), "# B\n").expect("write b");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root_a.path()).expect("register a");
        lib.register_workspace(root_b.path()).expect("register b");
        let host = Arc::new(WorkspaceHost::new(lib.clone(), fake_builder()));

        host.open_registered_workspace(root_a.path(), serve_config("/a"))
            .await
            .expect("open a");
        host.open_registered_workspace(root_b.path(), serve_config("/b"))
            .await
            .expect("open b");

        let app = host.router();
        let a = response_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/a/api/workspace")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        let b = response_json(
            app.oneshot(
                Request::builder()
                    .uri("/b/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;

        let root_a = root_a.path().canonicalize().expect("canonical a");
        let root_b = root_b.path().canonicalize().expect("canonical b");
        assert_eq!(a["root"], root_a.to_string_lossy().as_ref());
        assert_eq!(b["root"], root_b.to_string_lossy().as_ref());
    }

    #[tokio::test]
    async fn host_close_workspace_removes_the_route() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib.clone(), fake_builder()));
        host.open_registered_workspace(root.path(), serve_config("/workspace"))
            .await
            .expect("open");
        let app = host.clone().router();

        assert!(host.close_workspace("/workspace").expect("close"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/workspace/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn host_close_workspace_releases_handle_for_immediate_reopen() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib.clone(), fake_builder()));

        host.open_registered_workspace(root.path(), serve_config("/first"))
            .await
            .expect("open first");
        assert!(host.close_workspace("/first").expect("close first"));

        host.open_registered_workspace(root.path(), serve_config("/second"))
            .await
            .expect("reopen after close");
    }

    #[tokio::test]
    async fn close_workspace_releases_the_flock_before_returning() {
        // The on→off→on wedge fix: `close_workspace` must not return until the
        // per-workspace flock is genuinely released, not merely once the last
        // `Arc`'s strong count hits zero (which happens BEFORE `Workspace::drop`
        // runs the `_lock` drop). Asserted deterministically: the flock is free
        // the instant close returns.
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));
        host.open_registered_workspace(root.path(), serve_config("/ws"))
            .await
            .expect("open");

        // Capture the lock dir from the live workspace; while mounted, held.
        let lock_dir = host
            .live_workspace(root.path())
            .expect("live workspace")
            .paths()
            .lock
            .clone();
        assert!(
            !chan_workspace::lock::is_free(&lock_dir),
            "flock is held while the workspace is mounted"
        );

        assert!(host.close_workspace("/ws").expect("close"));
        assert!(
            chan_workspace::lock::is_free(&lock_dir),
            "flock is free the moment close_workspace returns"
        );
    }

    #[tokio::test]
    async fn close_workspace_for_root_unmounts_by_path() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));
        host.open_registered_workspace(root.path(), serve_config("/ws"))
            .await
            .expect("open");

        // The `chan unserve` host path: unmount the matching tenant by root.
        assert!(host
            .close_workspace_for_root(root.path())
            .expect("close by root"));
        assert!(host.mounted_prefixes().expect("prefixes").is_empty());

        // An already-unmounted root and an unknown root both report false
        // (no panic, no error) so unserve is idempotent / 404-able.
        assert!(!host
            .close_workspace_for_root(root.path())
            .expect("absent root"));
        let other = tempfile::tempdir().expect("other");
        assert!(!host
            .close_workspace_for_root(other.path())
            .expect("unknown root"));
    }

    #[tokio::test]
    async fn live_workspace_returns_the_mounted_runtime_handle() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib.clone(), fake_builder()));
        host.open_registered_workspace(root.path(), serve_config("/workspace"))
            .await
            .expect("open");

        // The live handle must be the SAME Arc the runtime holds, so a
        // feature toggle off it reaches the flock-holding workspace rather
        // than tripping WorkspaceAlreadyOpen on a re-open.
        let live = host
            .live_workspace(root.path())
            .expect("live workspace present");
        let canonical = root.path().canonicalize().expect("canonical root");
        assert_eq!(live.root(), canonical.as_path());

        // A path that no runtime mounts reads as not live.
        let other = tempfile::tempdir().expect("other dir");
        assert!(host.live_workspace(other.path()).is_none());

        // After close, the handle is no longer live.
        assert!(host.close_workspace("/workspace").expect("close"));
        assert!(host.live_workspace(root.path()).is_none());
    }

    #[tokio::test]
    async fn fresh_in_process_register_opens_without_workspace_not_registered() {
        // Regression guard for the desktop "workspace not registered" bug:
        // chan-desktop used to register a brand-new directory by
        // spawning `chan add` in a SEPARATE process, which mutated only
        // the on-disk registry. The embedded host's `Library` snapshot
        // never saw the row, so the immediately-following open returned
        // WorkspaceNotRegistered. Registering in-process through the SAME
        // `Library` the host owns makes the row visible at once: this
        // test registers a never-before-seen dir, then opens it on the
        // same handle with no intervening reload.
        let cfg = tempfile::tempdir().expect("config dir");
        let fresh = tempfile::tempdir().expect("fresh workspace dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(fresh.path())
            .expect("register fresh dir");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        let hosted = host
            .open_registered_workspace(fresh.path(), serve_config("/fresh"))
            .await
            .expect("fresh dir opens immediately after in-process register");
        let canonical = fresh.path().canonicalize().expect("canonical root");
        assert_eq!(hosted.root, canonical);
    }

    #[tokio::test]
    async fn open_or_get_re_register_is_idempotent_on_root() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        let first = host
            .open_or_get_registered_workspace(root.path(), serve_config("/first"))
            .await
            .expect("first mount");
        assert_eq!(first.prefix, "/first");

        // Re-registering the same root returns the EXISTING mount (its
        // original prefix + token) without re-opening the flocked
        // workspace, even when a different prefix is requested.
        let again = host
            .open_or_get_registered_workspace(root.path(), serve_config("/second"))
            .await
            .expect("idempotent re-register");
        assert_eq!(
            again.prefix, "/first",
            "existing prefix, not the requested /second"
        );
        assert_eq!(
            again.handle.token, first.handle.token,
            "same tenant, same token"
        );
        assert_eq!(
            host.mounted_prefixes().expect("prefixes"),
            vec!["/first".to_string()],
            "still one tenant"
        );
    }

    #[tokio::test]
    async fn open_or_get_duplicate_prefix_on_different_root_still_errors() {
        let cfg = tempfile::tempdir().expect("config dir");
        let a = tempfile::tempdir().expect("ws a");
        let b = tempfile::tempdir().expect("ws b");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(a.path()).expect("register a");
        lib.register_workspace(b.path()).expect("register b");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        host.open_or_get_registered_workspace(a.path(), serve_config("/shared"))
            .await
            .expect("mount a at /shared");

        // A DIFFERENT root requesting an already-taken prefix is a genuine
        // collision (not the same-root idempotent case) and still errors.
        let err = host
            .open_or_get_registered_workspace(b.path(), serve_config("/shared"))
            .await
            .expect_err("duplicate prefix on a different root must error");
        assert!(matches!(err, Error::Config(_)));
    }

    #[tokio::test]
    async fn open_or_get_concurrent_same_root_resolves_to_one_mount() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        // Two callers race the same fresh root. The registration lock
        // serializes them: one mounts, the other observes that mount in the
        // pre-check. Both resolve to a single tenant with the same prefix.
        let (a, b) = tokio::join!(
            host.open_or_get_registered_workspace(root.path(), serve_config("/race")),
            host.open_or_get_registered_workspace(root.path(), serve_config("/race")),
        );
        assert_eq!(a.expect("first resolves").prefix, "/race");
        assert_eq!(b.expect("second resolves").prefix, "/race");
        assert_eq!(
            host.mounted_prefixes().expect("prefixes"),
            vec!["/race".to_string()],
            "exactly one tenant mounted despite the race"
        );
    }

    #[tokio::test]
    async fn open_terminal_session_mounts_slim_tenant() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        // No workspace path, no registration: a terminal tenant is
        // backed by nothing but the embedded host.
        host.open_terminal_session(serve_config("/terminal-x"))
            .await
            .expect("open terminal session");

        let app = host.router();

        // A workspace-free terminal-surface route is mounted and
        // reachable. `build-info` is state-free, so it serves 200 even
        // with no workspace cell. (`/api/health` is mounted too but
        // reports 503 on a terminal tenant since it snapshots the
        // absent indexer — mounted, but workspace-dependent.)
        let build_info = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/terminal-x/api/build-info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(build_info.status(), StatusCode::OK);

        // A workspace-content route is ABSENT (the slim router never
        // mounted it), so it 404s rather than panicking on the missing
        // workspace cell.
        let files = app
            .oneshot(
                Request::builder()
                    .uri("/terminal-x/api/files")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(files.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn open_terminal_session_rejects_duplicate_prefix() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        host.open_terminal_session(serve_config("/terminal-1"))
            .await
            .expect("open first terminal");
        // Same prefix is refused by the shared duplicate-prefix guard.
        let err = host
            .open_terminal_session(serve_config("/terminal-1"))
            .await
            .expect_err("duplicate prefix must be rejected");
        assert!(matches!(err, Error::Config(_)));
    }

    #[tokio::test]
    async fn open_terminal_session_with_command_mounts_tenant() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        // A command-carrying terminal tenant mounts like a default-shell
        // one; the command becomes the tenant's PTY default (the running of
        // it is covered by the registry tests).
        host.open_terminal_session_with_command(
            serve_config("/terminal-cmd"),
            Some("printf hi".into()),
            None,
        )
        .await
        .expect("open terminal session with command");

        let app = host.router();
        let build_info = app
            .oneshot(
                Request::builder()
                    .uri("/terminal-cmd/api/build-info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(build_info.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn terminal_tenant_scrollback_empty_when_no_output() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        // No tenant mounted at the prefix -> empty (no panic).
        assert!(host.terminal_tenant_scrollback("/absent").is_empty());

        // A mounted terminal tenant with no session opened yet -> empty.
        // (The actual byte capture is covered by the registry's
        // `all_scrollback` test, which drives a session.)
        host.open_terminal_session(serve_config("/term-sb"))
            .await
            .expect("open terminal tenant");
        assert!(host.terminal_tenant_scrollback("/term-sb").is_empty());
    }

    #[tokio::test]
    async fn close_terminal_tenant_reaps_ptys_and_frees_the_prefix() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));

        host.open_terminal_session(serve_config("/control"))
            .await
            .expect("open terminal tenant");

        // Start a real PTY on the tenant's registry, standing in for the
        // connect script the desktop runs in a control terminal. Reach the
        // registry through the host's private map (same crate module).
        let registry = {
            let workspaces = host.workspaces.read().expect("host lock");
            workspaces
                .get("/control")
                .expect("tenant mounted")
                .artifacts
                .terminal_sessions
                .clone()
        };
        registry
            .create(CreateOptions {
                size: PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .expect("spawn control PTY");
        assert_eq!(registry.len(), 1, "one live PTY before close");

        // Closing the tenant reaps the PTY synchronously: the registry is
        // already drained on return, not eventually via the prune task. This
        // is the D4 defense — an explicit Disconnect must stop the script now.
        assert!(host
            .close_terminal_tenant("/control")
            .expect("close terminal tenant"));
        assert_eq!(registry.len(), 0, "PTY reaped on tenant close");

        // The prefix tore down cleanly and can be re-mounted at once.
        host.open_terminal_session(serve_config("/control"))
            .await
            .expect("remount after close");

        // An absent prefix is a no-op false, so Disconnect/Forget is
        // idempotent (a second teardown call doesn't error).
        assert!(!host
            .close_terminal_tenant("/absent")
            .expect("absent close is false"));
    }

    #[tokio::test]
    async fn tenant_terminal_session_count_tracks_live_ptys() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));
        host.open_terminal_session(serve_config("/count"))
            .await
            .expect("open terminal tenant");

        // A mounted tenant with no PTYs and an absent prefix both count zero.
        assert_eq!(host.tenant_terminal_session_count("/count"), 0);
        assert_eq!(host.tenant_terminal_session_count("/absent"), 0);

        // A live PTY lifts the count: this is what the off path consults to
        // refuse a terminal-killing unmount unless the caller forces it.
        let registry = {
            let workspaces = host.workspaces.read().expect("host lock");
            workspaces
                .get("/count")
                .expect("tenant mounted")
                .artifacts
                .terminal_sessions
                .clone()
        };
        registry
            .create(CreateOptions {
                size: PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .expect("spawn PTY");
        assert_eq!(host.tenant_terminal_session_count("/count"), 1);
    }

    #[tokio::test]
    async fn terminal_tenant_last_exit_reports_the_script_exit_code() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));
        host.open_terminal_session(serve_config("/ctl"))
            .await
            .expect("open terminal tenant");

        // No PTY spawned yet (and an absent tenant): nothing has exited.
        assert!(host.terminal_tenant_last_exit("/ctl").is_none());
        assert!(host.terminal_tenant_last_exit("/absent").is_none());

        // A PTY that exits non-zero stands in for a FAILING connect script.
        let registry = {
            let workspaces = host.workspaces.read().expect("host lock");
            workspaces
                .get("/ctl")
                .expect("tenant mounted")
                .artifacts
                .terminal_sessions
                .clone()
        };
        registry
            .create(CreateOptions {
                size: PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: Some("exit 7".to_string()),
                env: Default::default(),
            })
            .expect("spawn exiting PTY");

        // Poll the exit code the way the desktop scrape loop would.
        let mut code = None;
        for _ in 0..120 {
            if let Some(c) = host.terminal_tenant_last_exit("/ctl") {
                code = Some(c);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert_eq!(code, Some(7), "the failing script's exit code surfaces");
    }

    #[tokio::test]
    async fn cancel_all_reindex_is_safe_across_tenants() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root_a = tempfile::tempdir().expect("workspace a");
        let root_b = tempfile::tempdir().expect("workspace b");
        std::fs::write(root_a.path().join("a.md"), "# A\n").expect("write a");
        std::fs::write(root_b.path().join("b.md"), "# B\n").expect("write b");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root_a.path()).expect("register a");
        lib.register_workspace(root_b.path()).expect("register b");
        let host = Arc::new(WorkspaceHost::new(lib, fake_builder()));
        host.open_registered_workspace(root_a.path(), serve_config("/a"))
            .await
            .expect("open a");
        host.open_registered_workspace(root_b.path(), serve_config("/b"))
            .await
            .expect("open b");
        // A terminal tenant has no workspace cell; cancelling must skip it
        // without panicking.
        host.open_terminal_session(serve_config("/term"))
            .await
            .expect("open terminal tenant");

        // Cancels every workspace tenant's indexer across the map.
        host.cancel_all_reindex();

        // Idempotent: still a no-op after a tenant is closed, and with an
        // empty map.
        assert!(host.close_workspace("/a").expect("close a"));
        host.cancel_all_reindex();
    }

    #[test]
    fn path_prefix_matching_uses_segment_boundaries() {
        assert!(path_matches_prefix("/workspace", "/workspace"));
        assert!(path_matches_prefix(
            "/workspace/api/workspace",
            "/workspace"
        ));
        assert!(!path_matches_prefix(
            "/driveway/api/workspace",
            "/workspace"
        ));
        assert!(path_matches_prefix("/anything", ""));
    }

    #[test]
    fn assemble_window_records_joins_registry_with_live_state() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = WorkspaceHost::new(lib, fake_builder());

        // No registry installed: the window set is empty.
        assert!(host.assemble_window_records().is_empty());

        // Install a registry + identity, then mint a terminal and an (unmounted)
        // workspace window.
        let store = tempfile::tempdir().expect("store dir");
        let registry = Arc::new(WindowRegistry::open(store.path().join("windows.json")));
        let term = registry.create(WindowKind::Terminal, None);
        let ws = registry.create(WindowKind::Workspace, Some("/tmp/notes".into()));
        host.install_window_registry(registry, "lib-abc".into());

        let records = host.assemble_window_records();
        assert_eq!(records.len(), 2);
        // Every row is stamped with the library id and is persisted.
        assert!(records
            .iter()
            .all(|r| r.library_id == "lib-abc" && r.persisted));

        // No terminal tenant is mounted in this test, so the terminal window
        // has no live prefix/token and is not connected; durable fields carry
        // through. (When the shared tenant IS mounted it resolves — see
        // `assemble_resolves_a_terminal_window_to_the_shared_tenant`.)
        let term_rec = records
            .iter()
            .find(|r| r.window_id == term.window_id)
            .expect("terminal row");
        assert_eq!(term_rec.kind, WindowKind::Terminal);
        assert_eq!(term_rec.prefix, "");
        assert_eq!(term_rec.token, "");
        assert!(!term_rec.connected);
        assert_eq!(term_rec.title, term.title);

        // An off (unmounted) workspace carries its stable derived prefix, no
        // token, not connected.
        let ws_rec = records
            .iter()
            .find(|r| r.window_id == ws.window_id)
            .expect("workspace row");
        assert_eq!(ws_rec.kind, WindowKind::Workspace);
        assert_eq!(ws_rec.workspace_path.as_deref(), Some("/tmp/notes"));
        assert_eq!(
            ws_rec.prefix,
            allocate_workspace_prefix(Path::new("/tmp/notes")).unwrap()
        );
        assert_eq!(ws_rec.token, "");
        assert!(!ws_rec.connected);
    }

    #[tokio::test]
    async fn assemble_resolves_a_terminal_window_to_the_shared_tenant() {
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let host = WorkspaceHost::new(lib, fake_builder());

        // Mount the shared terminal tenant (records its prefix), install a
        // registry, then mint a terminal window.
        host.open_terminal_session(serve_config("/terminal"))
            .await
            .expect("open terminal session");
        let store = tempfile::tempdir().expect("store dir");
        let registry = Arc::new(WindowRegistry::open(store.path().join("windows.json")));
        let term = registry.create(WindowKind::Terminal, None);
        host.install_window_registry(registry, "local".into());

        // The terminal window now resolves to the shared tenant's prefix — the
        // old empty stub is gone — so the desktop watcher can open it. (token is
        // empty here only because the test serve_config sets no_token; in
        // production the tenant carries a token so should_show opens the window.)
        let records = host.assemble_window_records();
        let term_rec = records
            .iter()
            .find(|r| r.window_id == term.window_id)
            .expect("terminal row");
        assert_eq!(
            term_rec.prefix, "/terminal",
            "terminal resolves to the shared tenant prefix",
        );
        assert!(
            !term_rec.connected,
            "no live /ws socket for this window yet"
        );
    }

    #[test]
    fn mint_and_discard_window_round_trip() {
        // Minting before a registry is installed is an error, not a panic.
        let cfg = tempfile::tempdir().expect("config dir");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        let no_reg = WorkspaceHost::new(lib, fake_builder());
        assert!(no_reg.mint_window(WindowKind::Terminal, None).is_err());

        let cfg2 = tempfile::tempdir().expect("config dir");
        let lib2 = Library::open_at(cfg2.path().join("config.toml")).expect("library");
        let host = WorkspaceHost::new(lib2, fake_builder());
        let store = tempfile::tempdir().expect("store dir");
        let registry = Arc::new(WindowRegistry::open(store.path().join("windows.json")));
        host.install_window_registry(registry, "lib-mint".into());

        // Mint returns the assembled record, and the row lands in the feed.
        let term = host
            .mint_window(WindowKind::Terminal, None)
            .expect("mint terminal");
        assert_eq!(term.kind, WindowKind::Terminal);
        assert_eq!(term.library_id, "lib-mint");
        assert!(term.persisted);
        let ws = host
            .mint_window(WindowKind::Workspace, Some("/tmp/notes".into()))
            .expect("mint workspace");
        assert_eq!(ws.workspace_path.as_deref(), Some("/tmp/notes"));

        let mut ids: Vec<String> = host
            .assemble_window_records()
            .into_iter()
            .map(|r| r.window_id)
            .collect();
        ids.sort();
        let mut expected = vec![term.window_id.clone(), ws.window_id.clone()];
        expected.sort();
        assert_eq!(ids, expected);

        // Discard drops the row and reports it existed; a second discard is a
        // no-op (the handler's 404 path), as is an unknown id.
        assert!(host.discard_window(&term.window_id).expect("discard"));
        assert!(!host.discard_window(&term.window_id).expect("re-discard"));
        assert!(!host.discard_window("w-doesnotexist0000").expect("unknown"));

        let remaining: Vec<String> = host
            .assemble_window_records()
            .into_iter()
            .map(|r| r.window_id)
            .collect();
        assert_eq!(remaining, vec![ws.window_id]);
    }
}
