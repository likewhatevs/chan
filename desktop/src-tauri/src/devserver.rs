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

/// Scrape the devserver bearer token from a control terminal's output. The
/// devserver prints one line per start, `chan devserver: bind=<addr>
/// token=<token>`, so the desktop can recover the token even for a remote
/// devserver whose config file it cannot read.
///
/// `output` is raw PTY bytes (decoded lossily), so it carries ANSI escapes
/// and possibly several `token=` occurrences across restarts. Take the LAST
/// one and read the url-safe token run after it, which stops at the first
/// non-token byte (whitespace, an ANSI escape, end of line).
pub fn scrape_token(output: &str) -> Option<String> {
    output.rmatch_indices("token=").find_map(|(i, marker)| {
        let token: String = output[i + marker.len()..]
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
        .map(|e| {
            let url = assemble_tenant_url(&conn.host, conn.port, &e.prefix, &e.token)?;
            Ok(DevserverWorkspaceRow {
                prefix: e.prefix,
                path: e.path,
                label: e.label,
                on: e.on,
                url,
            })
        })
        .collect()
}

/// `POST /api/devserver/terminals`: mount a standalone terminal tenant and
/// return its assembled tab URL.
pub async fn open_terminal(conn: &DevserverConn) -> Result<String, String> {
    let url = format!(
        "{}/api/devserver/terminals",
        base_origin(&conn.host, conn.port)
    );
    let resp = http_client()?
        .post(&url)
        .bearer_auth(&conn.token)
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
    fn workspace_delete_url_appends_the_prefix_verbatim() {
        assert_eq!(
            workspace_delete_url("127.0.0.1", 8787, "/api/notes-1a2b3c"),
            "http://127.0.0.1:8787/api/devserver/workspaces/api/notes-1a2b3c"
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
    fn scrape_token_reads_the_devserver_line() {
        let out = "some boot noise\nchan devserver: bind=127.0.0.1:8787 token=tok_abc123\n$ ";
        assert_eq!(scrape_token(out).as_deref(), Some("tok_abc123"));
    }

    #[test]
    fn scrape_token_takes_the_last_occurrence_across_restarts() {
        let out = "chan devserver: bind=… token=old_TOKEN\n[restart]\nchan devserver: bind=… token=new-TOKEN_2\n";
        assert_eq!(scrape_token(out).as_deref(), Some("new-TOKEN_2"));
    }

    #[test]
    fn scrape_token_stops_at_ansi_or_whitespace() {
        // Raw PTY bytes carry ANSI; the token run stops at the escape byte.
        let out = "chan devserver: token=tok_xyz\x1b[0m extra";
        assert_eq!(scrape_token(out).as_deref(), Some("tok_xyz"));
    }

    #[test]
    fn scrape_token_none_when_absent_or_empty() {
        assert_eq!(scrape_token("no token here\n$ "), None);
        assert_eq!(scrape_token("token= \nnext"), None);
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
}
