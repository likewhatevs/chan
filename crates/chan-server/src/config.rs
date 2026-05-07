//! chan-server preferences.
//!
//! Persisted at `<config>/chan/server.toml` (sibling of
//! `<config>/chan/llm.toml`). Holds chan-server-specific paths
//! and toggles that aren't user content (those live in the
//! drive) and aren't LLM-shaped (those live in chan-llm).
//!
//! Today: `attachments_dir` and `answers_dir`. Both are
//! drive-relative POSIX paths; the actual file I/O routes
//! through `chan_core::Drive::write_bytes` / `write_text` so
//! the path sandbox + special-file refusal + atomic-write
//! invariants apply.
//!
//! New fields land here when a route surfaces a server-shaped
//! setting (e.g. a future "open-in-browser on launch" toggle).
//! Anything filesystem-shaped on the drive itself stays in
//! chan-core; anything LLM-shaped stays in chan-llm.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Drive-relative directory where /api/attachments uploads
    /// land. Default `"attachments"` (a sibling of the user's
    /// notes). The frontend renders the configured value;
    /// callers can pass a sub-path (`"media/2026"`) and it'll
    /// be sandboxed under the drive root via Drive::write_bytes.
    #[serde(default = "default_attachments_dir")]
    pub attachments_dir: String,
    /// Drive-relative directory where /api/answers writes the
    /// assistant's "save this answer to a note" output. Default
    /// `"answers"`.
    #[serde(default = "default_answers_dir")]
    pub answers_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            attachments_dir: default_attachments_dir(),
            answers_dir: default_answers_dir(),
        }
    }
}

fn default_attachments_dir() -> String {
    "attachments".into()
}

fn default_answers_dir() -> String {
    "answers".into()
}

impl ServerConfig {
    pub fn load() -> Result<Self, Error> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        toml::from_str(&raw).map_err(|e| Error::Config(e.to_string()))
    }

    pub fn save(&self) -> Result<(), Error> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        chan_core::fs_ops::atomic_write(path, body.as_bytes())
            .map_err(|e| Error::Config(e.to_string()))?;
        Ok(())
    }
}

/// Default server config path: `<config>/chan/server.toml`. iOS /
/// Android callers pass an explicit path via `load_from` /
/// `save_to` since their sandbox dir isn't `dirs::config_dir`.
pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("chan").join("server.toml"))
        .unwrap_or_else(|| PathBuf::from("chan-server.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("server.toml");
        let cfg = ServerConfig::default();
        cfg.save_to(&p).unwrap();
        let loaded = ServerConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
        assert_eq!(loaded.attachments_dir, "attachments");
        assert_eq!(loaded.answers_dir, "answers");
    }

    #[test]
    fn populated_round_trips() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("server.toml");
        let cfg = ServerConfig {
            attachments_dir: "media/2026".into(),
            answers_dir: "qa".into(),
        };
        cfg.save_to(&p).unwrap();
        let loaded = ServerConfig::load_from(&p).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let cfg = ServerConfig::load_from(&tmp.path().join("nope.toml")).unwrap();
        assert_eq!(cfg, ServerConfig::default());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("server.toml");
        std::fs::write(&p, "answers_dir = \"qa\"\n").unwrap();
        let cfg = ServerConfig::load_from(&p).unwrap();
        assert_eq!(cfg.attachments_dir, "attachments"); // default applied
        assert_eq!(cfg.answers_dir, "qa");
    }
}
