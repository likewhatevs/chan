//! Publish chan's MCP bridge into external agent discovery files.
//!
//! The MCP bridge socket is per `chan serve` process, so chan-owned
//! entries are refreshed in place. Entries with the same name that do
//! not look like chan's `__mcp-proxy` command are treated as user-owned
//! and left untouched.

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use toml::value::Table;

const SERVER_NAME: &str = "chan";
const MCP_PROXY_ARG: &str = "__mcp-proxy";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChanMcpDescriptor {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
}

impl ChanMcpDescriptor {
    pub(crate) fn from_socket(socket_path: &Path) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        Some(Self::from_parts(exe, socket_path))
    }

    fn from_parts(command: impl Into<PathBuf>, socket_path: impl AsRef<Path>) -> Self {
        Self {
            command: command.into().display().to_string(),
            args: vec![
                MCP_PROXY_ARG.to_string(),
                socket_path.as_ref().display().to_string(),
            ],
        }
    }
}

pub(crate) fn publish_for_agents(drive_root: &Path, socket_path: &Path) {
    let Some(descriptor) = ChanMcpDescriptor::from_socket(socket_path) else {
        tracing::warn!("mcp discovery skipped: current executable path is unavailable");
        return;
    };
    let Some(home) = dirs::home_dir() else {
        tracing::warn!("mcp discovery skipped: home directory is unavailable");
        return;
    };

    let targets = [
        AgentTarget::claude(home.join(".claude.json"), drive_root.to_path_buf()),
        AgentTarget::codex(home.join(".codex/config.toml")),
        AgentTarget::gemini(home.join(".gemini/settings.json")),
    ];
    for target in targets {
        if let Err(e) = target.publish(&descriptor) {
            tracing::warn!("mcp discovery publish failed for {}: {e}", target.name());
        }
    }
}

#[derive(Debug, Clone)]
enum AgentTarget {
    Claude { path: PathBuf, drive_root: PathBuf },
    Codex { path: PathBuf },
    Gemini { path: PathBuf },
}

impl AgentTarget {
    fn claude(path: PathBuf, drive_root: PathBuf) -> Self {
        Self::Claude { path, drive_root }
    }

    fn codex(path: PathBuf) -> Self {
        Self::Codex { path }
    }

    fn gemini(path: PathBuf) -> Self {
        Self::Gemini { path }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Claude { .. } => "claude",
            Self::Codex { .. } => "codex",
            Self::Gemini { .. } => "gemini",
        }
    }

    fn publish(&self, descriptor: &ChanMcpDescriptor) -> Result<()> {
        match self {
            Self::Claude { path, drive_root } => {
                publish_claude_config(path, drive_root, descriptor)
            }
            Self::Codex { path } => publish_codex_config(path, descriptor),
            Self::Gemini { path } => publish_gemini_config(path, descriptor),
        }
    }
}

fn publish_claude_config(
    path: &Path,
    drive_root: &Path,
    descriptor: &ChanMcpDescriptor,
) -> Result<()> {
    let mut root = read_json_object(path)?;
    let projects = object_field(&mut root, "projects")?;
    let project = object_field(projects, &drive_root.display().to_string())?;
    let servers = object_field(project, "mcpServers")?;
    upsert_json_server(servers, descriptor)?;
    write_json_atomic(path, &Value::Object(root))
}

fn publish_gemini_config(path: &Path, descriptor: &ChanMcpDescriptor) -> Result<()> {
    let mut root = read_json_object(path)?;
    let servers = object_field(&mut root, "mcpServers")?;
    upsert_json_server(servers, descriptor)?;
    write_json_atomic(path, &Value::Object(root))
}

fn publish_codex_config(path: &Path, descriptor: &ChanMcpDescriptor) -> Result<()> {
    let mut root = read_toml_table(path)?;
    let servers = table_field(&mut root, "mcp_servers")?;
    match servers.get(SERVER_NAME) {
        Some(existing) if !toml_server_is_chan_owned(existing) => {
            tracing::warn!("codex mcp server named {SERVER_NAME:?} exists and is user-owned");
            return Ok(());
        }
        _ => {}
    }
    let mut server = Table::new();
    server.insert(
        "command".to_string(),
        toml::Value::String(descriptor.command.clone()),
    );
    server.insert(
        "args".to_string(),
        toml::Value::Array(
            descriptor
                .args
                .iter()
                .map(|arg| toml::Value::String(arg.clone()))
                .collect(),
        ),
    );
    servers.insert(SERVER_NAME.to_string(), toml::Value::Table(server));
    write_toml_atomic(path, &toml::Value::Table(root))
}

fn upsert_json_server(
    servers: &mut Map<String, Value>,
    descriptor: &ChanMcpDescriptor,
) -> Result<()> {
    match servers.get(SERVER_NAME) {
        Some(existing) if !json_server_is_chan_owned(existing) => {
            tracing::warn!("mcp server named {SERVER_NAME:?} exists and is user-owned");
            return Ok(());
        }
        _ => {}
    }
    let mut server = Map::new();
    server.insert(
        "command".to_string(),
        Value::String(descriptor.command.clone()),
    );
    server.insert(
        "args".to_string(),
        Value::Array(
            descriptor
                .args
                .iter()
                .map(|arg| Value::String(arg.clone()))
                .collect(),
        ),
    );
    server.insert("env".to_string(), Value::Object(Map::new()));
    servers.insert(SERVER_NAME.to_string(), Value::Object(server));
    Ok(())
}

