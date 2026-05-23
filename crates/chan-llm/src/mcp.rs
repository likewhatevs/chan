//! MCP server: stdio transport that exposes the chan-llm tool sandbox
//! as a Model Context Protocol service.
//!
//! Two consumers use this:
//!
//!   - The `chan-llm-mcp` binary, which any MCP client (Claude
//!     Desktop, Claude Code, Cursor, Continue, ...) can spawn against
//!     a chan drive to gain chan-drive-sandboxed file access.
//!   - The `chan __mcp` subcommand and chan-server MCP bridge, which
//!     host the same tool service for external agents without
//!     reopening the drive.
//!
//! Tools exposed: `read_file`, `write_file`, `list_files`,
//! `search_content`, and `read_image`. The first four route through
//! `tools::execute` (text-only JSON results); `read_image` is the
//! binary read path: it pulls bytes through `Drive::read` (path
//! sandbox, regular-file gate, lstat) and returns base64-encoded
//! image content as an MCP `image` content block, capped at
//! `max_image_bytes` (default 10 MiB, configurable by the server
//! builder or `--max-image-bytes`).

use std::io::Cursor;
use std::sync::Arc;

use base64::engine::Engine as _;
use chan_drive::Drive;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{Content, ErrorData},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler, ServiceExt,
};
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::LlmError;
use crate::tools::{self, ToolContext};

const MCP_FRAME_HEADER_LIMIT: usize = 8192;
const MCP_FRAME_BODY_LIMIT: usize = 16 * 1024 * 1024;

/// Default hard cap on a single `read_image` response, in bytes.
/// 10 MiB covers the typical image attachment ceiling of frontier
/// vision models with margin for base64's ~33% inflation; oversized
/// images surface as an `invalid_params` error so the model can
/// recover instead of bloating a single tool turn. Overridable via
/// `Server::with_max_image_bytes` or `--max-image-bytes` on the
/// standalone binary.
pub const DEFAULT_MCP_IMAGE_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Extension -> MIME table for `read_image`. We keep the set narrow
/// on purpose: anything outside this list is either not renderable by
/// frontier vision models (BMP, TIFF) or carries enough sharp edges
/// (SVG's inline script, AVIF's still-patchy decoder support) that
/// the user is better served opening the file in a real viewer.
/// Comparison is case-insensitive.
const IMAGE_EXTENSIONS: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("gif", "image/gif"),
];

/// Returns the MIME type for `rel` when its extension is in the
/// `read_image` allowlist, else `None`. The comparison strips an
/// optional leading dot and lowercases; `image.PNG` and `image.png`
/// both match. Does not touch the filesystem.
pub fn is_supported_image(rel: &str) -> Option<&'static str> {
    let dot = rel.rfind('.')?;
    let ext = &rel[dot + 1..];
    if ext.is_empty() {
        return None;
    }
    let lower = ext.to_ascii_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .find_map(|(e, m)| if *e == lower { Some(*m) } else { None })
}

// Note: we deliberately do NOT bring `crate::error::Result` into scope.
// The `#[tool_handler]` macro expands to code that uses bare `Result`,
// which would otherwise resolve to chan-llm's `Result<T, LlmError>`
// instead of `std::result::Result<T, ErrorData>` and break the trait
// bound. Use fully qualified `crate::error::Result` where needed.

/// MCP server handle. Owns a `ToolContext` (drive handle); each
/// tool dispatch routes through `tools::execute`, so chan-drive's
/// path sandbox, special-file refusal, and editable-text gate apply
/// to MCP-driven calls.
///
/// Cloning is cheap: `ToolContext` is just an `Arc<Drive>`. The
/// rmcp tool macros expand into code that requires `Clone` on the
/// host type.
#[derive(Clone)]
pub struct Server {
    ctx: ToolContext,
    /// Hard cap on a single `read_image` response, in bytes. Set via
    /// `with_max_image_bytes`; defaults to
    /// `DEFAULT_MCP_IMAGE_MAX_BYTES`. Lives on the server (not the
    /// `ToolContext`) because it's an MCP-surface policy, not a
    /// chan-drive invariant: the same `Drive` can host a server with
    /// a different cap.
    max_image_bytes: u64,
}

