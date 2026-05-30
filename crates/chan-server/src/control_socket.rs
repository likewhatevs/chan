//! First-party control socket for local `chan` CLI helpers.
//!
//! MCP stays scoped to workspace tools for external agents. This socket is
//! for UI commands from chan-spawned terminals, such as `chan open`,
//! where the command must target one frontend window in the already
//! running server process.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(unix)]
use chan_workspace::Workspace;
#[cfg(unix)]
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::broadcast;
#[cfg(unix)]
use tokio::task::JoinHandle;

use crate::state::WorkspaceCell;
use crate::terminal_sessions::Registry as TerminalRegistry;

/// Settable handle to the terminal registry. The registry is built after
/// the control socket starts (it needs the control socket path for
/// `$CHAN_CONTROL_SOCKET`), so the caller passes an empty cell here and
/// fills it once the registry exists. Category-2 requests
/// (`cs term write` / `term list`) read it.
pub type TerminalRegistryCell = Arc<OnceLock<Arc<TerminalRegistry>>>;

#[cfg(unix)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    // Category 1: open a UI tab in the originating window.
    OpenPath {
        window_id: String,
        path: PathBuf,
    },
    OpenGraph {
        window_id: String,
        #[serde(default)]
        path: Option<PathBuf>,
    },
    OpenTermNew {
        window_id: String,
        #[serde(default)]
        path: Option<PathBuf>,
        #[serde(default)]
        tab_name: Option<String>,
        #[serde(default)]
        tab_group: Option<String>,
    },
    OpenDashboard {
        window_id: String,
        #[serde(default)]
        carousel_index: Option<u32>,
    },
    // Category 2: act on / inspect live PTY sessions via the registry.
    TermWrite {
        #[serde(default)]
        tab_name: Option<String>,
        #[serde(default)]
        tab_group: Option<String>,
        data: String,
    },
    TermList,
}

#[cfg(unix)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok { message: String },
    Error { message: String },
}

#[cfg(unix)]
#[derive(Debug, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
// The shared `Open` prefix is the wire contract: serde renames each
// variant to its `open_*` command string that the SPA's
// `handleWindowCommand` matches on. Renaming to drop the prefix would
// rename the wire command and break the SPA.
#[allow(clippy::enum_variant_names)]
enum WindowCommand {
    OpenFile {
        path: String,
    },
    OpenBrowser {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        select: Option<String>,
        #[serde(skip_serializing_if = "is_false")]
        enter: bool,
    },
    OpenGraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        is_dir: bool,
    },
    OpenTermNew {
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
    },
    OpenDashboard {
        #[serde(skip_serializing_if = "Option::is_none")]
        carousel_index: Option<u32>,
    },
}

#[cfg(unix)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(unix)]
#[derive(Debug, Serialize)]
struct WindowCommandFrame {
    #[serde(rename = "type")]
    frame_type: &'static str,
    window_id: String,
    #[serde(flatten)]
    command: WindowCommand,
}

#[cfg(unix)]
pub struct ControlHandle {
    socket_path: PathBuf,
    accept_loop: Option<JoinHandle<()>>,
}

#[cfg(not(unix))]
pub struct ControlHandle {
    socket_path: PathBuf,
}

impl ControlHandle {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

#[cfg(unix)]
impl Drop for ControlHandle {
    fn drop(&mut self) {
        if let Some(h) = self.accept_loop.take() {
            h.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

pub fn pick_socket_path() -> PathBuf {
    crate::mcp_bridge::pick_named_socket_path("control")
}

#[cfg(unix)]
pub fn start(
    socket_path: PathBuf,
    workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
    events_tx: broadcast::Sender<String>,
    self_writes: Arc<crate::self_writes::SelfWrites>,
    terminal_registry: TerminalRegistryCell,
) -> std::io::Result<ControlHandle> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;

    let accept_loop = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("control socket accept: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let workspace_cell = workspace_cell.clone();
            let events_tx = events_tx.clone();
            let self_writes = self_writes.clone();
            let terminal_registry = terminal_registry.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => ControlResponse::Error {
                        message: "empty control request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<ControlRequest>(&line) {
                        Ok(req) => handle_request(
                            req,
                            &workspace_cell,
                            &events_tx,
                            &self_writes,
                            terminal_registry.get(),
                        ),
                        Err(e) => ControlResponse::Error {
                            message: format!("invalid control request: {e}"),
                        },
                    },
                    Err(e) => ControlResponse::Error {
                        message: format!("read control request: {e}"),
                    },
                };
                if let Ok(mut out) = serde_json::to_vec(&response) {
                    out.push(b'\n');
                    let _ = write.write_all(&out).await;
                }
            });
        }
    });

    Ok(ControlHandle {
        socket_path,
        accept_loop: Some(accept_loop),
    })
}

