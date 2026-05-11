//! Desktop-only sidecar config.
//!
//! The chan registry (`~/.chan/config.toml`) is the source of truth
//! for which drives exist. This file holds only desktop-specific
//! state that has no place in chan proper:
//!
//! - `dev_mode`: open DevTools on every window.
//! - `sidecar`: per-drive UI state (currently just the on-toggle),
//!   keyed by canonical drive path so a `mv` on disk doesn't
//!   silently revive stale state for a different drive.
//!
//! Per-drive serve URLs are intentionally NOT persisted: chan rotates
//! the bearer token on every `chan serve`, so a saved URL would
//! decay to garbage between launches. The URL lives in `AppState`
//! in memory while a serve is running, and the desktop webview
//! reloads it fresh on every On toggle.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriveSidecar {
    /// Port the drive's `chan serve` last bound to, persisted so a
    /// stop-then-start cycle reuses the same port and any browser
    /// tabs the user has open keep their URL valid.
    #[serde(default)]
    pub last_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub dev_mode: bool,
    /// Per-drive UI state, keyed by canonical drive path.
    #[serde(default)]
    pub sidecar: HashMap<String, DriveSidecar>,
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            path: config_path()?,
        })
    }

    pub fn get(&self) -> io::Result<Config> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&mut self, cfg: &Config) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(cfg)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

fn config_path() -> io::Result<PathBuf> {
    let base = if cfg!(target_os = "linux") {
        dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?
            .join("chan-desktop")
    } else {
        dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config dir"))?
            .join("Chan Desktop")
    };
    Ok(base.join("config.json"))
}
