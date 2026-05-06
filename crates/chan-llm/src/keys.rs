// Three-tier API-key resolution.
//
//   1. Environment variable (per backend: `ANTHROPIC_API_KEY`, etc.)
//   2. OS keychain via the `keyring` crate (macOS Keychain, Windows
//      Credential Manager, Linux Secret Service / kwallet).
//   3. The on-disk file fallback (LlmConfig::keys; mode 0600).
//
// Env beats keychain beats file. Env wins because per-shell overrides
// are a useful debugging knob over SSH and inside CI; the keychain is
// the desktop default; the file is the headless-server fallback for
// boxes without a session bus.
//
// Writes only ever go to the keychain (via `set_*_key`). The file
// fallback is read-only from chan-llm's perspective: the only way
// for a key to land there is the user editing the TOML by hand.
// This keeps us from silently rewriting a user-managed file.

use crate::backends::BackendKind;
use crate::config::LlmConfig;
use crate::error::Result;

const SERVICE: &str = "chan";

/// Where the resolved key came from. Useful for status UIs that
/// want to render "key from keychain" / "key from env" badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Env,
    Keychain,
    File,
    Missing,
}

/// Resolve the API key for a backend, walking env -> keychain -> file.
/// Returns `None` if every tier comes up empty.
pub fn resolve(kind: BackendKind, config: &LlmConfig) -> (Option<String>, KeyStatus) {
    let env_var = match kind {
        BackendKind::Anthropic => "ANTHROPIC_API_KEY",
        BackendKind::Gemini => "GEMINI_API_KEY",
        BackendKind::Ollama => return (None, KeyStatus::Missing), // keyless
    };
    if let Ok(v) = std::env::var(env_var) {
        if !v.is_empty() {
            return (Some(v), KeyStatus::Env);
        }
    }
    if let Ok(v) = keychain_get(kind) {
        if !v.is_empty() {
            return (Some(v), KeyStatus::Keychain);
        }
    }
    if let Some(v) = config.keys.for_backend(kind) {
        if !v.is_empty() {
            return (Some(v.to_owned()), KeyStatus::File);
        }
    }
    (None, KeyStatus::Missing)
}

/// Status-only variant: same resolution, doesn't return the key
/// itself. Used by UI status endpoints that should never echo a
/// secret back to the caller.
pub fn status(kind: BackendKind, config: &LlmConfig) -> KeyStatus {
    resolve(kind, config).1
}

/// Persist a key to the OS keychain. The file fallback is
/// intentionally not a write target: a user-managed TOML is
/// cleaner than chan-llm rewriting it on the user's behalf.
pub fn set(kind: BackendKind, key: &str) -> Result<()> {
    if matches!(kind, BackendKind::Ollama) {
        return Ok(()); // keyless; no-op for symmetry
    }
    let entry = keyring::Entry::new(SERVICE, account(kind))?;
    entry.set_password(key)?;
    Ok(())
}

/// Drop a key from the OS keychain. Doesn't touch the file fallback;
/// callers that want to clear a file-stored key edit the TOML.
pub fn clear(kind: BackendKind) -> Result<()> {
    if matches!(kind, BackendKind::Ollama) {
        return Ok(());
    }
    let entry = keyring::Entry::new(SERVICE, account(kind))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn keychain_get(kind: BackendKind) -> Result<String> {
    let entry = keyring::Entry::new(SERVICE, account(kind))?;
    Ok(entry.get_password()?)
}

fn account(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Anthropic => "anthropic",
        BackendKind::Gemini => "gemini",
        BackendKind::Ollama => "ollama",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Keys;

    #[test]
    fn ollama_is_always_missing() {
        let cfg = LlmConfig::default();
        let (key, st) = resolve(BackendKind::Ollama, &cfg);
        assert!(key.is_none());
        assert_eq!(st, KeyStatus::Missing);
    }

    #[test]
    fn file_fallback_when_env_unset() {
        // Make sure no env var is set for this test. We can't
        // remove env reliably across threads, so this test only
        // asserts the file-fallback path WHEN env happens to be
        // unset; if a developer has ANTHROPIC_API_KEY set in their
        // shell the env tier wins and the test still passes
        // (resolve returns Some, status = Env).
        let cfg = LlmConfig {
            keys: Keys {
                anthropic: Some("from-file".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let (key, st) = resolve(BackendKind::Anthropic, &cfg);
        match st {
            KeyStatus::Env => assert!(key.is_some()), // CI/dev shell with env set
            KeyStatus::Keychain => assert!(key.is_some()), // dev's keychain has it
            KeyStatus::File => assert_eq!(key.as_deref(), Some("from-file")),
            KeyStatus::Missing => panic!("expected file fallback to find the key"),
        }
    }
}
