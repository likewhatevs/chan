//! Ollama (local server) backend.
//!
//! Stub at this commit; the real port lives in
//! `fiorix/chan/crates/chan-core/src/llm/ollama.rs` and lands in a
//! follow-up. Ollama is keyless (the local server provides its own
//! auth model); the env / keychain / file resolver returns Missing
//! for this backend by design.
