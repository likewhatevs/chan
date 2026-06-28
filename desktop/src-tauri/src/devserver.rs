//! Devserver management-API client.
//!
//! A devserver is a headless `chan devserver` aggregating many workspaces
//! on one box, reached over an `ssh -L` tunnel or direct loopback. The
//! desktop drives a small HTTP/JSON surface the devserver reserves at its
//! root prefix:
//!
//! - `GET  /api/devserver/info` (unauthenticated): health, version, label.
//! - `GET  /api/devserver/workspaces` (bearer): the workspaces to group.
//!
//! Every workspace is its own tokened tenant. The devserver
//! returns each tenant's `prefix` and per-tenant `token`; the desktop
//! assembles the tenant URL itself, `http://{host}:{port}{prefix}/index.html?t={token}`,
//! and opens it with the same outbound-window machinery as any remote URL.
//! Assembling client-side keeps the desktop in control of the local tunnel
//! port and avoids the devserver needing to know how it is reached.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Deserialize;

/// Per-request cap so an unreachable devserver cannot hang the launcher's
/// workspace poll, matching `probe_url`'s connect timeout.
const HTTP_TIMEOUT_SECS: u64 = 5;

/// Live connection to one devserver, keyed by the desktop-local
/// `Devserver.id` in [`DevserverConns`]. Connection state is held in memory
/// only: the bearer token rotates with the devserver, so a persisted copy
/// would decay between launches (the same reason local serve URLs live in
/// memory rather than `config.json`).
#[derive(Debug, Clone)]
pub struct DevserverConn {
    /// Tunnel endpoint host the desktop dials, e.g. `127.0.0.1` for an
    /// `ssh -L` forward.
    pub host: String,
    /// Tunnel endpoint port the desktop dials.
    pub port: u16,
    /// Devserver-level bearer token, distinct from the per-tenant tokens.
    /// Sent as `Authorization: Bearer <token>` on every endpoint except the
    /// unauthenticated info probe.
    pub token: String,
    /// Human display name for window titles (the server's `host_label`, else the
    /// dialed host). Resolved once at connect and carried on the conn so a
    /// reconnect (which clones the conn) reuses it without re-probing.
    pub name: String,
}

/// In-memory map of connected devservers keyed by `Devserver.id`. A
/// devserver absent from the map is disconnected: its `[DEVSERVER]` section
/// shows the disconnected placeholder rather than live workspace rows.
#[derive(Default)]
pub struct DevserverConns {
    inner: Mutex<HashMap<String, DevserverConn>>,
}

