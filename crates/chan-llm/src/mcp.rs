//! MCP server: stdio transport that exposes the chan-llm tool sandbox
//! as a Model Context Protocol service.
//!
//! Two consumers wear this:
//!
//!   - The `chan-llm-mcp` binary, which any MCP client (Claude
//!     Desktop, Claude Code, Cursor, Continue, ...) can spawn against
//!     a chan drive to gain chan-drive-sandboxed file access.
//!   - The ClaudeCli backend (v2 follow-up; see issue #1): chan-llm
//!     writes a temporary `--mcp-config` file pointing at
//!     chan-llm-mcp and disallows claude's native Read / Write /
//!     Edit / Glob / Grep tools, so the agent's edits flow through
//!     `tools::execute` and chan-drive's gates fire.
//!
//! v1 ships tools-only. Resources (binary content like images,
//! browse-style discovery) are deferred to issue #2.

use std::sync::Arc;

use chan_drive::Drive;
use rmcp::{
    handler::server::wrapper::Parameters, model::ErrorData, schemars, tool, tool_handler,
    tool_router, transport::stdio, ServerHandler, ServiceExt,
};
use serde::Deserialize;

use crate::error::LlmError;
use crate::tools::{self, ToolContext, ToolOutcome};

// Note: we deliberately do NOT bring `crate::error::Result` into scope.
// The `#[tool_handler]` macro expands to code that uses bare `Result`,
// which would otherwise resolve to chan-llm's `Result<T, LlmError>`
// instead of `std::result::Result<T, ErrorData>` and break the trait
// bound. Use fully qualified `crate::error::Result` where needed.

/// MCP server handle. Owns a `ToolContext` (drive + auto-apply flag);
/// each tool dispatch routes through `tools::execute`, so chan-drive's
/// path sandbox, special-file refusal, and editable-text gate apply
/// to MCP-driven calls the same way they apply to in-process backends.
///
/// Cloning is cheap: `ToolContext` is `Arc<Drive>` + a bool. The
/// rmcp tool macros expand into code that requires `Clone` on the
/// host type.
#[derive(Clone)]
pub struct Server {
    ctx: ToolContext,
}

impl Server {
    pub fn new(drive: Arc<Drive>, auto_apply_writes: bool) -> Self {
        Self {
            ctx: ToolContext::new(drive, auto_apply_writes),
        }
    }

    /// Run the server on stdio. JSON-RPC frames in on stdin, out on
    /// stdout; rmcp's internal tracing goes to stderr. Blocks until
    /// the client disconnects.
    pub async fn serve_stdio(self) -> crate::error::Result<()> {
        let svc = self
            .serve(stdio())
            .await
            .map_err(|e| LlmError::Mcp(format!("serve: {e}")))?;
        svc.waiting()
            .await
            .map_err(|e| LlmError::Mcp(format!("waiting: {e}")))?;
        Ok(())
    }
}

// ---- tool param schemas -----------------------------------------------
//
// Descriptions on the params types are surfaced to the MCP client
// as JSON-schema field descriptions; the tool-level descriptions
// below explain the action itself. We keep both terse; claude
// already gets richer guidance from `prompts::SYSTEM_PROMPT`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFileParams {
    /// POSIX-style relative path under the drive root.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteFileParams {
    /// POSIX-style relative path under the drive root. Must end in
    /// `.md` or `.txt`; chan-drive's editable-text gate refuses other
    /// extensions.
    pub path: String,
    /// Full new file content. Partial diffs are not supported.
    pub content: String,
    /// Optional optimistic-concurrency token. When set, the write
    /// only succeeds if the file's current mtime (in nanoseconds)
    /// equals this value. Use the `mtime_ns` from your prior
    /// `read_file` response. On mismatch the call returns an
    /// error and the caller should re-read.
    #[serde(default)]
    pub expected_mtime_ns: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchContentParams {
    pub query: String,
    /// Hard cap on hits returned. Default 20.
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListFilesParams {
    /// Optional POSIX rel-path prefix to scope the listing to a
    /// subdirectory. Empty / omitted lists the whole drive (capped).
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EmptyParams {}

// ---- tool dispatch ----------------------------------------------------

#[tool_router]
impl Server {
    #[tool(
        description = "Read the full UTF-8 content of a file in the active drive. The path is POSIX-style relative to the drive root."
    )]
    fn read_file(
        &self,
        Parameters(p): Parameters<ReadFileParams>,
    ) -> std::result::Result<String, ErrorData> {
        run_tool("read_file", &serde_json::json!({"path": p.path}), &self.ctx)
    }

    #[tool(
        description = "Replace the content of a file in the active drive (creates the parent directory if needed). Path is POSIX-style relative to the drive root and must end in .md or .txt."
    )]
    fn write_file(
        &self,
        Parameters(p): Parameters<WriteFileParams>,
    ) -> std::result::Result<String, ErrorData> {
        let mut args = serde_json::json!({"path": p.path, "content": p.content});
        if let Some(mtime_ns) = p.expected_mtime_ns {
            args["expected_mtime_ns"] = serde_json::json!(mtime_ns);
        }
        run_tool("write_file", &args, &self.ctx)
    }

    #[tool(
        description = "List files in the active drive as { entries, count, total }. Pass an optional `prefix` (POSIX rel-path) to scope the listing; capped at 2000 entries."
    )]
    fn list_files(
        &self,
        Parameters(p): Parameters<ListFilesParams>,
    ) -> std::result::Result<String, ErrorData> {
        let mut args = serde_json::json!({});
        if let Some(prefix) = p.prefix {
            args["prefix"] = serde_json::Value::String(prefix);
        }
        run_tool("list_files", &args, &self.ctx)
    }

    #[tool(
        description = "Search the drive's BM25 index for the given query. Returns hits with relative paths, relevance scores, and short snippets."
    )]
    fn search_content(
        &self,
        Parameters(p): Parameters<SearchContentParams>,
    ) -> std::result::Result<String, ErrorData> {
        let mut args = serde_json::json!({"query": p.query});
        if let Some(limit) = p.limit {
            args["limit"] = serde_json::json!(limit);
        }
        run_tool("search_content", &args, &self.ctx)
    }
}