impl Server {
    pub fn new(drive: Arc<Drive>) -> Self {
        Self {
            ctx: ToolContext::new(drive),
            max_image_bytes: DEFAULT_MCP_IMAGE_MAX_BYTES,
        }
    }

    /// Override the per-response `read_image` byte cap. Mirrors the
    /// caller that doesn't care about the cap keeps the default;
    /// chan-server and the standalone binary can set it from their
    /// own configuration surfaces.
    pub fn with_max_image_bytes(mut self, n: u64) -> Self {
        self.max_image_bytes = n;
        self
    }

    /// Run the server on stdio. JSON-RPC frames in on stdin, out on
    /// stdout; rmcp's internal tracing goes to stderr. Blocks until
    /// the client disconnects.
    pub async fn serve_stdio(self) -> crate::error::Result<()> {
        let (reader, writer) = stdio();
        self.serve_io(reader, writer).await
    }

    /// Run the server over an arbitrary async read/write pair. Used
    /// by chan-server to host the MCP service over a Unix-domain
    /// socket so each agent session can connect without trying to
    /// reopen the drive (which would deadlock against the parent
    /// process's per-drive flock). Blocks until the client closes
    /// its end of the transport.
    pub async fn serve_io<R, W>(self, reader: R, writer: W) -> crate::error::Result<()>
    where
        R: tokio::io::AsyncRead + Send + Unpin + 'static,
        W: tokio::io::AsyncWrite + Send + Unpin + 'static,
    {
        let (reader, writer, framed_tasks) = adapt_mcp_transport(reader, writer).await?;
        let svc = self
            .serve((reader, writer))
            .await
            .map_err(|e| LlmError::Mcp(format!("serve: {e}")))?;
        svc.waiting()
            .await
            .map_err(|e| LlmError::Mcp(format!("waiting: {e}")))?;
        drop(framed_tasks);
        Ok(())
    }
}

struct FramedTasks {
    inbound: tokio::task::JoinHandle<()>,
    outbound: tokio::task::JoinHandle<()>,
}

impl Drop for FramedTasks {
    fn drop(&mut self) {
        self.inbound.abort();
        self.outbound.abort();
    }
}

async fn adapt_mcp_transport<R, W>(
    mut reader: R,
    writer: W,
) -> crate::error::Result<(
    Box<dyn AsyncRead + Send + Unpin>,
    Box<dyn AsyncWrite + Send + Unpin>,
    Option<FramedTasks>,
)>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let mut first = [0u8; 1];
    let n = reader
        .read(&mut first)
        .await
        .map_err(|e| LlmError::Mcp(format!("read transport prelude: {e}")))?;
    if n == 0 {
        return Ok((Box::new(Cursor::new(Vec::new())), Box::new(writer), None));
    }
    let first = first[0];
    let prefixed = Cursor::new(vec![first]).chain(reader);
    if first != b'C' && first != b'c' {
        return Ok((Box::new(prefixed), Box::new(writer), None));
    }

    let (rmcp_in_reader, rmcp_in_writer) = tokio::io::duplex(64 * 1024);
    let (rmcp_out_reader, rmcp_out_writer) = tokio::io::duplex(64 * 1024);
    let inbound = tokio::spawn(async move {
        let _ = content_length_to_lines(prefixed, rmcp_in_writer).await;
    });
    let outbound = tokio::spawn(async move {
        let _ = lines_to_content_length(rmcp_out_reader, writer).await;
    });
    Ok((
        Box::new(rmcp_in_reader),
        Box::new(rmcp_out_writer),
        Some(FramedTasks { inbound, outbound }),
    ))
}

