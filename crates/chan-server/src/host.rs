//! Multi-workspace host runtime.
//!
//! `WorkspaceHost` is the in-process owner that chan-desktop can embed
//! instead of spawning one `chan serve` child per local workspace. Each
//! mounted workspace still gets its own `AppState`, watcher, indexer,
//! MCP bridge, control socket, terminal registry, and route prefix.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Weak};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use chan_workspace::{Library, Workspace};
use tower::ServiceExt;

use crate::desktop_window_ops::DesktopBridge;
use crate::state::WorkspaceCell;
use crate::{
    build_app, build_terminal_app, sanitize_prefix, AppArtifacts, Error, ServeConfig, ServeHandle,
};

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
}

struct HostedWorkspaceRuntime {
    root: PathBuf,
    /// Launch handle captured at mount time (addr, prefix, token). Lets the
    /// host hand back the existing mount on an idempotent re-register and
    /// list every tenant without rebuilding one.
    handle: ServeHandle,
    artifacts: AppArtifacts,
}

impl HostedWorkspaceRuntime {
    fn router(&self) -> Router {
        self.artifacts.app.clone()
    }

    /// Signal shutdown and tear the workspace cell down. Returns a `Weak`
    /// to the workspace so a caller that needs the per-workspace flock
    /// released before it returns (an in-process close then reopen) can
    /// wait for the last strong `Arc` to drop. `None` when the cell was
    /// already cleared (a second call, e.g. Drop after an explicit close).
    fn shutdown(&self) -> Option<Weak<Workspace>> {
        let _ = self.artifacts.shutdown_tx.send(true);
        clear_workspace_cell(&self.artifacts.workspace_cell)
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
    pub fn new(library: Library) -> Self {
        Self::with_desktop_bridge(library, DesktopBridge::default())
    }

    /// Create a host whose tenants share `desktop` — chan-desktop passes a
    /// bridge carrying the window-ops channel and the title map so
    /// `cs window <op>` reaches the Tauri app and `cs window list` shows
    /// real titles.
    pub fn with_desktop_bridge(library: Library, desktop: DesktopBridge) -> Self {
        Self {
            library,
            workspaces: RwLock::new(HashMap::new()),
            desktop,
            register_lock: tokio::sync::Mutex::new(()),
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

        let artifacts = build_app(
            self.library.clone(),
            workspace,
            &config,
            self.desktop.clone(),
        )
        .await?;
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
        self.open_terminal_session_with_command(config, None).await
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

        let artifacts =
            build_terminal_app(self.library.clone(), &config, self.desktop.clone()).await?;
        // The tenant's terminals run `command` (when set) rather than the
        // default shell; applied before the SPA can open the first one.
        artifacts.terminal_sessions.set_default_command(command);
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

    /// Close the workspace mounted at `prefix`.
    ///
    /// Returns `Ok(false)` when no workspace is mounted there. Closing
    /// sends the shared shutdown signal before dropping the runtime,
    /// so active WebSockets and terminal sessions get a clean exit
    /// path.
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
        if let Some(weak) = released {
            wait_for_workspace_release(&weak);
        }
        Ok(true)
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
        let cell = runtime.artifacts.workspace_cell.read().ok()?;
        Some(cell.as_ref()?.workspace.clone())
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

/// Clear the workspace cell, signalling teardown and dropping the host's
/// strong `Arc<Workspace>`. Returns a `Weak` to that workspace (or `None`
/// when the cell was already empty) so the caller can wait for the last
/// strong reference to drop and the flock to release.
fn clear_workspace_cell(
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
) -> Option<Weak<Workspace>> {
    let cell = workspace_cell.write().ok()?.take()?;
    let WorkspaceCell {
        workspace,
        watch_handle,
        indexer,
    } = cell;
    // Clear the shared cell before socket accept loops finish aborting;
    // otherwise their stale Arc can keep the workspace marked open.
    indexer.cancel();
    drop(watch_handle);
    drop(indexer);
    let weak = Arc::downgrade(&workspace);
    drop(workspace);
    Some(weak)
}

/// Block (bounded) until the last strong `Arc<Workspace>` drops after
/// teardown, which releases the per-workspace flock. The straggler is an
/// in-flight reindex on the blocking pool: `clear_workspace_cell` set the
/// indexer's cancel flag, and the reindex drops its `Arc` at its next
/// per-file cancel check, on a separate blocking-pool thread that makes
/// progress regardless of this wait. Close is an infrequent teardown and
/// the wait is typically a few milliseconds. Bounded so a wedged reindex
/// cannot hang close: past the deadline the caller sees the same
/// lingering-flock behavior it would have had without the wait.
fn wait_for_workspace_release(weak: &Weak<Workspace>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while weak.strong_count() > 0 {
        if Instant::now() >= deadline {
            tracing::warn!("close_workspace: workspace handle still held 5s after teardown");
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

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
        let host = Arc::new(WorkspaceHost::new(lib.clone()));

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
        let host = Arc::new(WorkspaceHost::new(lib.clone()));
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
        let host = Arc::new(WorkspaceHost::new(lib.clone()));

        host.open_registered_workspace(root.path(), serve_config("/first"))
            .await
            .expect("open first");
        assert!(host.close_workspace("/first").expect("close first"));

        host.open_registered_workspace(root.path(), serve_config("/second"))
            .await
            .expect("reopen after close");
    }

    #[tokio::test]
    async fn live_workspace_returns_the_mounted_runtime_handle() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace");
        let lib = Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path()).expect("register");
        let host = Arc::new(WorkspaceHost::new(lib.clone()));
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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

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
        let host = Arc::new(WorkspaceHost::new(lib));

        // A command-carrying terminal tenant mounts like a default-shell
        // one; the command becomes the tenant's PTY default (the running of
        // it is covered by the registry tests).
        host.open_terminal_session_with_command(
            serve_config("/terminal-cmd"),
            Some("printf hi".into()),
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
}