#[cfg(not(unix))]
pub fn start(
    _socket_path: PathBuf,
    _workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
    _events_tx: broadcast::Sender<String>,
    _self_writes: Arc<crate::self_writes::SelfWrites>,
    _terminal_registry: TerminalRegistryCell,
) -> std::io::Result<ControlHandle> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "control socket requires unix-domain sockets",
    ))
}

#[cfg(unix)]
fn handle_request(
    req: ControlRequest,
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    events_tx: &broadcast::Sender<String>,
    self_writes: &crate::self_writes::SelfWrites,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
) -> ControlResponse {
    match req {
        ControlRequest::OpenPath { window_id, path } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(open_path(
                &workspace,
                self_writes,
                &window_id,
                &path,
                events_tx,
            ))
        }
        ControlRequest::OpenGraph { window_id, path } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(open_graph(
                &workspace,
                &window_id,
                path.as_deref(),
                events_tx,
            ))
        }
        ControlRequest::OpenTermNew {
            window_id,
            path,
            tab_name,
            tab_group,
        } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(open_term_new(
                &workspace,
                &window_id,
                path.as_deref(),
                tab_name,
                tab_group,
                events_tx,
            ))
        }
        ControlRequest::OpenDashboard {
            window_id,
            carousel_index,
        } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            into_response(open_dashboard(&window_id, carousel_index, events_tx))
        }
        ControlRequest::TermWrite {
            tab_name,
            tab_group,
            data,
        } => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_write(
                registry,
                tab_name.as_deref(),
                tab_group.as_deref(),
                &data,
            ))
        }
        ControlRequest::TermList => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_list(registry))
        }
    }
}