fn json_server_is_chan_owned(value: &Value) -> bool {
    value
        .get("args")
        .and_then(Value::as_array)
        .and_then(|args| args.first())
        .and_then(Value::as_str)
        == Some(MCP_PROXY_ARG)
}

fn toml_server_is_chan_owned(value: &toml::Value) -> bool {
    value
        .get("args")
        .and_then(toml::Value::as_array)
        .and_then(|args| args.first())
        .and_then(toml::Value::as_str)
        == Some(MCP_PROXY_ARG)
}

fn object_field<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        anyhow::bail!("json field {key:?} exists but is not an object");
    }
    Ok(entry.as_object_mut().expect("object checked above"))
}

fn table_field<'a>(root: &'a mut Table, key: &str) -> Result<&'a mut Table> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| toml::Value::Table(Table::new()));
    if !entry.is_table() {
        anyhow::bail!("toml field {key:?} exists but is not a table");
    }
    Ok(entry.as_table_mut().expect("table checked above"))
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if raw.trim().is_empty() {
                return Ok(Map::new());
            }
            let value: Value = serde_json::from_str(&raw)
                .with_context(|| format!("parse json {}", path.display()))?;
            value
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("json root is not an object"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

fn read_toml_table(path: &Path) -> Result<Table> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if raw.trim().is_empty() {
                return Ok(Table::new());
            }
            let value: toml::Value =
                toml::from_str(&raw).with_context(|| format!("parse toml {}", path.display()))?;
            value
                .as_table()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("toml root is not a table"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Table::new()),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &bytes)
}

fn write_toml_atomic(path: &Path, value: &toml::Value) -> Result<()> {
    let text = toml::to_string_pretty(value)?;
    write_atomic(path, text.as_bytes())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("path has no valid file name: {}", path.display()))?;
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file =
            fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("write {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", tmp.display()))?;
        drop(file);
        fs::rename(&tmp, path)
            .with_context(|| format!("rename {} to {}", tmp.display(), path.display()))
    })();
    let _ = fs::remove_file(&tmp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(socket: &str) -> ChanMcpDescriptor {
        ChanMcpDescriptor::from_parts("/bin/chan", socket)
    }

    #[test]
    fn codex_publish_adds_entry_and_preserves_existing_servers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[mcp_servers.existing]
command = "node"
args = ["server.js"]
"#,
        )
        .expect("seed config");

        publish_codex_config(&path, &descriptor("/tmp/chan-a.sock")).expect("publish");
        let root = read_toml_table(&path).expect("read config");
        let servers = root
            .get("mcp_servers")
            .and_then(toml::Value::as_table)
            .expect("servers");

        assert!(servers.contains_key("existing"));
        assert_eq!(
            servers
                .get("chan")
                .and_then(toml::Value::as_table)
                .and_then(|server| server.get("args"))
                .and_then(toml::Value::as_array)
                .and_then(|args| args.get(1))
                .and_then(toml::Value::as_str),
            Some("/tmp/chan-a.sock")
        );
    }

    #[test]
    fn codex_publish_refreshes_chan_owned_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[mcp_servers.chan]
command = "/bin/chan"
args = ["__mcp-proxy", "/tmp/old.sock"]
"#,
        )
        .expect("seed config");

        publish_codex_config(&path, &descriptor("/tmp/new.sock")).expect("publish");
        let root = read_toml_table(&path).expect("read config");
        let args = root["mcp_servers"]["chan"]["args"]
            .as_array()
            .expect("args");
        assert_eq!(args[1].as_str(), Some("/tmp/new.sock"));
    }

    #[test]
    fn codex_publish_does_not_overwrite_user_owned_chan_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[mcp_servers.chan]
command = "node"
args = ["mine.js"]
"#,
        )
        .expect("seed config");

        publish_codex_config(&path, &descriptor("/tmp/new.sock")).expect("publish");
        let root = read_toml_table(&path).expect("read config");
        assert_eq!(
            root["mcp_servers"]["chan"]["command"].as_str(),
            Some("node")
        );
    }

    #[test]
    fn claude_publish_adds_project_local_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude.json");
        let drive = PathBuf::from("/drive/root");

        publish_claude_config(&path, &drive, &descriptor("/tmp/chan.sock")).expect("publish");
        let root = read_json_object(&path).expect("read config");
        let chan = &root["projects"]["/drive/root"]["mcpServers"]["chan"];

        assert_eq!(chan["command"], "/bin/chan");
        assert_eq!(chan["args"][0], "__mcp-proxy");
        assert_eq!(chan["args"][1], "/tmp/chan.sock");
    }

    #[test]
    fn gemini_publish_adds_entry_and_preserves_existing_servers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"mcpServers":{"existing":{"command":"node","args":["server.js"]}}}"#,
        )
        .expect("seed config");

        publish_gemini_config(&path, &descriptor("/tmp/chan.sock")).expect("publish");
        let root = read_json_object(&path).expect("read config");

        assert_eq!(root["mcpServers"]["existing"]["command"], "node");
        assert_eq!(root["mcpServers"]["chan"]["args"][1], "/tmp/chan.sock");
    }
}
