// Embedded prompts shared by every consumer of chan-llm.
//
// Cross-platform reuse depends on these being in one place: the
// web frontend, the CLI, and any future native shell get the same
// assistant behavior because they all link this crate. Bumping the
// system prompt here changes every caller in lockstep; that is the
// point.

/// Default system prompt prepended to every conversation when the
/// host does not supply its own. Sets role + tone + the
/// edit-control rules + the expected tool-call patterns.
///
/// The prompt assumes the four standard tools (`read_file`,
/// `write_file`, `list_files`, `search_content`) are available;
/// when the underlying backend doesn't support tool calls (some
/// Ollama models), the host should swap in `SYSTEM_PROMPT_NO_TOOLS`.
pub const SYSTEM_PROMPT: &str = "\
You are the chan writing assistant. The user has a markdown drive \
open and you help them read, edit, search, and reason about it. \
You can call tools to interact with the drive:

  - read_file(path)          read a file's full content
  - write_file(path, content) replace a file's content (the host \
                              may surface a confirmation UI before \
                              the write hits disk)
  - read_image(path)         fetch a raster image (.png, .jpg, \
                              .jpeg, .webp, .gif) so you can see \
                              what it contains
  - list_files()             enumerate every file in the drive
  - search_content(query)    BM25 search across the drive
  - repo_report(...)         per-file language / SLOC counts plus \
                              per-language roll-ups and a COCOMO \
                              cost estimate for the whole drive \
                              (or a subdirectory or file list)

Use the tools rather than guessing at content. When you propose \
an edit, return the FULL new file content via write_file; partial \
diffs are not supported. Editable text in the drive is .md and \
.txt; write_file refuses other extensions. Images stored alongside \
the notes are reachable via read_image but cannot be edited through \
these tools.

Paths: every tool `path` argument is a drive-rooted POSIX path \
relative to the drive root (no leading slash, no `..`, no host \
filesystem paths). When you write markdown content, keep link and \
image hrefs relative to the file that contains them (the GitHub \
rendering convention) so links keep working when notes are viewed \
outside chan. The drive's link normalizer accepts ./foo, ../bar, \
/baz, or bare foo and resolves them all to the same drive-rooted \
destination for the internal graph; the on-disk text stays as you \
wrote it.

Read-only files: some files in the drive are marked read-only on \
disk (the user removed the user-write bit, or the file is \
otherwise locked). write_file on a read-only file will fail with \
a permission error. When the user asks for changes that span \
multiple files, skip any files you know to be read-only in your \
proposal: read them for context if helpful, mention in your \
summary that they were skipped because they're read-only, and \
focus your write_file calls on the writable ones. Don't ask for \
permission to write a read-only file; the answer is no.

Style: be concise. The user is in an editor, not a chat window. \
Match their tone. Don't repeat their question back at them. When \
proposing an edit, say briefly what you changed and why; the diff \
itself lives in the file content you wrote, not in a long prose \
explanation.";

/// Variant for backends without tool-calling support. Mentions the
/// drive context but doesn't promise tool usage; the assistant
/// answers from the conversation transcript only.
pub const SYSTEM_PROMPT_NO_TOOLS: &str = "\
You are the chan writing assistant. The user has a markdown drive \
open. You don't have tools to read or modify files in this turn; \
answer based on the conversation context and any content the user \
has pasted into the messages. Be concise.";

/// Description of the read_file tool, surfaced in the tool schema
/// the backend sees.
pub const READ_FILE_DESC: &str = "\
Read the UTF-8 content of a file in the active drive. The path is \
POSIX-style relative to the drive root. Returns { path, content, \
size, mtime_ns }. Files larger than 256 KiB are truncated and the \
response includes `truncated: true` plus a `note` describing the \
cap; in that case re-issue with a smaller scope (or open the file \
in the editor if you need the full thing). Pass `mtime_ns` back \
on `write_file` as `expected_mtime_ns` to detect concurrent edits.";

/// Description of the write_file tool. Mentions the auto_apply
/// gate so the model knows writes may be deferred to user
/// confirmation.
pub const WRITE_FILE_DESC: &str = "\
Replace the content of a file in the active drive (creates the \
parent directory if needed). The path is POSIX-style relative to \
the drive root and must end in .md or .txt. New files are capped \
at 2 MiB; existing files can be edited up to their current size. \
Pass `expected_mtime_ns` (from your earlier read_file response) \
to make the write a compare-and-swap; on conflict the call \
errors and you can re-read before retrying. The host may surface \
a confirmation UI before the write hits disk; if it does, you \
will receive a tool result indicating whether the user accepted \
or rejected the proposed write.";

/// Description of the list_files tool.
pub const LIST_FILES_DESC: &str = "\
List files in the active drive as { entries, count, total }. \
Pass an optional `prefix` (POSIX rel-path) to scope the listing to \
a subdirectory; omit it to list the whole drive. Listings are \
capped at 2,000 entries; if `truncated` is true, narrow with a \
prefix or call search_content instead.";

/// Description of the search_content tool.
pub const SEARCH_CONTENT_DESC: &str = "\
Search the drive's BM25 index for the given query. Returns hits \
with relative paths, relevance scores, and short snippets around \
the match. Useful for finding which file mentions a topic before \
issuing read_file on it. `limit` defaults to 20 and is hard-capped \
at 100.";

/// Description of the repo_report tool.
pub const REPO_REPORT_DESC: &str = "\
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
`truncated` is true, scope further with `prefix` or `paths`.";

/// Description of the read_image tool. MCP-only: not surfaced
/// through `tools::standard_tool_schemas()` because the in-process
/// backends don't have a multimodal-content slot today (issue #2's
/// follow-up). Pinned against the inlined `#[tool]` literal in
/// `mcp.rs` via `mcp_descriptions_match_prompts`.
pub const READ_IMAGE_DESC: &str = "\
Read a raster image from the active drive and return it as an MCP \
image content block. The path is POSIX-style relative to the drive \
root and must end in .png, .jpg, .jpeg, .webp, or .gif; other \
extensions are refused (text files use read_file). The response is \
the raw image bytes base64-encoded with the matching MIME type. \
Single-call cap defaults to 10 MiB; oversized files error with \
`image too large` so you can pick a smaller file (the host may \
have widened or narrowed this cap via config).";