#[cfg(unix)]
fn require_window_id(window_id: &str) -> Result<(), String> {
    if window_id.trim().is_empty() {
        Err("window_id is required".into())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn into_response(result: Result<String, String>) -> ControlResponse {
    match result {
        Ok(message) => ControlResponse::Ok { message },
        Err(message) => ControlResponse::Error { message },
    }
}

#[cfg(unix)]
fn workspace_from_cell(
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
) -> Result<Arc<Workspace>, String> {
    let cell = workspace_cell
        .read()
        .map_err(|_| "workspace cell lock poisoned".to_string())?;
    let cell = cell
        .as_ref()
        .ok_or_else(|| "workspace cell unavailable".to_string())?;
    Ok(cell.workspace.clone())
}

#[cfg(unix)]
fn send_window_command(
    window_id: &str,
    command: WindowCommand,
    events_tx: &broadcast::Sender<String>,
) -> Result<(), String> {
    let frame = WindowCommandFrame {
        frame_type: "window_command",
        window_id: window_id.to_string(),
        command,
    };
    let raw = serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
    let _ = events_tx.send(raw);
    Ok(())
}

/// Resolve an optional requested path to a workspace-relative path plus
/// whether it is a directory. `None` / the workspace root resolve to
/// `(None, _)`, which the SPA treats as "no specific target".
#[cfg(unix)]
fn resolve_optional_rel(
    workspace: &Workspace,
    requested: Option<&Path>,
) -> Result<Option<(String, bool)>, String> {
    let Some(requested) = requested else {
        return Ok(None);
    };
    let rel = abs_to_workspace_rel(workspace.root(), requested)?;
    if rel.is_empty() {
        return Ok(None);
    }
    let is_dir = workspace
        .stat(&rel)
        .map(|stat| stat.is_dir)
        .unwrap_or(false);
    Ok(Some((rel, is_dir)))
}

/// Category 1: open the documentation graph in the originating window,
/// optionally focused on a file or directory.
#[cfg(unix)]
fn open_graph(
    workspace: &Workspace,
    window_id: &str,
    requested: Option<&Path>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let resolved = resolve_optional_rel(workspace, requested)?;
    let (path, is_dir) = match &resolved {
        Some((rel, is_dir)) => (Some(rel.clone()), *is_dir),
        None => (None, false),
    };
    send_window_command(
        window_id,
        WindowCommand::OpenGraph {
            path: path.clone(),
            is_dir,
        },
        events_tx,
    )?;
    Ok(match path {
        Some(rel) => format!("graph request queued for {rel}"),
        None => "graph request queued".into(),
    })
}

/// Category 1: open a new terminal tab in the originating window. A
/// requested file resolves to its parent directory as the cwd.
#[cfg(unix)]
fn open_term_new(
    workspace: &Workspace,
    window_id: &str,
    requested: Option<&Path>,
    tab_name: Option<String>,
    tab_group: Option<String>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let cwd = match resolve_optional_rel(workspace, requested)? {
        Some((rel, true)) => Some(rel),
        Some((rel, false)) => {
            let parent = parent_rel(&rel);
            (!parent.is_empty()).then_some(parent)
        }
        None => None,
    };
    send_window_command(
        window_id,
        WindowCommand::OpenTermNew {
            cwd: cwd.clone(),
            tab_name,
            tab_group,
        },
        events_tx,
    )?;
    Ok(match cwd {
        Some(rel) => format!("terminal request queued for {rel}"),
        None => "terminal request queued".into(),
    })
}

/// Category 1: open a Dashboard tab in the originating window.
#[cfg(unix)]
fn open_dashboard(
    window_id: &str,
    carousel_index: Option<u32>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    send_window_command(
        window_id,
        WindowCommand::OpenDashboard { carousel_index },
        events_tx,
    )?;
    Ok("dashboard request queued".into())
}

/// Category 2: write raw bytes to the matching live PTY sessions. At
/// least one selector is required so a missing filter cannot fan out to
/// every terminal by accident.
#[cfg(unix)]
fn term_write(
    registry: &TerminalRegistry,
    tab_name: Option<&str>,
    tab_group: Option<&str>,
    data: &str,
) -> Result<String, String> {
    if tab_name.is_none() && tab_group.is_none() {
        return Err("term write needs a tab name and/or group selector".into());
    }
    let written = registry.write_input_matching(tab_name, tab_group, data.as_bytes());
    if written == 0 {
        return Err("no live terminal session matched".into());
    }
    Ok(format!("wrote to {written} terminal session(s)"))
}

/// Category 2: list live terminal sessions as JSON, grouped by group.
#[cfg(unix)]
fn term_list(registry: &TerminalRegistry) -> Result<String, String> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for summary in registry.session_summaries() {
        let entry = serde_json::json!({
            "name": summary.tab_name,
            "session_id": summary.session_id,
            "cwd": summary.cwd.map(|p| p.to_string_lossy().into_owned()),
        });
        groups.entry(summary.tab_group).or_default().push(entry);
    }
    let payload = serde_json::json!({ "groups": groups });
    serde_json::to_string(&payload).map_err(|e| format!("encode terminal list: {e}"))
}

#[cfg(unix)]
fn open_path(
    workspace: &Workspace,
    self_writes: &crate::self_writes::SelfWrites,
    window_id: &str,
    requested: &Path,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let rel = abs_to_workspace_rel(workspace.root(), requested)?;
    if rel.is_empty() {
        let frame = WindowCommandFrame {
            frame_type: "window_command",
            window_id: window_id.to_string(),
            command: WindowCommand::OpenBrowser {
                path: String::new(),
                select: None,
                enter: true,
            },
        };
        let raw =
            serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
        let _ = events_tx.send(raw);
        return Ok("open request queued for /".into());
    }
    let stat = workspace.stat(&rel).ok();
    let command = if let Some(stat) = stat {
        if stat.is_dir {
            WindowCommand::OpenBrowser {
                path: rel.clone(),
                select: None,
                enter: true,
            }
        } else if rel.ends_with(".md") {
            WindowCommand::OpenFile { path: rel.clone() }
        } else {
            let parent = parent_rel(&rel);
            WindowCommand::OpenBrowser {
                path: parent,
                select: Some(rel.clone()),
                enter: false,
            }
        }
    } else if rel.ends_with(".md") {
        // Note before the write so the watcher's Created event is in the
        // suppression set before it can fire (see files.rs::api_write_file).
        self_writes.note(&rel);
        workspace
            .write_text(&rel, "")
            .map_err(|e| format!("create {rel}: {e}"))?;
        WindowCommand::OpenFile { path: rel.clone() }
    } else {
        return Err("file does not exist; chan open creates `.md` files only".into());
    };

    let frame = WindowCommandFrame {
        frame_type: "window_command",
        window_id: window_id.to_string(),
        command,
    };
    let raw = serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
    let _ = events_tx.send(raw);
    Ok(format!("open request queued for {rel}"))
}

#[cfg(unix)]
fn abs_to_workspace_rel(root: &Path, requested: &Path) -> Result<String, String> {
    if !requested.is_absolute() {
        return Err("control path must be absolute".into());
    }
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("canonicalize workspace root: {e}"))?;
    let existing_or_parent = if requested.exists() {
        requested
    } else {
        requested
            .parent()
            .ok_or_else(|| "path has no parent".to_string())?
    };
    let canon = existing_or_parent
        .canonicalize()
        .map_err(|e| format!("canonicalize path: {e}"))?;
    if !canon.starts_with(&root_canon) {
        return Err("path escapes workspace root".into());
    }
    let candidate = if requested.exists() {
        canon
    } else {
        canon.join(
            requested
                .file_name()
                .ok_or_else(|| "path has no file name".to_string())?,
        )
    };
    let rel = candidate
        .strip_prefix(&root_canon)
        .map_err(|_| "path escapes workspace root".to_string())?;
    Ok(path_to_posix(rel))
}

