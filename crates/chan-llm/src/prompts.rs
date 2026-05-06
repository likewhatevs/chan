// Embedded prompts shared by every consumer of chan-llm.
//
// Cross-platform reuse depends on these being in one place: the web
// frontend, the CLI, and any future native shell get the same
// assistant behavior because they all link this crate. Bumping the
// system prompt here changes every caller in lockstep; that is the
// point.
//
// The prompts in this initial commit are placeholders. The real
// prompts (system prompt, tool descriptions, edit-control rules,
// formatting expectations) port from `fiorix/chan/crates/chan-core/src/llm/`
// in a follow-up so the API contract stabilizes first and the prompt
// content can change independently as we iterate on assistant UX.

/// System prompt prepended to every conversation. Sets the
/// assistant's role, the markdown-edit constraints, and the
/// expected tool-call patterns.
pub const SYSTEM_PROMPT: &str = "\
You are the chan writing assistant. You help the user read, edit, \
and search the markdown drive they currently have open. Use the \
provided tools (read_file, write_file, list_files, search_content) \
rather than guessing at content. When proposing an edit, return \
the full new file content via write_file; partial diffs are not \
supported. Respect the editable-text whitelist: the drive only \
holds .md and .txt files for editing.";

/// Description of the read_file tool, surfaced in the tool schema
/// the backend sees.
pub const READ_FILE_DESC: &str = "\
Read the full UTF-8 content of a file in the active drive. The path \
is POSIX-style relative to the drive root.";

/// Description of the write_file tool. Mentions the auto_apply gate
/// so the model knows writes may be deferred to user confirmation.
pub const WRITE_FILE_DESC: &str = "\
Replace the content of a file in the active drive (creates the \
parent directory if needed). The path is POSIX-style relative to \
the drive root and must end in .md or .txt. The host may surface \
a confirmation UI before the write hits disk.";

/// Description of the list_files tool.
pub const LIST_FILES_DESC: &str = "\
Return the full file tree of the active drive as a list of \
relative paths plus directory markers.";

/// Description of the search_content tool.
pub const SEARCH_CONTENT_DESC: &str = "\
Search the drive's BM25 index. Returns hits with relative paths, \
relevance scores, and short snippets.";
