// Tool sandbox for the assistant.
//
// Four standard tools covering the editor's common operations:
//
//   read_file(path)       -> string
//   write_file(path, ...) -> ok / NeedsConfirmation
//   list_files()          -> tree
//   search_content(query) -> hits
//
// All four route through `chan_drive::Drive` so the filesystem
// invariants (path sandbox, special-file refusal, atomic writes)
// apply automatically. There's no escape hatch from chan-drive's
// gates: even if a backend invents a novel tool call, our
// `StandardTool::execute` never bypasses Drive.
//
// `auto_apply_writes`: when false, `write_file` returns `Pending`
// instead of calling `Drive::write_text`. The caller (server,
// native shell) shows a confirmation UI and re-issues the call
// with the user's approval. When true, writes go straight to disk.
// The flag lives in `LlmConfig`.

use std::sync::Arc;

use chan_drive::Drive;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::error::{LlmError, Result};

/// Soft cap on `read_file` output. 256 KiB is roughly 250k chars of
/// English, well past any realistic single-shot read for assistant
/// reasoning, and a tiny fraction of any frontier model's context
/// window. Past this we truncate and tell the model to issue a
/// follow-up read with the suffix it actually wants. Without the
/// cap, a misnamed binary or a runaway pasted-image markdown can
/// bloat the next turn's request body and the user's token bill.
pub const READ_FILE_CAP_BYTES: usize = 256 * 1024;

/// Soft cap on `list_files` entries. The drive layer caps the walk
/// at 500k; this layer caps the slice we hand to the model at a
/// number that fits a model's context plus leaves room for the
/// rest of the conversation. 2k entries renders as a few hundred
/// KB of JSON which is already pushing it; past this we truncate
/// and tell the model to narrow with the `prefix` arg.
pub const LIST_FILES_CAP_ENTRIES: usize = 2_000;

/// Hard cap on `search_content` result count. The model can ask
/// for any reasonable number; we clamp at 100 so a runaway
/// `limit=1000000` doesn't pull back a million hits the assistant
/// can't reason about anyway.
pub const SEARCH_CONTENT_MAX_LIMIT: u32 = 100;
pub const SEARCH_CONTENT_DEFAULT_LIMIT: u32 = 20;

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
        StandardTool::ListFiles => exec_list_files(args, ctx).map(ToolOutcome::Ok),
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
    // read_text_with_stat returns content + ns mtime in one stat
    // (no second-syscall race), so we can echo `mtime_ns` to the
    // model and accept it back on `write_file` for an OCC check.
    let (content, stat) = ctx.drive.read_text_with_stat(path)?;
    let original_len = content.len();
    let (content, truncated) = if original_len > READ_FILE_CAP_BYTES {
        // Truncate at a UTF-8 char boundary so we hand the model a
        // valid Rust String. find_char_boundary walks back at most
        // 4 bytes which is cheap.
        let mut cut = READ_FILE_CAP_BYTES;
        while cut > 0 && !content.is_char_boundary(cut) {
            cut -= 1;
        }
        (content[..cut].to_owned(), true)
    } else {
        (content, false)
    };
    let mut out = serde_json::json!({
        "path": path,
        "content": content,
        "size": original_len,
    });
    if let Some(mtime_ns) = stat.mtime_ns {
        out["mtime_ns"] = serde_json::json!(mtime_ns);
    }
    if truncated {
        out["truncated"] = serde_json::Value::Bool(true);
        out["note"] = serde_json::json!(format!(
            "file truncated to {READ_FILE_CAP_BYTES} bytes; full size {original_len}"
        ));
    }
    Ok(out)
}

fn exec_list_files(args: &Json, ctx: &ToolContext) -> Result<Json> {
    let prefix = args.get("prefix").and_then(|v| v.as_str());
    let tree = ctx.drive.list_tree()?;
    // Filter on prefix client-side: chan-drive's list_tree doesn't
    // take one yet, so we filter here. On a 500k drive this still
    // walks the full tree; once a prefix-aware drive op exists, this
    // reduces to a thin wrapper.
    let mut filtered: Vec<_> = match prefix {
        Some(p) if !p.is_empty() => {
            let p = p.trim_end_matches('/');
            let p_with_slash = format!("{p}/");
            tree.into_iter()
                .filter(|e| e.path == p || e.path.starts_with(&p_with_slash))
                .collect()
        }
        _ => tree,
    };
    let total = filtered.len();
    let truncated = total > LIST_FILES_CAP_ENTRIES;
    if truncated {
        filtered.truncate(LIST_FILES_CAP_ENTRIES);
    }
    let entries = serde_json::to_value(&filtered)
        .map_err(|e| LlmError::Tool(format!("serialize tree: {e}")))?;
    let mut out = serde_json::json!({
        "entries": entries,
        "count": filtered.len(),
        "total": total,
    });
    if truncated {
        out["truncated"] = serde_json::Value::Bool(true);
        out["note"] = serde_json::json!(format!(
            "listing capped at {LIST_FILES_CAP_ENTRIES} of {total}; \
             call again with a `prefix` to narrow."
        ));
    }
    Ok(out)
}

