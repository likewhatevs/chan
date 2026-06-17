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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, Request as HttpRequest, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chan_workspace::Library;
use serde::{Deserialize, Serialize};

use crate::auth::random_token;
use crate::devserver_api::{
    DevserverInfo, MountedPrefix, MountedTerminal, OpenWorkspaceRequest, WorkspaceEntry,
    DEVSERVER_API_PROTOCOL,
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
/// reconnecting client keeps working across restarts; `enabled_workspaces`
/// is the set of roots that were mounted, re-mounted on the next start.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedConfig {
    #[serde(default)]
    devserver_token: String,
    #[serde(default)]
    enabled_workspaces: Vec<String>,
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

/// A mounted workspace as the devserver tracks it, the source of truth for
/// `GET /api/devserver/workspaces`. Keyed by prefix in [`DevserverState`].
struct WorkspaceRecord {
    root: PathBuf,
    prefix: String,
    label: String,
    token: String,
}

/// Shared runtime state behind the management API and the discovery socket.
struct DevserverState {
    host: Arc<WorkspaceHost>,
    addr: SocketAddr,
    /// Devserver-level bearer token, distinct from per-workspace tokens.
    token: String,
    host_label: String,
    /// Mounted workspaces by prefix. Terminal tenants are NOT tracked here;
    /// they are not listed and reset on restart.
    workspaces: Mutex<HashMap<String, WorkspaceRecord>>,
    store: DevserverStore,
    /// Monotonic source for standalone-terminal prefixes within a run.
    terminal_seq: AtomicU64,
}

impl DevserverState {
    /// Mount the workspace at `root` (registering it with the shared
    /// `Library` first) and record it, idempotent on the root. Returns the
    /// prefix it is mounted at. Used by `POST workspaces`, the discovery
    /// socket, and restart re-mounting.
    async fn register_workspace(&self, root: &Path) -> Result<String, Error> {
        let prefix = allocate_workspace_prefix(root)?;
        // The host opens through the shared `Library`, which requires the
        // root to be registered; registering an already-known root is a
        // no-op.
        self.host.library().register_workspace(root)?;
        let hosted = self
            .host
            .open_or_get_registered_workspace(root, tenant_config(self.addr, &prefix))
            .await?;
        let record = WorkspaceRecord {
            root: hosted.root.clone(),
            prefix: hosted.prefix.clone(),
            label: workspace_label(&hosted.root),
            // Tenants are configured with `no_token: false`, so the handle
            // always carries a token; default to empty rather than panic.
            token: hosted.handle.token.clone().unwrap_or_default(),
        };
        {
            let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.insert(hosted.prefix.clone(), record);
        }
        self.persist_enabled();
        Ok(hosted.prefix)
    }

    /// Unmount the tenant at `prefix` and drop it from the persisted set.
    /// Returns whether a tenant was actually mounted there.
    fn forget_workspace(&self, prefix: &str) -> Result<bool, Error> {
        let closed = self.host.close_workspace(prefix)?;
        {
            let mut workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            workspaces.remove(prefix);
        }
        self.persist_enabled();
        Ok(closed)
    }

    /// Persist the bearer token plus the currently-mounted workspace roots,
    /// so a restart comes back serving exactly what is mounted now.
    fn persist_enabled(&self) {
        let enabled_workspaces: Vec<String> = {
            let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
            let mut roots: Vec<String> = workspaces
                .values()
                .map(|record| record.root.to_string_lossy().into_owned())
                .collect();
            roots.sort();
            roots
        };
        let cfg = PersistedConfig {
            devserver_token: self.token.clone(),
            enabled_workspaces,
        };
        if let Err(e) = self.store.save(&cfg) {
            tracing::warn!("persisting devserver config: {e}");
        }
    }

    /// Snapshot the mounted workspaces for the list endpoint, sorted by
    /// prefix for a stable listing. Mounted means on, so `on` is always true
    /// for a listed entry in this round.
    fn workspace_entries(&self) -> Vec<WorkspaceEntry> {
        let workspaces = self.workspaces.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<WorkspaceEntry> = workspaces
            .values()
            .map(|record| WorkspaceEntry {
                prefix: record.prefix.clone(),
                path: record.root.to_string_lossy().into_owned(),
                label: record.label.clone(),
                on: true,
                token: record.token.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        entries
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
    let state = Arc::new(DevserverState {
        host: host.clone(),
        addr: config.addr,
        token: token.clone(),
        host_label: config.host_label,
        workspaces: Mutex::new(HashMap::new()),
        store,
        terminal_seq: AtomicU64::new(0),
    });

    // Re-mount what was on. A root that fails to re-mount surfaces a note
    // and is left off; persist below records the survivors.
    for root in &persisted.enabled_workspaces {
        let path = PathBuf::from(root);
        if let Err(e) = state.register_workspace(&path).await {
            eprintln!("chan devserver: NOTE: could not re-mount {root}: {e}");
        }
    }
    // Persist once now so a newly-minted token + the surviving enabled set
    // land even before the first management call.
    state.persist_enabled();

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
        .route("/api/devserver/workspaces/*prefix", delete(handle_forget))
        .route("/api/devserver/terminals", post(handle_open_terminal))
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

async fn handle_open_terminal(State(state): State<Arc<DevserverState>>) -> Response {
    let n = state.terminal_seq.fetch_add(1, Ordering::Relaxed);
    let prefix = format!("/api/terminal-{n}");
    match state
        .host
        .open_terminal_session(tenant_config(state.addr, &prefix))
        .await
    {
        Ok(hosted) => Json(MountedTerminal {
            prefix: hosted.prefix,
            token: hosted.handle.token.unwrap_or_default(),
        })
        .into_response(),
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
            enabled_workspaces: vec!["/a".into(), "/b".into()],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: PersistedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.devserver_token, "tok");
        assert_eq!(back.enabled_workspaces, vec!["/a".to_string(), "/b".into()]);
        // Tolerant of a missing/empty file shape.
        let empty: PersistedConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.devserver_token, "");
        assert!(empty.enabled_workspaces.is_empty());
    }

    #[test]
    fn store_save_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = DevserverStore::at(dir.path().join("nested").join("config.json"));
        // Missing file loads a default.
        assert_eq!(store.load().devserver_token, "");
        let cfg = PersistedConfig {
            devserver_token: "abc".into(),
            enabled_workspaces: vec!["/x".into()],
        };
        store.save(&cfg).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.devserver_token, "abc");
        assert_eq!(loaded.enabled_workspaces, vec!["/x".to_string()]);
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
}