#[tool_handler(
    name = "chan",
    instructions = "Tools for reading, writing, listing, and searching files in a chan markdown drive. All file operations are sandboxed under the drive root by chan-drive."
)]
impl ServerHandler for Server {}

/// Adapter: dispatch into `tools::execute`, then translate
/// `ToolOutcome` and `LlmError` into MCP-shaped responses.
///
/// `Pending` becomes an `invalid_params` error so the model itself
/// surfaces the deferral instead of a write going through silently.
/// The standalone binary forces auto-apply on, so this branch fires
/// only in the embedded claude_cli path (issue #1) where a future
/// side channel will let the host approve writes.
///
/// Error messages are run through `mcp_safe_message` so chan-drive's
/// Display strings (which may carry host absolute paths via
/// `SpecialFile.path` / `SymlinkEscape`) don't leak across the
/// MCP boundary. The MCP client may be a third-party process; we
/// surface the variant kind and the model-actionable bits, no host
/// filesystem layout.
fn run_tool(
    name: &str,
    args: &serde_json::Value,
    ctx: &ToolContext,
) -> std::result::Result<String, ErrorData> {
    match tools::execute(name, args, ctx) {
        Ok(ToolOutcome::Ok(v)) => serde_json::to_string(&v)
            .map_err(|e| ErrorData::internal_error(format!("serialize result: {e}"), None)),
        Ok(ToolOutcome::Pending { tool, .. }) => Err(ErrorData::invalid_params(
            format!("{tool} deferred: auto_apply_writes is off; user must approve"),
            None,
        )),
        Err(e) => Err(ErrorData::internal_error(mcp_safe_message(&e), None)),
    }
}