fn exec_search_content(args: &Json, ctx: &ToolContext) -> Result<Json> {
    let query = arg_string(args, "query")?;
    let raw_limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(SEARCH_CONTENT_DEFAULT_LIMIT as u64);
    // Clamp to u32 max via the hard cap; saturating cast keeps us
    // safe from a u64::MAX -> truncation surprise.
    let limit = raw_limit.min(SEARCH_CONTENT_MAX_LIMIT as u64) as u32;
    let res = ctx.drive.search(
        query,
        &chan_drive::SearchOpts {
            limit,
            ..Default::default()
        },
    )?;
    serde_json::to_value(&res).map_err(|e| LlmError::Tool(format!("serialize hits: {e}")))
}

fn exec_write_file(args: &Json, ctx: &ToolContext) -> Result<ToolOutcome> {
    let path = arg_string(args, "path")?;
    let content = arg_string(args, "content")?;
    // Optional optimistic-concurrency token. The assistant gets
    // mtime_ns back from `read_file`; passing it here makes the
    // write a compare-and-swap against the file's current mtime,
    // which catches the case where the user (or another tool) has
    // edited the file between the assistant's read and its write.
    let expected_mtime_ns = args.get("expected_mtime_ns").and_then(|v| v.as_i64());
    if !ctx.auto_apply_writes {
        return Ok(ToolOutcome::Pending {
            tool: "write_file".into(),
            args: args.clone(),
        });
    }
    if let Some(expected) = expected_mtime_ns {
        ctx.drive
            .write_text_if_unchanged(path, Some(expected), content)?;
    } else {
        // No expected mtime supplied: fall back to a plain write.
        // This is the "the model didn't read first" path and
        // matches the legacy behavior; new flows should always
        // round-trip mtime_ns.
        ctx.drive.write_text(path, content)?;
    }
    Ok(ToolOutcome::Ok(serde_json::json!({
        "path": path,
        "bytes_written": content.len(),
    })))
}

/// JSON-schema descriptor for one tool, in the OpenAI-shaped
/// `{name, description, parameters}` form most backends accept
/// directly (Anthropic / Ollama use it verbatim; Gemini wraps it
/// in its own `functionDeclarations` object). Backends translate
/// from this to their wire format.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Json,
}