#[cfg(unix)]
fn path_to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(unix)]
fn parent_rel(rel: &str) -> String {
    rel.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn parent_rel_returns_empty_for_root_file() {
        assert_eq!(parent_rel("a.png"), "");
        assert_eq!(parent_rel("notes/a.png"), "notes");
    }

    #[test]
    fn handle_request_reports_poisoned_workspace_cell() {
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let poisoned = workspace_cell.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.write().expect("poison setup");
            panic!("poison workspace cell");
        })
        .join();
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, _) = broadcast::channel(1);

        let response = handle_request(
            ControlRequest::OpenPath {
                window_id: "window-a".to_string(),
                path: PathBuf::from("/tmp/note.md"),
            },
            &workspace_cell,
            &tx,
            &self_writes,
            None,
        );

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, "workspace cell lock poisoned");
            }
            ControlResponse::Ok { message } => panic!("unexpected ok response: {message}"),
        }
    }

    #[test]
    fn open_path_creates_markdown_and_broadcasts_window_command() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        std::fs::create_dir_all(root.path().join("notes")).expect("notes dir");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &workspace,
            &self_writes,
            "window-a",
            &root.path().join("notes/new.md"),
            &tx,
        )
        .expect("open path");

        assert!(message.contains("notes/new.md"));
        assert!(workspace.exists("notes/new.md"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["type"], "window_command");
        assert_eq!(frame["window_id"], "window-a");
        assert_eq!(frame["command"], "open_file");
        assert_eq!(frame["path"], "notes/new.md");
    }

    #[test]
    fn open_path_enters_existing_directory() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        std::fs::create_dir_all(root.path().join("notes/sub")).expect("sub dir");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &workspace,
            &self_writes,
            "window-a",
            &root.path().join("notes/sub"),
            &tx,
        )
        .expect("open path");

        assert!(message.contains("notes/sub"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["type"], "window_command");
        assert_eq!(frame["window_id"], "window-a");
        assert_eq!(frame["command"], "open_browser");
        assert_eq!(frame["path"], "notes/sub");
        assert_eq!(frame["select"], Value::Null);
        assert_eq!(frame["enter"], true);
    }

    fn test_workspace() -> (tempfile::TempDir, tempfile::TempDir, Arc<Workspace>) {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        (cfg, root, workspace)
    }

    fn empty_registry() -> (tempfile::TempDir, TerminalRegistry) {
        use crate::config::TerminalConfig;
        use crate::terminal_sessions::RegistryConfig;
        let root = tempfile::tempdir().expect("workspace root");
        let registry = TerminalRegistry::new(RegistryConfig {
            workspace_root: root.path().to_path_buf(),
            mcp_socket_path: None,
            control_socket_path: None,
            terminal: TerminalConfig::default(),
        });
        (root, registry)
    }

    #[test]
    fn open_graph_broadcasts_window_command_for_a_directory() {
        let (_cfg, root, workspace) = test_workspace();
        std::fs::create_dir_all(root.path().join("notes/sub")).expect("sub dir");
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_graph(
            &workspace,
            "window-a",
            Some(&root.path().join("notes/sub")),
            &tx,
        )
        .expect("open graph");

        assert!(message.contains("notes/sub"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_graph");
        assert_eq!(frame["path"], "notes/sub");
        assert_eq!(frame["is_dir"], true);
    }

    #[test]
    fn open_graph_without_a_path_targets_the_whole_graph() {
        let (_cfg, _root, workspace) = test_workspace();
        let (tx, mut rx) = broadcast::channel(4);

        open_graph(&workspace, "window-a", None, &tx).expect("open graph");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_graph");
        assert_eq!(frame["path"], Value::Null);
        assert_eq!(frame["is_dir"], false);
    }

    #[test]
    fn open_term_new_uses_the_parent_directory_for_a_file() {
        let (_cfg, root, workspace) = test_workspace();
        std::fs::create_dir_all(root.path().join("notes")).expect("notes dir");
        std::fs::write(root.path().join("notes/today.md"), "x").expect("write file");
        let (tx, mut rx) = broadcast::channel(4);

        open_term_new(
            &workspace,
            "window-a",
            Some(&root.path().join("notes/today.md")),
            Some("build".into()),
            Some("foobar".into()),
            &tx,
        )
        .expect("open term new");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_term_new");
        assert_eq!(frame["cwd"], "notes");
        assert_eq!(frame["tab_name"], "build");
        assert_eq!(frame["tab_group"], "foobar");
    }

    #[test]
    fn open_dashboard_carries_the_carousel_index() {
        let (tx, mut rx) = broadcast::channel(4);

        open_dashboard("window-a", Some(2), &tx).expect("open dashboard");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_dashboard");
        assert_eq!(frame["carousel_index"], 2);
    }

    #[test]
    fn term_write_requires_a_selector() {
        let (_root, registry) = empty_registry();
        let err = term_write(&registry, None, None, "ls").expect_err("no selector");
        assert!(err.contains("selector"), "got: {err}");
    }

    #[test]
    fn term_write_reports_no_match_on_an_empty_registry() {
        let (_root, registry) = empty_registry();
        let err = term_write(&registry, Some("nope"), None, "ls").expect_err("no match");
        assert!(err.contains("no live terminal session"), "got: {err}");
    }

    #[test]
    fn term_list_has_no_groups_without_sessions() {
        let (_root, registry) = empty_registry();
        let json = term_list(&registry).expect("term list");
        let value: Value = serde_json::from_str(&json).expect("json");
        assert_eq!(value["groups"], serde_json::json!({}));
    }
}