impl DevserverConns {
    pub fn get(&self, id: &str) -> Option<DevserverConn> {
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn set(&self, id: String, conn: DevserverConn) {
        self.inner.lock().unwrap().insert(id, conn);
    }

    pub fn remove(&self, id: &str) -> Option<DevserverConn> {
        self.inner.lock().unwrap().remove(id)
    }

    pub fn is_connected(&self, id: &str) -> bool {
        self.inner.lock().unwrap().contains_key(id)
    }
}

/// The management-API protocol version this desktop speaks. A devserver
/// reporting a different `protocol` is refused at connect rather than
/// driven against shapes that may have shifted.
pub const DEVSERVER_API_PROTOCOL: u32 = 1;

/// `GET /api/devserver/info`: the unauthenticated health probe.
#[derive(Debug, Clone, Deserialize)]
pub struct DevserverInfo {
    pub devserver_version: String,
    pub protocol: u32,
    /// Human label for the box, shown in the `[DEVSERVER {host}]` header
    /// once connected.
    pub host_label: String,
    /// The devserver library's `library_id`: supplied at
    /// connect so the desktop can mint the control terminal as a registry row
    /// under it even on a zero-window connect (no window record to learn it from).
    #[serde(default)]
    pub library_id: String,
    /// The devserver host's OS family (`macos | windows | linux | other`),
    /// surfaced to the launcher as the machine icon. `#[serde(default)]`: empty
    /// from a devserver too old to report it.
    #[serde(default)]
    pub os: String,
    /// Best-effort human OS string for the launcher tooltip; absent when unknown.
    #[serde(default)]
    pub pretty_name: Option<String>,
}

/// One element of `GET /api/devserver/workspaces`: a tenant the desktop
/// turns into a launcher row plus an assembled tenant URL.
#[derive(Debug, Clone, Deserialize)]
struct WorkspaceEntry {
    prefix: String,
    path: String,
    label: String,
    on: bool,
    token: String,
}

/// `POST /api/devserver/workspaces/{prefix}/on` body — mirrors the server's
/// `SetWorkspaceOnRequest`. `on:false` keeps the workspace registered
/// (unmount-but-remember), distinct from `DELETE` = Forget. `force` overrides
/// the server's off-with-live-terminals guard (the 409 below).
#[derive(Debug, serde::Serialize)]
struct SetWorkspaceOnRequest {
    on: bool,
    force: bool,
}

/// The server's 409 body when an unforced off is rejected because the tenant
/// still has live terminals — mirrors `ActiveTerminalsRejection`.
#[derive(Debug, serde::Deserialize)]
struct ActiveTerminalsRejection {
    active_terminals: usize,
}

/// Why a devserver workspace on/off failed, structured so the SPA can tell a
/// confirm-before-off (live terminals → offer to force) apart from a plain
/// failure. Serialized to the frontend as the command's error.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind")]
pub enum SetWorkspaceOnError {
    /// An unforced off was rejected: `active_terminals` live terminals would be
    /// killed. The SPA confirms, then retries with `force: true`.
    ActiveTerminals { active_terminals: usize },
    /// Any other failure (network, decode, non-409 status), as a plain message.
    Other { message: String },
}

impl SetWorkspaceOnError {
    pub fn other(msg: impl std::fmt::Display) -> Self {
        Self::Other {
            message: msg.to_string(),
        }
    }
}

/// A devserver workspace as the launcher renders it: the tenant fields plus
/// the assembled tenant URL ready for the outbound-window machinery.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DevserverWorkspaceRow {
    pub prefix: String,
    pub path: String,
    pub label: String,
    pub on: bool,
    pub url: String,
}

/// One process-wide `reqwest::Client`, reused across every devserver request.
///
/// Building a fresh `Client` per call (the old body of this fn) defeated
/// reqwest's keep-alive connection pool: the 5s workspace/colour poll
/// (`main.rs`) opens two requests per cycle, and a per-call Client never reuses
/// a connection, so the devserver held each one ESTABLISHED waiting for a reuse
/// that never came — ~22 leaked conns/min until it hit its 1024-fd cap (~40 min)
/// and started failing every accept with "Too many open files"
/// (`dev/devserver-bug/analysis.md`). A single cached Client shares one
/// connection pool across all callers (the pool keys on host:port, so each
/// devserver endpoint still gets its own pooled connection), collapsing
/// ESTABLISHED from ~967 to ~1. `Client` is internally `Arc`-backed, so the
/// per-call `.clone()` is cheap and every clone shares that one pool.
///
/// The signature is unchanged so the ~9 call sites stay as they are. The build
/// result (success OR the rare TLS-backend init failure) is memoized: a failure
/// here is deterministic, so caching it avoids re-attempting a build that cannot
/// succeed.
fn http_client() -> Result<reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .map_err(|e| format!("building devserver http client: {e}"))
        })
        .clone()
}

/// The management-API origin the desktop dials. Raw HTTP over the tunnel today
/// (the common `ssh -L` loopback case). FOLLOW-UP: a proxied-HTTPS dial (with
/// OAuth) will branch on the stored URL's scheme — [`parse_devserver_url`] keeps
/// the host/port; the scheme-aware branch is deferred (OAuth not built yet).
fn base_origin(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

/// Parse a stored devserver URL into the `(host, port)` the raw-tunnel dial
/// uses. The port defaults from the scheme when the URL omits it (`https`→443,
/// `http`→80), so `https://x.devserver.chan.app` resolves without an explicit
/// port. Bare `host:port` (no scheme) is rejected — the launcher requires a
/// `scheme://host` URL. The scheme is preserved in the stored URL for the
/// deferred proxied-HTTPS+OAuth dial branch (see [`base_origin`]); this only
/// extracts what the current raw-tunnel dial needs.
pub fn parse_devserver_url(url: &str) -> Result<(String, u16), String> {
    let parsed =
        url::Url::parse(url.trim()).map_err(|e| format!("invalid devserver URL {url:?}: {e}"))?;
    let host = parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or_else(|| format!("devserver URL {url:?} has no host"))?
        .to_string();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| format!("devserver URL {url:?} has no port and an unknown scheme"))?;
    Ok((host, port))
}

