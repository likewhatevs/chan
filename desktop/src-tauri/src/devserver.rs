//! Devserver management-API client.
//!
//! A devserver is a headless `chan devserver` aggregating many workspaces
//! on one box, reached over an `ssh -L` tunnel or direct loopback. The
//! desktop drives a small HTTP/JSON surface the devserver reserves at its
//! root prefix:
//!
//! - `GET  /api/devserver/info` (unauthenticated): health, version, label.
//! - `GET  /api/devserver/workspaces` (bearer): the workspaces to group.
//! - `POST /api/devserver/terminals` (bearer): mount a standalone terminal.
//!
//! Every workspace and terminal is its own tokened tenant. The devserver
//! returns each tenant's `prefix` and per-tenant `token`; the desktop
//! assembles the tenant URL itself, `http://{host}:{port}{prefix}/index.html?t={token}`,
//! and opens it with the same outbound-window machinery as any remote URL.
//! Assembling client-side keeps the desktop in control of the local tunnel
//! port and avoids the devserver needing to know how it is reached.

use std::collections::HashMap;
use std::sync::Mutex;
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

/// `POST /api/devserver/terminals`: the mounted terminal tenant's prefix
/// and its per-tenant token, enough to assemble the tab URL.
#[derive(Debug, Clone, Deserialize)]
struct MountedTerminal {
    prefix: String,
    token: String,
}

/// `POST /api/devserver/workspaces/{prefix}/on` body — mirrors the server's
/// `SetWorkspaceOnRequest`. `on:false` keeps the workspace registered
/// (unmount-but-remember), distinct from `DELETE` = Forget.
#[derive(Debug, serde::Serialize)]
struct SetWorkspaceOnRequest {
    on: bool,
}

/// A devserver workspace as the launcher renders it: the tenant fields plus
/// the assembled tenant URL ready for the outbound-window machinery.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DevserverWorkspaceRow {
    pub prefix: String,
    pub path: String,
    pub label: String,
    pub on: bool,
    pub url: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("building devserver http client: {e}"))
}

fn base_origin(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
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

/// `POST /api/devserver/terminals` body carrying the desktop-assigned window
/// `label` (the `?w=` key, in the desktop's outbound family). Per Seam 4
/// Amendment 6 the devserver persists `{label, prefix, command}` keyed by the
/// label, so the same standalone terminal re-surfaces on reconnect.
#[derive(Debug, serde::Serialize)]
struct OpenTerminalRequest {
    label: String,
}

/// `POST /api/devserver/terminals {label}`: mount a PERSISTED standalone
/// terminal tenant under the desktop-assigned `label` and return its assembled
/// tab URL. The label is the stable identity (echoed by `fetch_terminals` on
/// reconnect); the per-mount prefix/token come back fresh.
pub async fn open_terminal_with_label(conn: &DevserverConn, label: &str) -> Result<String, String> {
    let url = format!(
        "{}/api/devserver/terminals",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .post(&url)
        .bearer_auth(&conn.token)
        .json(&OpenTerminalRequest {
            label: label.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("opening devserver terminal: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver terminals returned HTTP {}",
            resp.status()
        ));
    }
    let terminal = resp
        .json::<MountedTerminal>()
        .await
        .map_err(|e| format!("decoding devserver terminal: {e}"))?;
    assemble_tenant_url(&conn.host, conn.port, &terminal.prefix, &terminal.token)
}

/// One element of `GET /api/devserver/terminals`: a persisted standalone
/// terminal the desktop re-creates as a window on connect. `label` is the
/// stable `?w=` window key the desktop minted; `prefix`/`token` are the
/// current mount (the prefix need not be byte-stable across restarts).
#[derive(Debug, Clone, Deserialize)]
struct PersistedTerminalEntry {
    label: String,
    prefix: String,
    token: String,
}

/// A persisted devserver terminal as the desktop re-creates it: its stable
/// window label + the assembled tenant URL.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DevserverTerminalRow {
    pub label: String,
    pub url: String,
}

/// `GET /api/devserver/terminals`: the devserver's persisted standalone
/// terminals (Seam 4 Amendment 5), each with its assembled tenant URL, to
/// re-surface as windows on connect/reconnect.
// Imperative devserver-window path superseded by the window watcher (the
// reconcile re-surfaces terminals); deleted with the imperative layer in
// S2-DEVSERVER D3.
#[allow(dead_code)]
pub async fn fetch_terminals(conn: &DevserverConn) -> Result<Vec<DevserverTerminalRow>, String> {
    let url = format!(
        "{}/api/devserver/terminals",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .get(&url)
        .bearer_auth(&conn.token)
        .send()
        .await
        .map_err(|e| format!("listing devserver terminals: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver terminals returned HTTP {}",
            resp.status()
        ));
    }
    let entries = resp
        .json::<Vec<PersistedTerminalEntry>>()
        .await
        .map_err(|e| format!("decoding devserver terminals: {e}"))?;
    entries
        .into_iter()
        .map(|e| {
            let url = assemble_tenant_url(&conn.host, conn.port, &e.prefix, &e.token)?;
            Ok(DevserverTerminalRow {
                label: e.label,
                url,
            })
        })
        .collect()
}

/// One row of `GET /api/devserver/windows` (contracts.md Amendment 8): a
/// PERSISTED window across any of the devserver's tenants (workspace OR
/// standalone terminal). The desktop enumerates these to offer CLOSED-but-
/// persisted windows for reopen in the Window menu. Deserialized 1:1 from the
/// frozen wire; `kind`/`title` are optional (mirror `WindowInfo`). `prefix` +
/// the CURRENT (re-minted) per-mount `token` assemble the reopen URL; `token`
/// is empty when the tenant is off (not menu-reopenable — use the launcher row).
#[derive(Debug, Clone, Deserialize)]
pub struct DevserverWindowRow {
    pub label: String,
    pub prefix: String,
    pub token: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub connected: bool,
    pub saved: bool,
}

/// `GET /api/devserver/windows` (Amendment 8): every PERSISTED window across all
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

/// The full Seam-W window set a connected devserver serves at
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
/// (`kind: Terminal`). (The D3.1 mint helper, pulled forward for the
/// D1-completion boot-terminal bootstrap; the launcher-open reroute stays D3.)
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
        return Err(format!("library window mint returned HTTP {}", resp.status()));
    }
    resp.json::<chan_server::WindowRecord>()
        .await
        .map_err(|e| format!("decoding minted window: {e}"))
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

