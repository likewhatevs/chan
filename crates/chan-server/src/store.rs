//! Tiny TOML round-trip helpers shared by the chan-server config files
//! (`server.toml`, `preferences.toml`).
//!
//! Every load returns the type's `Default` if the file is missing.
//! Saves go through `chan_drive::fs_ops::atomic_write`, so the parent
//! directory is created and the rename is fsync'd consistently with
//! the rest of the app.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Error;

pub fn load_toml<T>(path: &Path) -> Result<T, Error>
where
    T: DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let raw = std::fs::read_to_string(path)?;
    toml::from_str(&raw).map_err(|e| Error::Config(e.to_string()))
}

pub fn save_toml<T>(path: &Path, value: &T) -> Result<(), Error>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = toml::to_string_pretty(value).map_err(|e| Error::Config(e.to_string()))?;
    chan_drive::fs_ops::atomic_write(path, body.as_bytes())
        .map_err(|e| Error::Config(e.to_string()))?;
    Ok(())
}
