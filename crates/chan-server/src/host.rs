//! Multi-workspace host runtime.
//!
//! `WorkspaceHost` is the in-process owner that chan-desktop can embed
//! instead of spawning one `chan serve` child per local workspace. Each
//! mounted workspace still gets its own `AppState`, watcher, indexer,
//! MCP bridge, control socket, terminal registry, and route prefix.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use chan_workspace::{Library, Workspace};
use tower::ServiceExt;

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
}

struct HostedWorkspaceRuntime {
    root: PathBuf,
    artifacts: AppArtifacts,
}

impl HostedWorkspaceRuntime {
    fn router(&self) -> Router {
        self.artifacts.app.clone()
    }

    fn shutdown(&self) {
        let _ = self.artifacts.shutdown_tx.send(true);
        clear_workspace_cell(&self.artifacts.workspace_cell);
    }
}

impl Drop for HostedWorkspaceRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl WorkspaceHost {
    /// Create an empty host backed by the caller's `Library`.
    pub fn new(library: Library) -> Self {
        Self {
            library,
            workspaces: RwLock::new(HashMap::new()),
        }
    }

    /// Return the shared workspace registry handle.
    pub fn library(&self) -> &Library {
        &self.library
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

        let artifacts = build_app(self.library.clone(), workspace, &config).await?;
        let handle = ServeHandle {
            addr: config.addr,
            prefix: prefix.clone(),
            token: artifacts.token.clone(),
        };
        let hosted = HostedWorkspace {
            root: root.clone(),
            prefix: prefix.clone(),
            handle,
        };
        let runtime = HostedWorkspaceRuntime { root, artifacts };

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

    /// Mount a workspace-less "terminal-only" tenant under
    /// `config.prefix`.
    ///
    /// Mirrors [`open_workspace`](Self::open_workspace) but backs the
    /// mount with [`build_terminal_app`] instead of `build_app`: no
    /// `Arc<Workspace>`, no watcher / indexer / MCP bridge / control
    /// socket. The slim tenant serves only the terminal + window-session
    /// routes plus the SPA shell, so a standalone terminal window
    /// (desktop webview in `?kind=terminal` mode) gets a PTY surface
    /// without a workspace behind it.
    ///
    /// The tenant lands in the SAME `workspaces` map as workspace mounts
    /// and is reached by the same `host_dispatch` prefix routing, so the
    /// duplicate-prefix guard and `close_workspace` apply uniformly. The
    /// returned [`HostedWorkspace::root`] is the PTY cwd (the user's home
    /// dir) since there is no workspace root; `handle.launch_url()`
    /// resolves against `config.addr`/`prefix`/token exactly like a
    /// workspace mount.
    pub async fn open_terminal_session(
        &self,
        mut config: ServeConfig,
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

        let artifacts = build_terminal_app(self.library.clone(), &config).await?;
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
            handle,
        };
        let runtime = HostedWorkspaceRuntime { root, artifacts };

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
        Ok(runtime.is_some())
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

fn clear_workspace_cell(workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>) {
    let cell = match workspace_cell.write() {
        Ok(mut cell) => cell.take(),
        Err(_) => return,
    };
    let Some(cell) = cell else {
        return;
    };
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
    drop(workspace);
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
