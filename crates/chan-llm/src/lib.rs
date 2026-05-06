//! LLM backends and tool execution for chan.
//!
//! Three backends (Anthropic, Gemini, Ollama) and a tool sandbox
//! that lets the assistant call into the active drive via
//! `chan-core` (`read_file`, `write_file`, `list_files`,
//! `search_content`). API-key resolution follows a 3-tier policy:
//! environment variable, OS keychain, then `~/.config/chan/api-keys.toml`
//! (mode 0600). Backends and tools port in follow-up commits.
//!
//! Stubbed at the initial commit so the workspace compiles end to
//! end; substantive code lands per the migration plan in
//! `design.md`.

#![forbid(unsafe_code)]

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("chan-llm is not implemented yet; backends port in follow-up commits")]
    NotImplemented,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