async fn content_length_to_lines<R, W>(mut reader: R, mut writer: W) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        let header = match read_frame_header(&mut reader).await? {
            Some(header) => header,
            None => return Ok(()),
        };
        let len = parse_content_length(&header)?;
        if len > MCP_FRAME_BODY_LIMIT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "mcp frame body too large",
            ));
        }
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body).await?;
        writer.write_all(&body).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
}

async fn lines_to_content_length<R, W>(mut reader: R, mut writer: W) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        line.clear();
        loop {
            let n = reader.read(&mut byte).await?;
            if n == 0 {
                return Ok(());
            }
            if byte[0] == b'\n' {
                break;
            }
            line.push(byte[0]);
            if line.len() > MCP_FRAME_BODY_LIMIT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "mcp line body too large",
                ));
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        let header = format!("Content-Length: {}\r\n\r\n", line.len());
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(&line).await?;
        writer.flush().await?;
    }
}

async fn read_frame_header<R>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>>
where
    R: AsyncRead + Unpin,
{
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            if header.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "partial mcp frame header",
            ));
        }
        header.push(byte[0]);
        if header.len() > MCP_FRAME_HEADER_LIMIT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "mcp frame header too large",
            ));
        }
        if header.ends_with(b"\r\n\r\n") {
            return Ok(Some(header));
        }
    }
}

