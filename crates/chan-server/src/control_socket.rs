//! First-party control socket for local `chan` CLI helpers.
//!
//! MCP stays scoped to drive tools for external agents. This socket is
//! for UI commands from chan-spawned terminals, such as `chan open`,
//! where the command must target one frontend window in the already
//! running server process.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[cfg(unix)]
use chan_drive::Drive;
#[cfg(unix)]
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::broadcast;
#[cfg(unix)]
use tokio::task::JoinHandle;

use crate::state::DriveCell;

#[cfg(unix)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    OpenPath { window_id: String, path: PathBuf },
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
    drive_cell: Arc<RwLock<Option<DriveCell>>>,
    events_tx: broadcast::Sender<String>,
    self_writes: Arc<crate::self_writes::SelfWrites>,
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
            let drive_cell = drive_cell.clone();
            let events_tx = events_tx.clone();
            let self_writes = self_writes.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => ControlResponse::Error {
                        message: "empty control request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<ControlRequest>(&line) {
                        Ok(req) => handle_request(req, &drive_cell, &events_tx, &self_writes),
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
    _drive_cell: Arc<RwLock<Option<DriveCell>>>,
    _events_tx: broadcast::Sender<String>,
    _self_writes: Arc<crate::self_writes::SelfWrites>,
) -> std::io::Result<ControlHandle> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "control socket requires unix-domain sockets",
    ))
}

#[cfg(unix)]
fn handle_request(
    req: ControlRequest,
    drive_cell: &Arc<RwLock<Option<DriveCell>>>,
    events_tx: &broadcast::Sender<String>,
    self_writes: &crate::self_writes::SelfWrites,
) -> ControlResponse {
    match req {
        ControlRequest::OpenPath { window_id, path } => {
            if window_id.trim().is_empty() {
                return ControlResponse::Error {
                    message: "window_id is required".into(),
                };
            }
            let drive = {
                let cell = match drive_cell.read() {
                    Ok(cell) => cell,
                    Err(_) => {
                        return ControlResponse::Error {
                            message: "drive cell lock poisoned".into(),
                        };
                    }
                };
                let Some(cell) = cell.as_ref() else {
                    return ControlResponse::Error {
                        message: "drive cell unavailable".into(),
                    };
                };
                cell.drive.clone()
            };
            match open_path(&drive, self_writes, &window_id, &path, events_tx) {
                Ok(message) => ControlResponse::Ok { message },
                Err(message) => ControlResponse::Error { message },
            }
        }
    }
}

#[cfg(unix)]
fn open_path(
    drive: &Drive,
    self_writes: &crate::self_writes::SelfWrites,
    window_id: &str,
    requested: &Path,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let rel = abs_to_drive_rel(drive.root(), requested)?;
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
    let stat = drive.stat(&rel).ok();
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
        drive
            .write_text(&rel, "")
            .map_err(|e| format!("create {rel}: {e}"))?;
        self_writes.note(&rel);
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
fn abs_to_drive_rel(root: &Path, requested: &Path) -> Result<String, String> {
    if !requested.is_absolute() {
        return Err("control path must be absolute".into());
    }
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("canonicalize drive root: {e}"))?;
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
        return Err("path escapes drive root".into());
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
        .map_err(|_| "path escapes drive root".to_string())?;
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
    fn handle_request_reports_poisoned_drive_cell() {
        let drive_cell: Arc<RwLock<Option<DriveCell>>> = Arc::new(RwLock::new(None));
        let poisoned = drive_cell.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.write().expect("poison setup");
            panic!("poison drive cell");
        })
        .join();
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, _) = broadcast::channel(1);

        let response = handle_request(
            ControlRequest::OpenPath {
                window_id: "window-a".to_string(),
                path: PathBuf::from("/tmp/note.md"),
            },
            &drive_cell,
            &tx,
            &self_writes,
        );

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, "drive cell lock poisoned");
            }
            ControlResponse::Ok { message } => panic!("unexpected ok response: {message}"),
        }
    }

    #[test]
    fn open_path_creates_markdown_and_broadcasts_window_command() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("drive root");
        std::fs::create_dir_all(root.path().join("notes")).expect("notes dir");
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_drive(root.path()).expect("register drive");
        let drive = lib.open_drive(root.path()).expect("open drive");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &drive,
            &self_writes,
            "window-a",
            &root.path().join("notes/new.md"),
            &tx,
        )
        .expect("open path");

        assert!(message.contains("notes/new.md"));
        assert!(drive.exists("notes/new.md"));
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
        let root = tempfile::tempdir().expect("drive root");
        std::fs::create_dir_all(root.path().join("notes/sub")).expect("sub dir");
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_drive(root.path()).expect("register drive");
        let drive = lib.open_drive(root.path()).expect("open drive");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &drive,
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
}