/// Assemble the tenant URL the desktop opens for a devserver tenant:
/// `http://{host}:{port}{prefix}/index.html?t={token}`. `prefix` is an
/// absolute route path such as `/api/notes-1a2b3c`. Routing through
/// `url::Url` percent-encodes the token query value.
pub fn assemble_tenant_url(
    host: &str,
    port: u16,
    prefix: &str,
    token: &str,
) -> Result<String, String> {
    let base = base_origin(host, port);
    let mut url = url::Url::parse(&base).map_err(|e| format!("bad devserver base {base}: {e}"))?;
    let path = format!("{}/index.html", prefix.trim_end_matches('/'));
    url.set_path(&path);
    url.query_pairs_mut().append_pair("t", token);
    Ok(url.to_string())
}

/// Path the devserver persists its config (including the bearer token) at on
/// the local box, the sibling of the desktop's own `desktop/config.json`
/// under the shared `~/.chan` home.
fn local_devserver_config_path() -> std::path::PathBuf {
    chan_workspace::paths::config_dir()
        .join("devserver")
        .join("config.json")
}

/// The local devserver's persisted config, of which the desktop only needs
/// the bearer token. A devserver on the same box writes this `0600`, so on a
/// local-loopback connection the desktop reads the token straight from the
/// file rather than scraping it from terminal output.
#[derive(Debug, Deserialize)]
struct LocalDevserverConfig {
    devserver_token: String,
    /// The devserver's last bound port, so a local connect dials the CURRENT
    /// port instead of a stored URL that goes stale when a `--port 0` devserver
    /// restarts on a different OS-assigned port. Absent (`0`) on an older config.
    #[serde(default)]
    port: u16,
}

/// Read the bearer token of a devserver running on this same box from its
/// persisted config. Fails when no local devserver has started (the file is
/// absent) or the file lacks the token.
pub fn read_local_token() -> Result<String, String> {
    let path = local_devserver_config_path();
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "reading the local devserver config at {}: {e}",
            path.display()
        )
    })?;
    let cfg: LocalDevserverConfig = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parsing the local devserver config: {e}"))?;
    if cfg.devserver_token.is_empty() {
        return Err("the local devserver config has no token yet".to_string());
    }
    Ok(cfg.devserver_token)
}

/// Read the last bound port of a devserver running on this same box from its
/// persisted config, or `None` when the file is absent/unreadable or carries no
/// bound port (`0`, an older config). A local connect dials this so it reaches
/// the current port after the devserver restarts on a new OS-assigned port,
/// instead of the stored URL's stale port.
pub fn read_local_port() -> Option<u16> {
    let bytes = std::fs::read(local_devserver_config_path()).ok()?;
    let cfg: LocalDevserverConfig = serde_json::from_slice(&bytes).ok()?;
    (cfg.port != 0).then_some(cfg.port)
}