fn parse_content_length(header: &[u8]) -> std::io::Result<usize> {
    let header = std::str::from_utf8(header)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "non-utf8 header"))?;
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse::<usize>().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "bad content length")
            });
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "missing content length",
    ))
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadImageParams {
    /// POSIX-style relative path under the drive root. Must end in
    /// one of: .png, .jpg, .jpeg, .webp, .gif.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GraphNeighborsParams {
    /// POSIX-style relative path of the file whose graph adjacency
    /// you want.
    pub path: String,
    /// `out` (this file's outbound edges), `in` (backlinks), or
    /// `both`. Defaults to `both` when omitted.
    #[serde(default)]
    pub direction: Option<String>,
    /// Subset of `link` / `tag` / `mention` to include. Omit for
    /// every kind.
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GraphFilesWithTagParams {
    /// Tag name including the leading `#`, e.g. `#design`.
    pub tag: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RepoReportParams {
    /// Optional POSIX rel-path prefix to scope the snapshot to a
    /// subdirectory.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Optional explicit list of POSIX rel-paths. When non-empty,
    /// takes precedence over `prefix`.
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    /// Include per-file rows in the response (capped at 200).
    /// Default false: only totals, per-language roll-ups, and the
    /// COCOMO summary are returned.
    #[serde(default)]
    pub include_files: Option<bool>,
}

// ---- tool dispatch ----------------------------------------------------

// Tool descriptions duplicate `crate::prompts::*_DESC` as string
// literals here because rmcp-macros 1.6's `#[tool(description =
// ...)]` only parses string literals (darling's FromMeta<String>
// rejects const paths). The `mcp_descriptions_match_prompts` test
// below pins the two copies together so a drift breaks the build.
#[tool_router]
impl Server {
    #[tool(description = "\
Read the UTF-8 content of a file in the active drive. The path is \
POSIX-style relative to the drive root. Returns { path, content, \
size, mtime_ns }. Files larger than 256 KiB are truncated and the \
response includes `truncated: true` plus a `note` describing the \
cap; in that case re-issue with a smaller scope (or open the file \
in the editor if you need the full thing). Pass `mtime_ns` back \
on `write_file` as `expected_mtime_ns` to detect concurrent edits.")]
    fn read_file(
        &self,
        Parameters(p): Parameters<ReadFileParams>,
    ) -> std::result::Result<String, ErrorData> {
        run_tool("read_file", &serde_json::json!({"path": p.path}), &self.ctx)
    }

    #[tool(description = "\
Replace the content of a file in the active drive (creates the \
parent directory if needed). The path is POSIX-style relative to \
the drive root and must end in .md or .txt. New files are capped \
at 2 MiB; existing files can be edited up to their current size. \
Pass `expected_mtime_ns` (from your earlier read_file response) \
to make the write a compare-and-swap; on conflict the call \
errors and you can re-read before retrying. Writes apply \
immediately. When a request touches multiple files or feels \
destructive, call AskUserQuestion first with a numbered plan and \
wait for the user's answer before any write_file call.")]
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

    #[tool(description = "\
List files in the active drive as { entries, count, total }. \
Pass an optional `prefix` (POSIX rel-path) to scope the listing to \
a subdirectory; omit it to list the whole drive. Listings are \
capped at 2,000 entries; if `truncated` is true, narrow with a \
prefix or call search_content instead.")]
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

    #[tool(description = "\
Search the drive's BM25 index for the given query. Returns hits \
with relative paths, relevance scores, and short snippets around \
the match. Useful for finding which file mentions a topic before \
issuing read_file on it. `limit` defaults to 20 and is hard-capped \
at 100.")]
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

    #[tool(description = "\
Read a raster image from the active drive and return it as an MCP \
image content block. The path is POSIX-style relative to the drive \
root and must end in .png, .jpg, .jpeg, .webp, or .gif; other \
extensions are refused (text files use read_file). The response is \
the raw image bytes base64-encoded with the matching MIME type. \
Single-call cap defaults to 10 MiB; oversized files error with \
`image too large` so you can pick a smaller file (the host may \
have widened or narrowed this cap via config).")]
    fn read_image(
        &self,
        Parameters(p): Parameters<ReadImageParams>,
    ) -> std::result::Result<Content, ErrorData> {
        let mime = is_supported_image(&p.path).ok_or_else(|| {
            // Mirror the "path refused" wording the other handlers
            // use after the chan-drive scrub: the model recovers by
            // listing the drive and picking a supported extension.
            ErrorData::invalid_params(
                "path refused: unsupported image extension (expected png/jpg/jpeg/webp/gif)"
                    .to_string(),
                None,
            )
        })?;
        // Drive::read handles the path sandbox, regular-file gate,
        // and lstat refusal; it does NOT apply is_editable_text
        // (that's the text-only gate). Image extensions are the
        // sole policy we add on top.
        let bytes =
            self.ctx.drive.read(&p.path).map_err(|e| {
                ErrorData::internal_error(mcp_safe_message(&LlmError::from(e)), None)
            })?;
        // Cap before base64 so we don't pay the allocation on
        // oversized files. Base64 inflates by ~4/3, so the
        // post-encode payload is at most `max_image_bytes * 4 / 3`.
        if (bytes.len() as u64) > self.max_image_bytes {
            return Err(ErrorData::invalid_params(
                format!(
                    "image too large: {} bytes exceeds {} byte cap",
                    bytes.len(),
                    self.max_image_bytes
                ),
                None,
            ));
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(Content::image(b64, mime))
    }

    #[tool(description = "\
Read the drive's link graph for a single file. Returns `out` (this \
file's outbound edges: wiki/markdown `[[links]]`, `#tags`, and \
`@@mentions`) and `in` (backlinks: every other file that points at \
this one). Use it for backlink-aware questions ('what links here?'), \
to discover a tag's neighbourhood without reading every file, or to \
plan an edit that should also touch the files that reference this \
one. Optional `direction` (`out` / `in` / `both`, default `both`) \
and `kinds` (subset of `link`/`tag`/`mention`) narrow the response.")]
    fn graph_neighbors(
        &self,
        Parameters(p): Parameters<GraphNeighborsParams>,
    ) -> std::result::Result<String, ErrorData> {
        let mut args = serde_json::json!({"path": p.path});
        if let Some(dir) = p.direction {
            args["direction"] = serde_json::Value::String(dir);
        }
        if let Some(kinds) = p.kinds {
            args["kinds"] = serde_json::json!(kinds);
        }
        run_tool("graph_neighbors", &args, &self.ctx)
    }

    #[tool(description = "\
List every `#tag` known to the drive's graph index with the number \
of files that carry it. No arguments. Use it when the user asks \
about tag usage, before a rename / merge, or to discover the actual \
taxonomy instead of guessing. Pair with `graph_files_with_tag` to \
expand a tag into its file list.")]
    fn graph_tags(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> std::result::Result<String, ErrorData> {
        run_tool("graph_tags", &serde_json::json!({}), &self.ctx)
    }

    #[tool(description = "\
Return every file that carries the given `#tag`. The argument \
includes the leading `#`. Cheap: the graph index keeps this \
membership as a direct lookup, so it's preferable to scanning every \
file with search_content when the user has a specific tag in mind.")]
    fn graph_files_with_tag(
        &self,
        Parameters(p): Parameters<GraphFilesWithTagParams>,
    ) -> std::result::Result<String, ErrorData> {
        run_tool(
            "graph_files_with_tag",
            &serde_json::json!({"tag": p.tag}),
            &self.ctx,
        )
    }

    #[tool(description = "\
Snapshot the drive's code/content report: per-file language, code \
lines, comments, blanks, a complexity heuristic (keyword count, \
not cyclomatic), plus per-language roll-ups and a Basic COCOMO \
cost estimate. The drive maintains this index incrementally as \
files change, so the call is cheap to repeat. Use it when the user \
asks about repo size, language mix, where the code lives, or to \
scope a refactor. Optional args: `prefix` (POSIX rel-path) to \
limit the snapshot to a subdirectory, or `paths` (array) for an \
explicit file list. When both are present, `paths` wins. \
`include_files` (default false) controls whether the per-file rows \
are returned; leave it off for an overview, set true when you \
need to drill in. The per-file array is capped at 200 entries; if \
`truncated` is true, scope further with `prefix` or `paths`.")]
    fn repo_report(
        &self,
        Parameters(p): Parameters<RepoReportParams>,
    ) -> std::result::Result<String, ErrorData> {
        let mut args = serde_json::json!({});
        if let Some(prefix) = p.prefix {
            args["prefix"] = serde_json::Value::String(prefix);
        }
        if let Some(paths) = p.paths {
            args["paths"] = serde_json::json!(paths);
        }
        if let Some(b) = p.include_files {
            args["include_files"] = serde_json::Value::Bool(b);
        }
        run_tool("repo_report", &args, &self.ctx)
    }
}

#[tool_handler(
    name = "chan",
    instructions = "Tools for reading, writing, listing, and searching files in a chan markdown drive. All file operations are sandboxed under the drive root by chan-drive."
)]
impl ServerHandler for Server {}

/// Adapter: dispatch into `tools::execute`, then translate the
/// result `Json` and any `LlmError` into MCP-shaped responses.
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
        Ok(v) => serde_json::to_string(&v)
            .map_err(|e| ErrorData::internal_error(format!("serialize result: {e}"), None)),
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
        LlmError::Mcp(_) => "mcp error".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_drive::Library;
    use tempfile::TempDir;

    /// Pin the inlined `#[tool(description = ...)]` literals to the
    /// canonical `prompts::*_DESC` constants. rmcp-macros 1.6 won't
    /// accept const paths in the attribute (darling parses
    /// description as Lit::Str), so the strings are duplicated; this
    /// test breaks the build if the two copies drift.
    #[test]
    fn mcp_descriptions_match_prompts() {
        let read = Server::read_file_tool_attr();
        assert_eq!(
            read.description.as_deref(),
            Some(crate::prompts::READ_FILE_DESC)
        );
        let write = Server::write_file_tool_attr();
        assert_eq!(
            write.description.as_deref(),
            Some(crate::prompts::WRITE_FILE_DESC)
        );
        let list = Server::list_files_tool_attr();
        assert_eq!(
            list.description.as_deref(),
            Some(crate::prompts::LIST_FILES_DESC)
        );
        let search = Server::search_content_tool_attr();
        assert_eq!(
            search.description.as_deref(),
            Some(crate::prompts::SEARCH_CONTENT_DESC)
        );
        let image = Server::read_image_tool_attr();
        assert_eq!(
            image.description.as_deref(),
            Some(crate::prompts::READ_IMAGE_DESC)
        );
        let neighbors = Server::graph_neighbors_tool_attr();
        assert_eq!(
            neighbors.description.as_deref(),
            Some(crate::prompts::GRAPH_NEIGHBORS_DESC)
        );
        let tags = Server::graph_tags_tool_attr();
        assert_eq!(
            tags.description.as_deref(),
            Some(crate::prompts::GRAPH_TAGS_DESC)
        );
        let files_with_tag = Server::graph_files_with_tag_tool_attr();
        assert_eq!(
            files_with_tag.description.as_deref(),
            Some(crate::prompts::GRAPH_FILES_WITH_TAG_DESC)
        );
        let report = Server::repo_report_tool_attr();
        assert_eq!(
            report.description.as_deref(),
            Some(crate::prompts::REPO_REPORT_DESC)
        );
    }

    fn fixture() -> (TempDir, TempDir, Server) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), Some("Test".into()))
            .unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        let server = Server::new(drive);
        (cfg, drive_dir, server)
    }

    #[test]
    fn read_file_dispatches_to_drive() {
        let (_cfg, root, server) = fixture();
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
    fn write_file_applies_immediately() {
        let (_cfg, root, server) = fixture();
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
        let (_cfg, root, server) = fixture();
        std::fs::write(root.path().join("a.md"), "x").unwrap();
        let out = server
            .list_files(Parameters(ListFilesParams { prefix: None }))
            .unwrap();
        assert!(out.contains("a.md"), "got: {out}");
    }

    #[test]
    fn write_file_rejects_non_text_via_chan_drive() {
        let (_cfg, _root, server) = fixture();
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
        let (_cfg, _root, server) = fixture();
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
    fn is_supported_image_extension_table() {
        assert_eq!(is_supported_image("a.png"), Some("image/png"));
        assert_eq!(is_supported_image("a.PNG"), Some("image/png"));
        assert_eq!(is_supported_image("a.jpg"), Some("image/jpeg"));
        assert_eq!(is_supported_image("a.jpeg"), Some("image/jpeg"));
        assert_eq!(is_supported_image("a.webp"), Some("image/webp"));
        assert_eq!(is_supported_image("a.gif"), Some("image/gif"));
        assert_eq!(is_supported_image("a.md"), None);
        assert_eq!(is_supported_image("a.bmp"), None);
        assert_eq!(is_supported_image("a.svg"), None);
        assert_eq!(is_supported_image("a"), None);
        assert_eq!(is_supported_image(""), None);
        assert_eq!(is_supported_image("a."), None);
    }

    #[test]
    fn read_image_returns_base64_image_content() {
        let (_cfg, root, server) = fixture();
        // Minimal valid PNG: the 8-byte signature is enough to
        // verify round-trip; we don't care that it's a parseable
        // image, only that read_image reads the bytes verbatim and
        // hands them to base64 with the right MIME.
        let bytes = b"\x89PNG\r\n\x1a\n";
        std::fs::write(root.path().join("a.png"), bytes).unwrap();
        let content = server
            .read_image(Parameters(ReadImageParams {
                path: "a.png".into(),
            }))
            .unwrap();
        let img = content.as_image().expect("expected ImageContent");
        assert_eq!(img.mime_type, "image/png");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&img.data)
            .unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn read_image_refuses_unsupported_extension() {
        let (_cfg, root, server) = fixture();
        std::fs::write(root.path().join("a.bmp"), b"BMP").unwrap();
        let err = server
            .read_image(Parameters(ReadImageParams {
                path: "a.bmp".into(),
            }))
            .unwrap_err();
        assert!(
            err.message
                .to_lowercase()
                .contains("unsupported image extension"),
            "msg={}",
            err.message
        );
    }

    #[test]
    fn read_image_caps_response_size() {
        let (_cfg, root, server_default) = fixture();
        // Override the cap to 4 bytes so a 5-byte file overflows;
        // exercises with_max_image_bytes alongside the cap check.
        let server = server_default.with_max_image_bytes(4);
        std::fs::write(root.path().join("a.png"), b"\x89PNG\r").unwrap();
        let err = server
            .read_image(Parameters(ReadImageParams {
                path: "a.png".into(),
            }))
            .unwrap_err();
        assert!(
            err.message.contains("image too large"),
            "msg={}",
            err.message
        );
        // The numeric size and cap stay in the message: the model
        // needs them to pick a smaller file or to surface the cap
        // to the user.
        assert!(err.message.contains("5"), "msg={}", err.message);
        assert!(err.message.contains("4"), "msg={}", err.message);
    }

    #[test]
    fn read_image_path_sandbox_error_is_scrubbed() {
        let (_cfg, _root, server) = fixture();
        // Path escape: chan-drive's path-resolver refuses this; the
        // error surfaces through mcp_safe_message as the scrubbed
        // category, no host filesystem detail.
        let err = server
            .read_image(Parameters(ReadImageParams {
                path: "../escape.png".into(),
            }))
            .unwrap_err();
        assert!(
            err.message.to_lowercase().contains("path refused"),
            "msg={}",
            err.message
        );
        assert!(
            !err.message.contains("escape.png"),
            "leaked path: {}",
            err.message
        );
    }

    #[test]
    fn server_with_max_image_bytes_overrides_default() {
        let (_cfg, _root, server_default) = fixture();
        let server = server_default.with_max_image_bytes(123);
        assert_eq!(server.max_image_bytes, 123);
    }

    #[test]
    fn server_defaults_to_default_image_cap() {
        let (_cfg, _root, server) = fixture();
        assert_eq!(server.max_image_bytes, DEFAULT_MCP_IMAGE_MAX_BYTES);
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

    #[tokio::test]
    async fn framed_content_length_initialize_roundtrips() {
        let (_cfg, _root, server) = fixture();
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server_io);
        let server_task =
            tokio::spawn(async move { server.serve_io(server_read, server_write).await });

        let (mut client_read, mut client_write) = tokio::io::split(client_io);
        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "probe", "version": "0"}
            }
        })
        .to_string();
        write_content_length_frame(&mut client_write, init.as_bytes())
            .await
            .unwrap();

        let header = read_frame_header(&mut client_read)
            .await
            .unwrap()
            .expect("response header");
        let len = parse_content_length(&header).unwrap();
        let mut body = vec![0u8; len];
        client_read.read_exact(&mut body).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "chan");

        drop(client_write);
        server_task.abort();
    }

    #[tokio::test]
    async fn newline_initialize_still_roundtrips() {
        let (_cfg, _root, server) = fixture();
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server_io);
        let server_task =
            tokio::spawn(async move { server.serve_io(server_read, server_write).await });

        let (mut client_read, mut client_write) = tokio::io::split(client_io);
        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "probe", "version": "0"}
            }
        })
        .to_string();
        client_write.write_all(init.as_bytes()).await.unwrap();
        client_write.write_all(b"\n").await.unwrap();
        client_write.flush().await.unwrap();

        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            client_read.read_exact(&mut byte).await.unwrap();
            if byte[0] == b'\n' {
                break;
            }
            line.push(byte[0]);
        }
        let response: serde_json::Value = serde_json::from_slice(&line).unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "chan");

        drop(client_write);
        server_task.abort();
    }

    async fn write_content_length_frame<W>(writer: &mut W, body: &[u8]) -> std::io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(body).await?;
        writer.flush().await
    }
}