/// Return JSON-schema descriptors for the four built-in tools.
/// Backends call this once per request to populate their
/// vendor-specific `tools` field.
pub fn standard_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            name: "read_file",
            description: crate::prompts::READ_FILE_DESC,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "POSIX rel path under the drive root."
                    }
                },
                "required": ["path"],
            }),
        },
        ToolSchema {
            name: "write_file",
            description: crate::prompts::WRITE_FILE_DESC,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                    "expected_mtime_ns": {
                        "type": "integer",
                        "description": "Optional. If set, the write only succeeds when the file's current mtime equals this value (the `mtime_ns` from your prior read_file response). Use this to detect concurrent edits; on conflict the call returns an error and you can re-read."
                    }
                },
                "required": ["path", "content"],
            }),
        },
        ToolSchema {
            name: "list_files",
            description: crate::prompts::LIST_FILES_DESC,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "prefix": {
                        "type": "string",
                        "description": "Optional POSIX rel-path prefix to scope the listing. Empty / omitted lists the whole drive (capped)."
                    }
                }
            }),
        },
        ToolSchema {
            name: "search_content",
            description: crate::prompts::SEARCH_CONTENT_DESC,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": {
                        "type": "integer",
                        "description": "Hard cap on hits returned. Default 20."
                    }
                },
                "required": ["query"],
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_drive::Library;
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
    fn read_file_truncates_large_content() {
        let (_cfg, root, ctx) = fixture();
        let big = "x".repeat(READ_FILE_CAP_BYTES + 1024);
        std::fs::write(root.path().join("big.md"), &big).unwrap();
        let out = execute("read_file", &serde_json::json!({"path": "big.md"}), &ctx).unwrap();
        match out {
            ToolOutcome::Ok(v) => {
                assert_eq!(v["truncated"], serde_json::Value::Bool(true));
                assert_eq!(
                    v["size"].as_u64().unwrap(),
                    (READ_FILE_CAP_BYTES + 1024) as u64
                );
                let content = v["content"].as_str().unwrap();
                assert!(content.len() <= READ_FILE_CAP_BYTES);
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn list_files_filters_by_prefix_and_caps() {
        let (_cfg, root, ctx) = fixture();
        std::fs::create_dir_all(root.path().join("notes")).unwrap();
        std::fs::create_dir_all(root.path().join("recipes")).unwrap();
        std::fs::write(root.path().join("notes/a.md"), "x").unwrap();
        std::fs::write(root.path().join("notes/b.md"), "x").unwrap();
        std::fs::write(root.path().join("recipes/r.md"), "x").unwrap();
        let out = execute("list_files", &serde_json::json!({"prefix": "notes"}), &ctx).unwrap();
        match out {
            ToolOutcome::Ok(v) => {
                let entries = v["entries"].as_array().unwrap();
                let paths: Vec<&str> = entries
                    .iter()
                    .map(|e| e["path"].as_str().unwrap())
                    .collect();
                assert!(paths.contains(&"notes/a.md"));
                assert!(paths.contains(&"notes/b.md"));
                assert!(!paths.iter().any(|p| p.starts_with("recipes/")));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn search_content_clamps_limit() {
        let (_cfg, _root, ctx) = fixture();
        // Doesn't actually run a real search beyond the engine's
        // empty-state response; we just verify the call goes through
        // and the limit clamp doesn't panic.
        let _ = execute(
            "search_content",
            &serde_json::json!({"query": "anything", "limit": 1_000_000}),
            &ctx,
        )
        .unwrap();
    }

    #[test]
    fn read_file_returns_mtime_ns_for_round_trip() {
        let (_cfg, root, ctx) = fixture();
        std::fs::write(root.path().join("a.md"), "hello").unwrap();
        let out = execute("read_file", &serde_json::json!({"path": "a.md"}), &ctx).unwrap();
        match out {
            ToolOutcome::Ok(v) => {
                assert!(
                    v["mtime_ns"].is_i64() || v["mtime_ns"].is_null(),
                    "mtime_ns should be present (i64) or absent on FSes without ns mtime"
                );
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn write_file_with_mismatched_mtime_returns_conflict() {
        let (_cfg, _root, mut ctx) = fixture();
        ctx.auto_apply_writes = true;
        ctx.drive.write_text("a.md", "v1").unwrap();
        // Stale mtime from a parallel-universe earlier write.
        let stale = serde_json::json!({
            "path": "a.md",
            "content": "v2",
            "expected_mtime_ns": 1i64,
        });
        let err = execute("write_file", &stale, &ctx).unwrap_err();
        // Typed passthrough: hosts can branch on the kind without
        // string-matching.
        assert!(matches!(err, LlmError::WriteConflict { .. }));
        assert_eq!(ctx.drive.read_text("a.md").unwrap(), "v1");
    }

    #[test]
    fn write_file_with_matching_mtime_succeeds() {
        let (_cfg, _root, mut ctx) = fixture();
        ctx.auto_apply_writes = true;
        ctx.drive.write_text("a.md", "v1").unwrap();
        let stat = ctx.drive.stat("a.md").unwrap();
        let args = serde_json::json!({
            "path": "a.md",
            "content": "v2",
            "expected_mtime_ns": stat.mtime_ns,
        });
        let out = execute("write_file", &args, &ctx).unwrap();
        assert!(matches!(out, ToolOutcome::Ok(_)));
        assert_eq!(ctx.drive.read_text("a.md").unwrap(), "v2");
    }

    #[test]
    fn write_file_rejects_non_text_via_chan_drive() {
        let (_cfg, _root, mut ctx) = fixture();
        ctx.auto_apply_writes = true;
        let err = execute(
            "write_file",
            &serde_json::json!({"path": "img.png", "content": "x"}),
            &ctx,
        )
        .unwrap_err();
        // chan-drive's editable-text gate fires; the assistant cannot
        // bypass it through the tool sandbox. Typed PathRefused so
        // hosts can render a specific "wrong extension" message.
        assert!(matches!(err, LlmError::PathRefused(_)));
    }
}