/// Build an MCP-safe error message for `err`. Strips host paths and
/// chan-drive Display details that aren't relevant to the model
/// while preserving the kind and any model-actionable numbers
/// (sizes, mtimes, limits).
fn mcp_safe_message(err: &LlmError) -> String {
    match err {
        LlmError::WriteConflict { current_mtime_ns } => {
            format!("write conflict: file changed on disk (current mtime ns: {current_mtime_ns:?})")
        }
        LlmError::WriteTooLarge { kind, size, limit } => {
            format!("write too large: {size} bytes exceeds {limit} byte cap for {kind}")
        }
        LlmError::ListingTooLarge { observed, limit } => {
            format!("listing too large: {observed} entries (cap {limit})")
        }
        LlmError::PathRefused(_) => {
            // The chan-drive Display may carry an absolute path
            // (SpecialFile.path, SymlinkEscape); flatten to the
            // category. The model knows which call it issued; the
            // category is enough to recover.
            "path refused: not editable, not a regular file, or escapes drive root".to_string()
        }
        LlmError::Core(_) => {
            // chan-drive errors that didn't get a typed passthrough
            // (DriveLocked, DriveAlreadyOpen, Trash*, Search, Graph,
            // Watch, ConfigDecode, Io). Several include paths or
            // host-specific detail; surface the category only.
            "drive operation failed".to_string()
        }
        LlmError::Io(_) => "i/o error".to_string(),
        LlmError::Tool(msg) => format!("tool error: {msg}"),
        LlmError::Http(_) => "http error".to_string(),
        LlmError::BackendError { status, .. } => format!("backend error: {status}"),
        LlmError::Keychain(_) => "keychain error".to_string(),
        LlmError::ConfigDecode(_) => "config decode error".to_string(),
        LlmError::ConfigEncode(_) => "config encode error".to_string(),
        LlmError::Mcp(_) => "mcp error".to_string(),
        LlmError::MissingApiKey(_) => "api key missing".to_string(),
        LlmError::BackendNotConfigured => "no backend configured".to_string(),
        LlmError::NotImplemented(_) => "not implemented".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_drive::Library;
    use tempfile::TempDir;

    fn fixture(auto_apply: bool) -> (TempDir, TempDir, Server) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        let server = Server::new(drive, auto_apply);
        (cfg, drive_dir, server)
    }

    #[test]
    fn read_file_dispatches_to_drive() {
        let (_cfg, root, server) = fixture(true);
        std::fs::write(root.path().join("a.md"), "hello").unwrap();
        let out = server
            .read_file(Parameters(ReadFileParams {
                path: "a.md".into(),
            }))
            .unwrap();
        // tools::execute returns {"path": ..., "content": ...} as JSON;
        // we serialize that to a string for the MCP response.
        assert!(out.contains("hello"), "got: {out}");
    }

    #[test]
    fn write_file_pending_when_auto_apply_off() {
        let (_cfg, _root, server) = fixture(false);
        let err = server
            .write_file(Parameters(WriteFileParams {
                path: "a.md".into(),
                content: "x".into(),
                expected_mtime_ns: None,
            }))
            .unwrap_err();
        assert!(
            err.message.to_lowercase().contains("deferred"),
            "msg={}",
            err.message
        );
    }

    #[test]
    fn write_file_ok_when_auto_apply_on() {
        let (_cfg, root, server) = fixture(true);
        let out = server
            .write_file(Parameters(WriteFileParams {
                path: "a.md".into(),
                content: "hi".into(),
                expected_mtime_ns: None,
            }))
            .unwrap();
        assert!(out.contains("a.md"), "got: {out}");
        assert_eq!(
            std::fs::read_to_string(root.path().join("a.md")).unwrap(),
            "hi"
        );
    }

    #[test]
    fn list_files_returns_tree() {
        let (_cfg, root, server) = fixture(true);
        std::fs::write(root.path().join("a.md"), "x").unwrap();
        let out = server
            .list_files(Parameters(ListFilesParams { prefix: None }))
            .unwrap();
        assert!(out.contains("a.md"), "got: {out}");
    }

    #[test]
    fn write_file_rejects_non_text_via_chan_drive() {
        let (_cfg, _root, server) = fixture(true);
        let err = server
            .write_file(Parameters(WriteFileParams {
                path: "img.png".into(),
                content: "x".into(),
                expected_mtime_ns: None,
            }))
            .unwrap_err();
        // chan-drive's editable-text gate fires; the MCP surface
        // returns the scrubbed kind ("path refused"), not the
        // chan-drive Display string (which would echo "img.png" /
        // host paths). The model gets the category and recovers.
        assert!(
            err.message.to_lowercase().contains("path refused"),
            "msg={}",
            err.message
        );
    }

    #[test]
    fn mcp_error_message_does_not_leak_host_paths() {
        // Trigger a path refusal that, prior to the scrub, would
        // echo "img.png" and any chan-drive Display detail. After
        // the scrub the message is category-only.
        let (_cfg, _root, server) = fixture(true);
        let err = server
            .write_file(Parameters(WriteFileParams {
                path: "img.png".into(),
                content: "x".into(),
                expected_mtime_ns: None,
            }))
            .unwrap_err();
        assert!(
            !err.message.contains("img.png"),
            "leaked path: {}",
            err.message
        );
        assert!(
            !err.message.contains('/'),
            "looks like an absolute path: {}",
            err.message
        );
    }

    #[test]
    fn mcp_error_message_keeps_actionable_numbers() {
        // WriteConflict carries a numeric mtime; that's
        // model-actionable and stays in the scrubbed output.
        let err = mcp_safe_message(&LlmError::WriteConflict {
            current_mtime_ns: Some(123_456_789),
        });
        assert!(
            err.contains("123456789") || err.contains("123_456_789") || err.contains("123456_789"),
            "should keep mtime numeric in: {err}",
        );
        assert!(err.to_lowercase().contains("conflict"));
    }
}
