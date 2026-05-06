// Tool sandbox for the assistant.
//
// Four standard tools covering the editor's common operations:
//
//   read_file(path)       -> string
//   write_file(path, ...) -> ok / NeedsConfirmation
//   list_files()          -> tree
//   search_content(query) -> hits
//
// All four route through `chan_core::Drive` so the filesystem
// invariants (path sandbox, special-file refusal, atomic writes)
// apply automatically. There's no escape hatch from chan-core's
// gates: even if a backend invents a novel tool call, our
// `StandardTool::execute` never bypasses Drive.
//
// `auto_apply_writes`: when false, `write_file` returns `Pending`
// instead of calling `Drive::write_text`. The caller (server,
// native shell) shows a confirmation UI and re-issues the call
// with the user's approval. When true, writes go straight to disk.
// The flag lives in `LlmConfig`.

use std::sync::Arc;

use chan_core::Drive;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::error::{LlmError, Result};

/// Context the tools see. Owns an `Arc<Drive>` so tool calls cross
/// thread boundaries cheaply; the auto-apply flag is checked
/// per-call so toggling it at runtime takes immediate effect.
#[derive(Clone)]
pub struct ToolContext {
    pub drive: Arc<Drive>,
    pub auto_apply_writes: bool,
}

impl ToolContext {
    pub fn new(drive: Arc<Drive>, auto_apply_writes: bool) -> Self {
        Self {
            drive,
            auto_apply_writes,
        }
    }
}

/// The four built-in tools. Backends see these as named tool
/// schemas; the assistant proposes calls and we dispatch through
/// `StandardTool::execute`. Adding a new built-in tool means a new
/// variant here plus a handler arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardTool {
    ReadFile,
    WriteFile,
    ListFiles,
    SearchContent,
}

impl StandardTool {
    pub fn name(self) -> &'static str {
        match self {
            StandardTool::ReadFile => "read_file",
            StandardTool::WriteFile => "write_file",
            StandardTool::ListFiles => "list_files",
            StandardTool::SearchContent => "search_content",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "read_file" => Some(StandardTool::ReadFile),
            "write_file" => Some(StandardTool::WriteFile),
            "list_files" => Some(StandardTool::ListFiles),
            "search_content" => Some(StandardTool::SearchContent),
            _ => None,
        }
    }
}

/// Outcome of a tool execution. `Pending` is the auto-apply gate:
/// a write that would touch disk but the user hasn't confirmed yet.
/// Consumers surface a UI for `Pending`, then re-issue with
/// `auto_apply_writes = true` (or call `Drive::write_text` directly).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Ok(Json),
    /// A write was proposed but `auto_apply_writes` is off. Body
    /// carries the proposed args so the host can echo them in the
    /// confirmation UI.
    Pending {
        tool: String,
        args: Json,
    },
}

/// Run a tool by name with the given args. Returns either the
/// tool's result JSON or a `Pending` indicator for unconfirmed
/// writes. Unknown tools error.
pub fn execute(name: &str, args: &Json, ctx: &ToolContext) -> Result<ToolOutcome> {
    let Some(tool) = StandardTool::from_name(name) else {
        return Err(LlmError::Tool(format!("unknown tool: {name}")));
    };
    match tool {
        StandardTool::ReadFile => exec_read_file(args, ctx).map(ToolOutcome::Ok),
        StandardTool::ListFiles => exec_list_files(ctx).map(ToolOutcome::Ok),
        StandardTool::SearchContent => exec_search_content(args, ctx).map(ToolOutcome::Ok),
        StandardTool::WriteFile => exec_write_file(args, ctx),
    }
}

fn arg_string<'a>(args: &'a Json, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LlmError::Tool(format!("missing string arg `{key}`")))
}

fn exec_read_file(args: &Json, ctx: &ToolContext) -> Result<Json> {
    let path = arg_string(args, "path")?;
    let content = ctx.drive.read_text(path)?;
    Ok(serde_json::json!({ "path": path, "content": content }))
}