/// Scrape the devserver bearer token from a control terminal's output, matching
/// the locked machine marker `CHAN_DEVSERVER_TOKEN=<token>` (the shared
/// `chan_server::DEVSERVER_TOKEN_MARKER`) that `chan devserver` emits on every
/// start AND `--systemd` re-attach. Single-sourcing the marker const keeps the
/// emitter and this scraper from drifting. The desktop scrapes this fresh on
/// every connect (and on a script re-run), so a recycled or rotated devserver is
/// handled by construction -- no stored/stale token to reuse.
///
/// `output` is raw PTY bytes (decoded lossily), so it carries ANSI escapes and
/// possibly several markers across restarts. Take the LAST one and read the
/// url-safe token run after it, which stops at the first non-token byte
/// (whitespace, an ANSI escape, end of line).
pub fn scrape_token(output: &str) -> Option<String> {
    let marker = chan_server::DEVSERVER_TOKEN_MARKER;
    output.rmatch_indices(marker).find_map(|(i, m)| {
        let token: String = output[i + m.len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        (!token.is_empty()).then_some(token)
    })
}

/// `GET /api/devserver/info`: unauthenticated, used to confirm the devserver
/// is up and read its version and label.
pub async fn fetch_info(host: &str, port: u16) -> Result<DevserverInfo, String> {
    let url = format!("{}/api/devserver/info", base_origin(host, port));
    let resp = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("reaching devserver {host}:{port}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("devserver info returned HTTP {}", resp.status()));
    }
    resp.json::<DevserverInfo>()
        .await
        .map_err(|e| format!("decoding devserver info: {e}"))
}

/// `GET /api/devserver/workspaces`: the live workspace list, each entry's
/// tenant URL already assembled.
pub async fn fetch_workspaces(conn: &DevserverConn) -> Result<Vec<DevserverWorkspaceRow>, String> {
    let url = format!(
        "{}/api/devserver/workspaces",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .get(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("listing devserver workspaces: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver workspaces returned HTTP {}",
            resp.status()
        ));
    }
    let entries = resp
        .json::<Vec<WorkspaceEntry>>()
        .await
        .map_err(|e| format!("decoding devserver workspaces: {e}"))?;
    entries
        .into_iter()
        .map(|e| row_from_entry(conn, e))
        .collect()
}

/// One `{ color }` frame of the devserver's `/api/library/local-color` GET.
#[derive(serde::Deserialize)]
struct LocalColorResponse {
    color: Option<String>,
}

/// `GET /api/library/local-color`: the devserver library's pane-highlight colour
/// (`#rrggbb`), or `None` for the default accent. Fetched ONCE on connect to warm
/// the desktop's per-devserver colour cache BEFORE the window watcher opens any
/// window, so a devserver window seeds its `?pane=` colour from the first build
/// instead of flashing blue until the async colour watch pushes. The
/// colour watch (`stream_color_feed`) keeps it live thereafter.
pub async fn fetch_local_color(conn: &DevserverConn) -> Result<Option<String>, String> {
    let url = format!(
        "{}/api/library/local-color",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .get(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("fetching devserver colour: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("devserver colour returned HTTP {}", resp.status()));
    }
    resp.json::<LocalColorResponse>()
        .await
        .map(|r| r.color)
        .map_err(|e| format!("decoding devserver colour: {e}"))
}

/// Turn a wire `WorkspaceEntry` into a launcher row, assembling the tenant URL
/// from its token. An off (registered-but-unmounted) row carries `token:""` and
/// gets an empty URL — it has no live tenant; the launcher renders it off and
/// Open turns it on first (which mints a fresh token).
fn row_from_entry(
    conn: &DevserverConn,
    e: WorkspaceEntry,
) -> Result<DevserverWorkspaceRow, String> {
    let url = if e.token.is_empty() {
        String::new()
    } else {
        assemble_tenant_url(&conn.host, conn.port, &e.prefix, &e.token)?
    };
    Ok(DevserverWorkspaceRow {
        prefix: e.prefix,
        path: e.path,
        label: e.label,
        on: e.on,
        url,
    })
}

/// One row of `GET /api/devserver/windows`: a
/// PERSISTED workspace window the desktop enumerates to offer CLOSED-but-
/// persisted windows for reopen in the Window menu. Deserialized 1:1 from the
/// frozen wire; `title` is optional (mirrors `WindowInfo`). `prefix` + the
/// CURRENT (re-minted) per-mount `token` assemble the reopen URL; `token` is
/// empty when the tenant is off (not menu-reopenable — use the launcher row).
#[derive(Debug, Clone, Deserialize)]
pub struct DevserverWindowRow {
    pub label: String,
    pub prefix: String,
    pub token: String,
    #[serde(default)]
    pub title: Option<String>,
    pub connected: bool,
    pub saved: bool,
}

/// `GET /api/devserver/windows`: every PERSISTED window across all
/// of the devserver's tenants, with the live `connected`/`saved` flags + the
/// current per-mount token. Authed like the rest. Persisted-only by construction
/// (a discarded window's blob is already gone server-side), so the desktop only
/// filters `saved && !connected` for the reopenable set.
pub async fn fetch_devserver_windows(
    conn: &DevserverConn,
) -> Result<Vec<DevserverWindowRow>, String> {
    let url = format!(
        "{}/api/devserver/windows",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .get(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("listing devserver windows: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("devserver windows returned HTTP {}", resp.status()));
    }
    resp.json::<Vec<DevserverWindowRow>>()
        .await
        .map_err(|e| format!("decoding devserver windows: {e}"))
}

/// The full window set a connected devserver serves at
/// `GET /api/library/windows` — the watcher's initial seed (it also carries the
/// devserver's `library_id`, stamped per row, the watcher's first read of which
/// library it is reconciling). The WS `/watch` then pushes every change. The new
/// library feed that supersedes the per-tenant `fetch_devserver_windows`.
pub async fn fetch_library_windows(
    conn: &DevserverConn,
) -> Result<Vec<chan_server::WindowRecord>, String> {
    let url = format!("{}/api/library/windows", base_origin(&conn.host, conn.port));
    let resp = http_client()?
        .get(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("listing library windows: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("library windows returned HTTP {}", resp.status()));
    }
    resp.json::<Vec<chan_server::WindowRecord>>()
        .await
        .map_err(|e| format!("decoding library windows: {e}"))
}

/// Mint a window on a connected devserver's library
/// (`POST /api/library/windows`): the library assigns the id, persists the
/// record, and fires the watch, so the desktop's watcher reconciles the new
/// window open — no client-side open. Used for the first-connect boot terminal
/// (`kind: Terminal`) and launcher-open reroutes.
pub async fn mint_library_window(
    conn: &DevserverConn,
    kind: chan_server::WindowKind,
    workspace_path: Option<String>,
) -> Result<chan_server::WindowRecord, String> {
    let url = format!("{}/api/library/windows", base_origin(&conn.host, conn.port));
    let body = chan_server::CreateWindow {
        kind,
        workspace_path,
    };
    let resp = http_client()?
        .post(&url)
        .bearer_auth(&conn.token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("minting library window: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "library window mint returned HTTP {}",
            resp.status()
        ));
    }
    resp.json::<chan_server::WindowRecord>()
        .await
        .map_err(|e| format!("decoding minted window: {e}"))
}

/// `DELETE /api/library/windows/{window_id}`: discard a devserver window's
/// registry record. The server drops the row, PERSISTS the removal
/// (`save_best_effort`), and fires the watch so every client's reconcile closes
/// the window. The devserver analog of the local `embedded.discard_window` — a
/// closed devserver window must DELETE its record, else it survives server-side
/// and reopens (empty) on restart. A 404 (already gone) is success.
pub async fn discard_library_window(conn: &DevserverConn, window_id: &str) -> Result<(), String> {
    let url = format!(
        "{}/api/library/windows/{}",
        base_origin(&conn.host, conn.port),
        window_id
    );
    let resp = http_client()?
        .delete(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("discarding library window: {e}"))?;
    if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(format!(
            "library window discard returned HTTP {}",
            resp.status()
        ));
    }
    Ok(())
}

/// The `DELETE` URL for unmounting a workspace tenant. The server route is
/// an axum wildcard, so `prefix` (an absolute route path like
/// `/api/notes-1a2b3c`) is appended verbatim after the collection path.
fn workspace_delete_url(host: &str, port: u16, prefix: &str) -> String {
    format!(
        "{}/api/devserver/workspaces{}",
        base_origin(host, port),
        prefix
    )
}

/// `DELETE /api/devserver/workspaces/{prefix}`: unmount a workspace tenant
/// from the devserver (the "Forget" action).
pub async fn forget_workspace(conn: &DevserverConn, prefix: &str) -> Result<(), String> {
    let url = workspace_delete_url(&conn.host, conn.port, prefix);
    let resp = http_client()?
        .delete(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("forgetting devserver workspace: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver workspace delete returned HTTP {}",
            resp.status()
        ));
    }
    Ok(())
}

/// `POST /api/library/windows/{window_id}/visibility` `{hidden}`: set a devserver
/// window's server-persisted visibility. The devserver owns its window
/// registry, so hiding/showing a remote window persists THERE and the desktop
/// mirrors it on the next connect. Distinct from the `/hide`+`/open` bridge ops
/// (transient, non-persistent). Fire-and-forget from the bury/unbury chokepoint.
pub async fn set_window_visibility(
    conn: &DevserverConn,
    window_id: &str,
    hidden: bool,
) -> Result<(), String> {
    let url = format!(
        "{}/api/library/windows/{}/visibility",
        base_origin(&conn.host, conn.port),
        window_id
    );
    let resp = http_client()?
        .post(&url)
        .bearer_auth(&conn.token)
        .json(&serde_json::json!({ "hidden": hidden }))
        .send()
        .await
        .map_err(|e| format!("setting devserver window visibility: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver window visibility returned HTTP {}",
            resp.status()
        ));
    }
    Ok(())
}

/// The on/off-toggle URL for a registered workspace: the collection path + the
/// prefix (an absolute route path) + `/on`. Distinct from the DELETE URL
/// (= Forget); on/off keeps the registration.
fn workspace_on_url(host: &str, port: u16, prefix: &str) -> String {
    format!(
        "{}/api/devserver/workspaces{}/on",
        base_origin(host, port),
        prefix
    )
}

/// `POST /api/devserver/workspaces/{prefix}/on` `{on, force}`: mount (`on:true`)
/// or unmount (`on:false`) a registered workspace WITHOUT forgetting it. Turning
/// on mints a fresh tenant token; turning off clears it. Idempotent server-side.
/// An unforced off is rejected with 409 + a live-terminal count when the tenant
/// has open terminals — surfaced as [`SetWorkspaceOnError::ActiveTerminals`] so
/// the SPA can confirm-then-force; `force: true` overrides the guard.
pub async fn set_workspace_on(
    conn: &DevserverConn,
    prefix: &str,
    on: bool,
    force: bool,
) -> Result<(), SetWorkspaceOnError> {
    let url = workspace_on_url(&conn.host, conn.port, prefix);
    let resp = http_client()
        .map_err(SetWorkspaceOnError::other)?
        .post(&url)
        .bearer_auth(&conn.token)
        .json(&SetWorkspaceOnRequest { on, force })
        .send()
        .await
        .map_err(|e| {
            SetWorkspaceOnError::other(format!("setting devserver workspace on/off: {e}"))
        })?;
    if resp.status() == reqwest::StatusCode::CONFLICT {
        // Off blocked by live terminals: surface the count for the confirm.
        let active_terminals = resp
            .json::<ActiveTerminalsRejection>()
            .await
            .map(|r| r.active_terminals)
            .unwrap_or(0);
        return Err(SetWorkspaceOnError::ActiveTerminals { active_terminals });
    }
    if !resp.status().is_success() {
        return Err(SetWorkspaceOnError::other(format!(
            "devserver workspace on/off returned HTTP {}",
            resp.status()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_devserver_url_reads_host_and_explicit_port() {
        assert_eq!(
            parse_devserver_url("http://127.0.0.1:8787").unwrap(),
            ("127.0.0.1".to_string(), 8787)
        );
    }

    #[test]
    fn parse_devserver_url_defaults_port_from_scheme() {
        assert_eq!(
            parse_devserver_url("https://box.example.com").unwrap(),
            ("box.example.com".to_string(), 443)
        );
        assert_eq!(
            parse_devserver_url("http://box.example.com").unwrap(),
            ("box.example.com".to_string(), 80)
        );
    }

    #[test]
    fn parse_devserver_url_rejects_bare_host_port_and_garbage() {
        // Bare host:port has no scheme — the launcher requires scheme://host.
        assert!(parse_devserver_url("127.0.0.1:8787").is_err());
        assert!(parse_devserver_url("not a url").is_err());
        assert!(parse_devserver_url("").is_err());
    }

    #[test]
    fn assemble_tenant_url_uses_host_port_prefix_token() {
        let url = assemble_tenant_url("127.0.0.1", 8787, "/api/notes-1a2b3c", "tok_abc").unwrap();
        assert_eq!(
            url,
            "http://127.0.0.1:8787/api/notes-1a2b3c/index.html?t=tok_abc"
        );
    }

    #[test]
    fn assemble_tenant_url_trims_a_trailing_slash_on_the_prefix() {
        let url = assemble_tenant_url("10.0.0.5", 9000, "/api/a-0000/", "t").unwrap();
        assert_eq!(url, "http://10.0.0.5:9000/api/a-0000/index.html?t=t");
    }

    #[test]
    fn assemble_tenant_url_percent_encodes_the_token() {
        let url = assemble_tenant_url("127.0.0.1", 8787, "/api/x-1", "a b&c").unwrap();
        assert!(url.ends_with("/api/x-1/index.html?t=a+b%26c"), "{url}");
    }

    #[test]
    fn devserver_info_decodes_the_wire_shape() {
        let json = r#"{"devserver_version":"0.38.0","protocol":1,"host_label":"lab box"}"#;
        let info: DevserverInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.devserver_version, "0.38.0");
        assert_eq!(info.protocol, 1);
        assert_eq!(info.host_label, "lab box");
    }

    #[test]
    fn workspace_entry_decodes_a_bare_array_element() {
        let json = r#"[{"prefix":"/api/notes-1a2b3c","path":"/home/a/notes","label":"notes","on":true,"token":"tok_abc"}]"#;
        let entries: Vec<WorkspaceEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prefix, "/api/notes-1a2b3c");
        assert_eq!(entries[0].path, "/home/a/notes");
        assert_eq!(entries[0].label, "notes");
        assert!(entries[0].on);
        assert_eq!(entries[0].token, "tok_abc");
    }

    #[test]
    fn devserver_window_row_decodes_reopenable_window_shape() {
        // Pins the GET /api/devserver/windows wire: title is
        // optional; connected/saved drive the reopenable filter; token is empty
        // when the tenant is off. An extra wire field (e.g. a legacy `kind`) is
        // ignored. A drift here reds before the menu misbehaves.
        let json = r#"[
          {"label":"workspace-abc-1","prefix":"/api/notes-abc","token":"tok1","kind":"workspace","title":"🏠 /n Window 1","connected":false,"saved":true},
          {"label":"workspace-def-2","prefix":"/api/notes-def","token":"","connected":true,"saved":true}
        ]"#;
        let rows: Vec<DevserverWindowRow> = serde_json::from_str(json).unwrap();
        assert_eq!(rows.len(), 2);
        // Row 0: reopenable (saved && !connected), title present; the unknown
        // `kind` field is ignored.
        assert_eq!(rows[0].label, "workspace-abc-1");
        assert_eq!(rows[0].prefix, "/api/notes-abc");
        assert_eq!(rows[0].token, "tok1");
        assert_eq!(rows[0].title.as_deref(), Some("🏠 /n Window 1"));
        assert!(rows[0].saved && !rows[0].connected);
        // Row 1: optional title absent (defaults None); empty token = off.
        assert_eq!(rows[1].title, None);
        assert!(rows[1].token.is_empty());
        assert!(rows[1].connected); // NOT reopenable (a client is attached)
    }

    #[test]
    fn workspace_delete_url_appends_the_prefix_verbatim() {
        assert_eq!(
            workspace_delete_url("127.0.0.1", 8787, "/api/notes-1a2b3c"),
            "http://127.0.0.1:8787/api/devserver/workspaces/api/notes-1a2b3c"
        );
    }

    #[test]
    fn workspace_on_url_appends_prefix_and_on() {
        assert_eq!(
            workspace_on_url("127.0.0.1", 8787, "/api/notes-1a2b3c"),
            "http://127.0.0.1:8787/api/devserver/workspaces/api/notes-1a2b3c/on"
        );
    }

    #[test]
    fn set_workspace_on_request_serializes_on_and_force_fields() {
        assert_eq!(
            serde_json::to_string(&SetWorkspaceOnRequest {
                on: false,
                force: false
            })
            .unwrap(),
            r#"{"on":false,"force":false}"#
        );
        assert_eq!(
            serde_json::to_string(&SetWorkspaceOnRequest {
                on: true,
                force: true
            })
            .unwrap(),
            r#"{"on":true,"force":true}"#
        );
    }

    #[test]
    fn row_from_entry_off_row_has_no_url_on_row_has_one() {
        let conn = DevserverConn {
            host: "127.0.0.1".into(),
            port: 8787,
            token: "dt".into(),
            name: "box".into(),
        };
        // Off (registered-but-unmounted): token:"" ⇒ empty URL.
        let off = WorkspaceEntry {
            prefix: "/api/notes-1a2b3c".into(),
            path: "/home/a/notes".into(),
            label: "notes".into(),
            on: false,
            token: String::new(),
        };
        let row = row_from_entry(&conn, off).unwrap();
        assert!(!row.on);
        assert_eq!(row.url, "");
        // On: a live token assembles the tenant URL.
        let on = WorkspaceEntry {
            prefix: "/api/notes-1a2b3c".into(),
            path: "/home/a/notes".into(),
            label: "notes".into(),
            on: true,
            token: "tok_live".into(),
        };
        let row = row_from_entry(&conn, on).unwrap();
        assert!(row.on);
        assert_eq!(
            row.url,
            "http://127.0.0.1:8787/api/notes-1a2b3c/index.html?t=tok_live"
        );
    }

    #[test]
    fn scrape_token_reads_the_marker_line() {
        // The locked machine marker, e.g. surfaced through a journalctl follow.
        let out = "some boot noise\nJun 17 host chan[12]: CHAN_DEVSERVER_TOKEN=tok_abc123\n$ ";
        assert_eq!(scrape_token(out).as_deref(), Some("tok_abc123"));
    }

    #[test]
    fn scrape_token_takes_the_last_occurrence_across_restarts() {
        let out = "CHAN_DEVSERVER_TOKEN=old_TOKEN\n[restart]\nCHAN_DEVSERVER_TOKEN=new-TOKEN_2\n";
        assert_eq!(scrape_token(out).as_deref(), Some("new-TOKEN_2"));
    }

    #[test]
    fn scrape_token_stops_at_ansi_or_whitespace() {
        // Raw PTY bytes carry ANSI; the token run stops at the escape byte.
        let out = "CHAN_DEVSERVER_TOKEN=tok_xyz\x1b[0m extra";
        assert_eq!(scrape_token(out).as_deref(), Some("tok_xyz"));
    }

    #[test]
    fn scrape_token_none_when_absent_or_empty() {
        assert_eq!(scrape_token("no token here\n$ "), None);
        assert_eq!(scrape_token("CHAN_DEVSERVER_TOKEN= \nnext"), None);
        // A loose `token=` (human-readable line) is NOT the machine marker.
        assert_eq!(
            scrape_token("chan devserver: bind=… token=tok_loose\n"),
            None
        );
    }

    #[test]
    fn scrape_token_ignores_the_w5_running_banner() {
        // The terminal layer prepends a `running: {command}\r\n` banner to
        // the control terminal's scrollback before the connect script runs — it is
        // the FIRST ring bytes, ahead of any token the devserver emits. Confirm it
        // can't disturb the scrape.
        //
        // 1. A real connect-script command never contains the marker (the token is
        //    runtime-generated by `chan devserver`, not passed in), so the banner is
        //    inert and the real token is read.
        let out = "running: ssh box -L 8787:localhost:8787 chan devserver\r\n\
                   CHAN_DEVSERVER_TOKEN=tok_real123\r\n$ ";
        assert_eq!(scrape_token(out).as_deref(), Some("tok_real123"));
        // 2. Even pathologically — a command string that literally embeds the marker
        //    — the banner is the FIRST bytes and `scrape_token` takes the LAST marker
        //    (`rmatch_indices`), so the devserver's real token (emitted AFTER the
        //    script connects) still wins; the banner's marker is never reached.
        let pathological = "running: CHAN_DEVSERVER_TOKEN=from_command chan devserver\r\n\
                            CHAN_DEVSERVER_TOKEN=tok_real456\r\n$ ";
        assert_eq!(scrape_token(pathological).as_deref(), Some("tok_real456"));
    }

    #[test]
    fn local_devserver_config_reads_token_and_port() {
        // The desktop reads the token + the bound port; legacy/unknown keys are
        // ignored, and an absent port defaults to 0 (an older config).
        let json = r#"{"devserver_token":"tok_box","port":9605,"workspaces":[],"terminals":[]}"#;
        let cfg: LocalDevserverConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.devserver_token, "tok_box");
        assert_eq!(cfg.port, 9605);

        let no_port = r#"{"devserver_token":"tok_box"}"#;
        let cfg: LocalDevserverConfig = serde_json::from_str(no_port).unwrap();
        assert_eq!(cfg.port, 0);
    }

    #[test]
    fn conns_set_get_remove_roundtrip() {
        let conns = DevserverConns::default();
        assert!(!conns.is_connected("ds1"));
        conns.set(
            "ds1".into(),
            DevserverConn {
                host: "127.0.0.1".into(),
                port: 8787,
                token: "tok".into(),
                name: "box".into(),
            },
        );
        assert!(conns.is_connected("ds1"));
        assert_eq!(conns.get("ds1").unwrap().port, 8787);
        assert!(conns.remove("ds1").is_some());
        assert!(!conns.is_connected("ds1"));
    }
}
