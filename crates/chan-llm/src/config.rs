// LlmConfig: the cross-platform settings layer for the assistant.
//
// Persisted to `~/.config/chan/llm.toml` (mode 0600 on Unix, since
// the file may be the on-disk fallback for API keys when env vars
// and the OS keychain are unavailable). Apps that don't have a home
// dir (iOS, Android sandboxes) pass an explicit path via `load_from`
// / `save_to`.
//
// Only fields that are genuinely cross-platform live here:
//
//   - which backend is selected
//   - which model per backend
//   - the auto_apply_writes flag (whether the assistant's write
//     proposals hit disk without a per-call confirmation)
//   - per-backend API keys (when stored in the file fallback)
//
// Editor preferences (font, theme, keyboard shortcuts) are NOT here.
// Those differ per platform and live in each app's native store
// (UserDefaults on iOS, SharedPreferences on Android, a TOML at the
// app level on web/CLI).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backends::BackendKind;
use crate::error::{LlmError, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which backend the assistant uses by default. None = no
    /// assistant configured yet (the UI should show a "pick a
    /// backend" prompt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendKind>,
    /// Per-backend model override. Falls back to the backend's
    /// default model when unset.
    #[serde(default, skip_serializing_if = "Models::is_empty")]
    pub models: Models,
    /// When true, the assistant's `write_file` tool calls go to disk
    /// without a per-call confirmation. When false, the consumer
    /// (web frontend, native shell) must surface a confirmation UI
    /// for each write. Default: false. Hard line: never silently
    /// flip to true.
    #[serde(default)]
    pub auto_apply_writes: bool,
    /// Per-backend API keys when stored in the on-disk fallback.
    /// Env vars and the OS keychain take precedence. Empty strings
    /// are treated as unset.
    #[serde(default, skip_serializing_if = "Keys::is_empty")]
    pub keys: Keys,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama: Option<String>,
}

impl Models {
    fn is_empty(&self) -> bool {
        self.anthropic.is_none() && self.gemini.is_none() && self.ollama.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<&str> {
        match kind {
            BackendKind::Anthropic => self.anthropic.as_deref(),
            BackendKind::Gemini => self.gemini.as_deref(),
            BackendKind::Ollama => self.ollama.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keys {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<String>,
}

impl Keys {
    fn is_empty(&self) -> bool {
        self.anthropic.is_none() && self.gemini.is_none()
    }

    pub fn for_backend(&self, kind: BackendKind) -> Option<&str> {
        match kind {
            BackendKind::Anthropic => self.anthropic.as_deref(),
            BackendKind::Gemini => self.gemini.as_deref(),
            // Ollama is keyless (local server).
            BackendKind::Ollama => None,
        }
    }
}

impl LlmConfig {
    pub fn load() -> Result<Self> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)?;
        atomic_write_strict(path, body.as_bytes())?;
        Ok(())
    }
}

/// Default config path: `<config_dir>/chan/llm.toml`. Uses the same
/// `dirs::config_dir` chan-core does, so the layout stays predictable
/// across desktop targets. iOS / Android callers pass an explicit
/// path via `load_from` / `save_to` since their sandbox dir isn't
/// `dirs::config_dir`.
fn default_path() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("chan").join("llm.toml"))
        .unwrap_or_else(|| PathBuf::from("chan-llm.toml"))
}

/// Atomic write + 0600 perms on Unix. The file may hold API keys,
/// so set perms before the rename so there's no readable-by-others
/// window.
fn atomic_write_strict(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| LlmError::Io("path has no parent".into()))?;
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600));
    }
    tmp.persist(path)
        .map_err(|e| LlmError::Io(e.error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig::default();
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn populated_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig {
            backend: Some(BackendKind::Anthropic),
            models: Models {
                anthropic: Some("claude-opus-4-7".into()),
                ..Default::default()
            },
            auto_apply_writes: true,
            keys: Keys {
                anthropic: Some("sk-ant-...".into()),
                ..Default::default()
            },
        };
        cfg.save_to(&p).unwrap();
        let loaded = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("nothing.toml");
        let cfg = LlmConfig::load_from(&p).unwrap();
        assert_eq!(cfg, LlmConfig::default());
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("llm.toml");
        let cfg = LlmConfig::default();
        cfg.save_to(&p).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "got {mode:o}");
    }
}