fn exec_list_files(ctx: &ToolContext) -> Result<Json> {
    let tree = ctx.drive.list_tree()?;
    serde_json::to_value(&tree).map_err(|e| LlmError::Tool(format!("serialize tree: {e}")))
}

fn exec_search_content(args: &Json, ctx: &ToolContext) -> Result<Json> {
    let query = arg_string(args, "query")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;
    let res = ctx.drive.search(
        query,
        &chan_core::SearchOpts {
            limit,
            ..Default::default()
        },
    )?;
    serde_json::to_value(&res).map_err(|e| LlmError::Tool(format!("serialize hits: {e}")))
}

fn exec_write_file(args: &Json, ctx: &ToolContext) -> Result<ToolOutcome> {
    let path = arg_string(args, "path")?;
    let content = arg_string(args, "content")?;
    if !ctx.auto_apply_writes {
        return Ok(ToolOutcome::Pending {
            tool: "write_file".into(),
            args: args.clone(),
        });
    }
    ctx.drive.write_text(path, content)?;
    Ok(ToolOutcome::Ok(serde_json::json!({
        "path": path,
        "bytes_written": content.len(),
    })))
}

/// Trait alias. The four `StandardTool`s are the in-tree impls; the
/// trait lets external crates register additional tools later
/// without touching this module. Today nothing uses it; keeping it
/// minimal until a real second consumer appears.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, args: &Json, ctx: &ToolContext) -> Result<ToolOutcome>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_core::Library;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, TempDir, ToolContext) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        let ctx = ToolContext::new(drive, false);
        (cfg, drive_dir, ctx)
    }

    #[test]
    fn read_file_returns_content() {
        let (_cfg, root, ctx) = fixture();
        std::fs::write(root.path().join("a.md"), "hello").unwrap();
        let out = execute("read_file", &serde_json::json!({"path": "a.md"}), &ctx).unwrap();
        match out {
            ToolOutcome::Ok(v) => assert_eq!(v["content"], "hello"),
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn write_file_pending_when_auto_apply_off() {
        let (_cfg, _root, ctx) = fixture();
        let out = execute(
            "write_file",
            &serde_json::json!({"path": "a.md", "content": "x"}),
            &ctx,
        )
        .unwrap();
        match out {
            ToolOutcome::Pending { tool, .. } => assert_eq!(tool, "write_file"),
            _ => panic!("expected Pending"),
        }
        // No file was written.
        assert!(!ctx.drive.exists("a.md"));
    }

    #[test]
    fn write_file_ok_when_auto_apply_on() {
        let (_cfg, _root, mut ctx_owned) = fixture();
        ctx_owned.auto_apply_writes = true;
        let out = execute(
            "write_file",
            &serde_json::json!({"path": "a.md", "content": "hello"}),
            &ctx_owned,
        )
        .unwrap();
        assert!(matches!(out, ToolOutcome::Ok(_)));
        assert_eq!(ctx_owned.drive.read_text("a.md").unwrap(), "hello");
    }

    #[test]
    fn unknown_tool_errors() {
        let (_cfg, _root, ctx) = fixture();
        let err = execute("rm_rf", &serde_json::json!({}), &ctx).unwrap_err();
        assert!(matches!(err, LlmError::Tool(_)));
    }

    #[test]
    fn write_file_rejects_non_text_via_chan_core() {
        let (_cfg, _root, mut ctx) = fixture();
        ctx.auto_apply_writes = true;
        let err = execute(
            "write_file",
            &serde_json::json!({"path": "img.png", "content": "x"}),
            &ctx,
        )
        .unwrap_err();
        // chan-core's editable-text gate fires; the assistant cannot
        // bypass it through the tool sandbox.
        assert!(matches!(err, LlmError::Core(_)));
    }
}
