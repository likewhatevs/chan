//! Read-only mirror of the chan workspace registry.
//!
//! chan persists its registry of known workspaces at `~/.chan/config.toml`
//! (see `chan_workspace::registry`). chan-desktop treats that file as the
//! source of truth for which workspaces exist on this machine. We only
//! parse the subset we need; mutation routes through the embedded
//! host's shared `chan_workspace::Library`, never this mirror.

use std::path::PathBuf;

use serde::Deserialize;

/// One entry in the chan registry. Mirrors the on-disk shape of
/// `chan_workspace::registry::KnownWorkspace`. We deliberately keep this
/// minimal: the desktop only needs the path for local workspaces.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub root_path: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    workspaces: Vec<RegistryEntry>,
}

/// Absolute path to the chan registry file. `~/.chan/config.toml` on every
/// desktop target. Routed through `chan_workspace::paths::global_config_path`
/// (the single config-dir authority) so a `CHAN_HOME` override isolates a smoke
/// instance — byte-identical to the old inlined `~/.chan/config.toml`, including
/// the home-unresolvable fallback (`.chan/config.toml`), when `CHAN_HOME` is unset.
pub fn path() -> PathBuf {
    chan_workspace::paths::global_config_path()
}

/// Read the registry. Missing file is not an error: it means the
/// user has not registered any workspaces yet. A malformed file is an
/// error: we never silently ignore a parse failure since that would
/// hide a corrupt user config.
pub fn read() -> std::io::Result<Vec<RegistryEntry>> {
    let p = path();
    let raw = match std::fs::read_to_string(&p) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let parsed: RegistryFile = toml::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(parsed.workspaces)
}
