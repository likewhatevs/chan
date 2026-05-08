//! MCP server: stdio transport that exposes the chan-llm tool sandbox
//! as a Model Context Protocol service.
//!
//! Two consumers wear this:
//!
//!   - The `chan-llm-mcp` binary, which any MCP client (Claude
//!     Desktop, Claude Code, Cursor, Continue, ...) can spawn against
//!     a chan drive to gain chan-core-sandboxed file access.
//!   - The ClaudeCli backend (v2 follow-up; see issue #1): chan-llm
//!     writes a temporary `--mcp-config` file pointing at
//!     chan-llm-mcp and disallows claude's native Read / Write /
//!     Edit / Glob / Grep tools, so the agent's edits flow through
//!     `tools::execute` and chan-core's gates fire.
//!
//! v1 ships tools-only. Resources (binary content like images,
//! browse-style discovery) are deferred to issue #2.

use std::sync::Arc;

use chan_core::Drive;
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
/// each tool dispatch routes through `tools::execute`, so chan-core's
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
// below explain the action itself. We keep both terse — claude
// already gets richer guidance from `prompts::SYSTEM_PROMPT`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFileParams {
    /// POSIX-style relative path under the drive root.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteFileParams {
    /// POSIX-style relative path under the drive root. Must end in
    /// `.md` or `.txt`; chan-core's editable-text gate refuses other
    /// extensions.
    pub path: String,
    /// Full new file content. Partial diffs are not supported.
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchContentParams {
    pub query: String,
    /// Hard cap on hits returned. Default 20.
    #[serde(default)]
    pub limit: Option<u32>,
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
        run_tool(
            "write_file",
            &serde_json::json!({"path": p.path, "content": p.content}),
            &self.ctx,
        )
    }

    #[tool(
        description = "Return the full file tree of the active drive as a list of relative paths plus directory markers and file sizes."
    )]
    fn list_files(&self, _: Parameters<EmptyParams>) -> std::result::Result<String, ErrorData> {
        run_tool("list_files", &serde_json::json!({}), &self.ctx)
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
    instructions = "Tools for reading, writing, listing, and searching files in a chan markdown drive. All file operations are sandboxed under the drive root by chan-core."
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
fn run_tool(
    name: &str,
    args: &serde_json::Value,
    ctx: &ToolContext,
) -> std::result::Result<String, ErrorData> {
    match tools::execute(name, args, ctx) {
        Ok(ToolOutcome::Ok(v)) => serde_json::to_string(&v)
            .map_err(|e| ErrorData::internal_error(format!("serialize result: {e}"), None)),
        Ok(ToolOutcome::Pending { tool, args }) => Err(ErrorData::invalid_params(
            format!("write deferred: {tool} {args}; auto_apply_writes is off"),
            None,
        )),
        Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_core::Library;
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
        let out = server.list_files(Parameters(EmptyParams {})).unwrap();
        assert!(out.contains("a.md"), "got: {out}");
    }

    #[test]
    fn write_file_rejects_non_text_via_chan_core() {
        let (_cfg, _root, server) = fixture(true);
        let err = server
            .write_file(Parameters(WriteFileParams {
                path: "img.png".into(),
                content: "x".into(),
            }))
            .unwrap_err();
        // chan-core's editable-text gate fires; the assistant cannot
        // bypass it through the MCP surface.
        assert!(
            err.message.to_lowercase().contains("png")
                || err.message.to_lowercase().contains("text")
                || err.message.to_lowercase().contains(".md"),
            "msg={}",
            err.message
        );
    }
}
