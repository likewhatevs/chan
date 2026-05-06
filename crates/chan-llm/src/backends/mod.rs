// Backend dispatch.
//
// Three backends in scope today:
//   - Anthropic (Claude)
//   - Gemini (Google)
//   - Ollama (local server)
//
// Each provides a `Backend` impl that owns its HTTP client config
// (auth header style, base URL, model defaults) and translates
// chan-llm's internal `LlmRequest` / streaming events into the
// backend's wire format. Backends DO NOT touch the filesystem or
// chan-core directly; the tool sandbox sits between them and disk.
//
// All three are stubs at this initial commit. The real ports follow
// from the old `fiorix/chan/crates/chan-core/src/llm/{claude,gemini,ollama}.rs`
// once the public API contract here stabilizes.

pub mod anthropic;
pub mod gemini;
pub mod ollama;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    Anthropic,
    Gemini,
    Ollama,
}

impl BackendKind {
    pub fn name(self) -> &'static str {
        match self {
            BackendKind::Anthropic => "anthropic",
            BackendKind::Gemini => "gemini",
            BackendKind::Ollama => "ollama",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            // Bumping defaults follows the same policy as the
            // toolchain pin: edit, fix any callers, land in one
            // commit. Don't drift between this constant and what
            // the backend modules expect.
            BackendKind::Anthropic => "claude-opus-4-7",
            BackendKind::Gemini => "gemini-2.5-pro",
            BackendKind::Ollama => "llama3.1",
        }
    }
}