/// `POST /api/devserver/workspaces/{prefix}/on` `{on}`: mount (`on:true`) or
/// unmount (`on:false`) a registered workspace WITHOUT forgetting it. Returns
/// the updated row; turning on mints a fresh tenant token (so the reassembled
/// URL is live), turning off clears it (empty URL). Idempotent server-side.
pub async fn set_workspace_on(
    conn: &DevserverConn,
    prefix: &str,
    on: bool,
) -> Result<DevserverWorkspaceRow, String> {
    let url = workspace_on_url(&conn.host, conn.port, prefix);
    let resp = http_client()?
        .post(&url)
        .bearer_auth(&conn.token)
        .json(&SetWorkspaceOnRequest { on })
        .send()
        .await
        .map_err(|e| format!("setting devserver workspace on/off: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "devserver workspace on/off returned HTTP {}",
            resp.status()
        ));
    }
    let entry = resp
        .json::<WorkspaceEntry>()
        .await
        .map_err(|e| format!("decoding devserver workspace on/off: {e}"))?;
    row_from_entry(conn, entry)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn devserver_window_row_decodes_amendment_8_shape() {
        // Pins the GET /api/devserver/windows wire (Amendment 8): kind/title are
        // optional; connected/saved drive the reopenable filter; token is empty
        // when the tenant is off. A drift here reds before the menu misbehaves.
        let json = r#"[
          {"label":"workspace-abc-1","prefix":"/api/notes-abc","token":"tok1","kind":"workspace","title":"🏠 /n Window 1","connected":false,"saved":true},
          {"label":"terminal-win-2","prefix":"/control-3","token":"","connected":true,"saved":true}
        ]"#;
        let rows: Vec<DevserverWindowRow> = serde_json::from_str(json).unwrap();
        assert_eq!(rows.len(), 2);
        // Row 0: workspace, reopenable (saved && !connected), kind/title present.
        assert_eq!(rows[0].label, "workspace-abc-1");
        assert_eq!(rows[0].prefix, "/api/notes-abc");
        assert_eq!(rows[0].token, "tok1");
        assert_eq!(rows[0].kind.as_deref(), Some("workspace"));
        assert_eq!(rows[0].title.as_deref(), Some("🏠 /n Window 1"));
        assert!(rows[0].saved && !rows[0].connected);
        // Row 1: optional kind/title absent (defaults None); empty token = off.
        assert_eq!(rows[1].kind, None);
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
    fn set_workspace_on_request_serializes_on_field() {
        assert_eq!(
            serde_json::to_string(&SetWorkspaceOnRequest { on: false }).unwrap(),
            r#"{"on":false}"#
        );
        assert_eq!(
            serde_json::to_string(&SetWorkspaceOnRequest { on: true }).unwrap(),
            r#"{"on":true}"#
        );
    }

    #[test]
    fn row_from_entry_off_row_has_no_url_on_row_has_one() {
        let conn = DevserverConn {
            host: "127.0.0.1".into(),
            port: 8787,
            token: "dt".into(),
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
    fn mounted_terminal_decodes_prefix_and_token() {
        let json = r#"{"prefix":"/api/terminal-9z","token":"tok_term"}"#;
        let term: MountedTerminal = serde_json::from_str(json).unwrap();
        assert_eq!(term.prefix, "/api/terminal-9z");
        assert_eq!(term.token, "tok_term");
    }

    #[test]
    fn open_terminal_request_serializes_label() {
        assert_eq!(
            serde_json::to_string(&OpenTerminalRequest {
                label: "outbound-abc123-3".into()
            })
            .unwrap(),
            r#"{"label":"outbound-abc123-3"}"#
        );
    }

    #[test]
    fn persisted_terminal_entry_decodes_a_bare_array_element() {
        let json = r#"[{"label":"outbound-abc123-3","prefix":"/control-2","token":"tok_t"}]"#;
        let entries: Vec<PersistedTerminalEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "outbound-abc123-3");
        assert_eq!(entries[0].prefix, "/control-2");
        assert_eq!(entries[0].token, "tok_t");
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
    fn local_devserver_config_reads_just_the_token() {
        let json = r#"{"devserver_token":"tok_box","enabled_workspaces":[],"window_configs":[]}"#;
        let cfg: LocalDevserverConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.devserver_token, "tok_box");
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
            },
        );
        assert!(conns.is_connected("ds1"));
        assert_eq!(conns.get("ds1").unwrap().port, 8787);
        assert!(conns.remove("ds1").is_some());
        assert!(!conns.is_connected("ds1"));
    }

    /// Issue 2a guard (@@Alex-mandated): the connect-flow auto-terminal mount
    /// (`open_terminal_with_label`) must carry a JSON body so the devserver's
    /// `Json<OpenTerminalRequest>` endpoint accepts it (200) — the old body-less
    /// POST sent no `Content-Type: application/json` and axum's `Json` extractor
    /// rejected it 415, breaking the connect flow. The loopback handler uses the
    /// SAME strict `Json` extractor the real endpoint does, so a regression back
    /// to a body-less / content-type-less request fails this test.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn open_terminal_with_label_request_is_accepted_by_a_strict_json_endpoint() {
        use axum::{routing::post, Json, Router};

        #[derive(serde::Deserialize)]
        struct Req {
            label: String,
        }
        #[derive(serde::Serialize)]
        struct Resp {
            prefix: String,
            token: String,
        }
        async fn handler(Json(req): Json<Req>) -> Json<Resp> {
            Json(Resp {
                prefix: format!("/api/term-{}", req.label),
                token: "tok_fake".to_string(),
            })
        }

        let app = Router::new().route("/api/devserver/terminals", post(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let conn = DevserverConn {
            host: "127.0.0.1".into(),
            port,
            token: "dt".into(),
        };
        let url = open_terminal_with_label(&conn, "guard")
            .await
            .expect("labeled terminal POST must be accepted (not HTTP 415)");
        assert!(url.contains("/api/term-guard/"), "{url}");
        assert!(url.contains("t=tok_fake"), "{url}");

        // Adversarial half: a body-less POST (the old connect-flow bug) IS
        // rejected 415 by the same endpoint — so the 200 above is the labeled
        // JSON body's doing, not a lax mock.
        let bodyless = http_client()
            .unwrap()
            .post(format!("http://127.0.0.1:{port}/api/devserver/terminals"))
            .bearer_auth("dt")
            .send()
            .await
            .unwrap();
        assert_eq!(
            bodyless.status(),
            reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
    }
}
