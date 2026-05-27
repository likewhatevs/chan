//! systacean-30: Team primitive. Teams live inside the
//! Drafts metadata subtree as `team-{name}/` directories carrying
//! `config.toml` (team config: members, host, prefix policy),
//! an `events/` subdirectory (per-team event channels watched by
//! chan-server per `-31`), and a `docs/` subdirectory
//! (generalised process docs per `-a-81`).
//!
//! Naming convention: directories prefixed `team-` distinguish
//! teams from regular drafts (`untitled-N` /
//! `rich-prompt-N`). The prefix is enforced by `create_team` and
//! consumed by `list_teams` to filter the drafts subtree.
//!
//! Parallels the `-24` Drafts foundation: filesystem primitives
//! at the module level + Workspace methods that thread the per-workspace
//! drafts root path through.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};

/// Team directory prefix. Used both to compose new
/// team dir names (`team-{name}`) and to filter the drafts
/// listing in `list_teams`.
pub const TEAM_DIR_PREFIX: &str = "team-";

/// Handle to a team directory. `name` is the bare team
/// name (e.g. `"marketing"`); `abs` is the absolute path on disk
/// (`<drafts_dir>/team-{name}/`). Use `config_path()` for the
/// config.toml location + `events_dir()` / `docs_dir()` for the
/// subdirectories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRef {
    pub name: String,
    pub abs: PathBuf,
}

impl TeamRef {
    pub fn config_path(&self) -> PathBuf {
        self.abs.join("config.toml")
    }

    pub fn events_dir(&self) -> PathBuf {
        self.abs.join("events")
    }

    pub fn docs_dir(&self) -> PathBuf {
        self.abs.join("docs")
    }
}

/// Per-team config persisted to `config.toml` at the team
/// directory root. Schema per the addendum-b spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamConfig {
    pub team_name: String,
    pub host_name: String,
    pub host_handle: String,
    #[serde(default = "default_auto_prefix_at")]
    pub auto_prefix_at: bool,
    pub created_at: String,
    #[serde(default)]
    pub members: Vec<Member>,
}

fn default_auto_prefix_at() -> bool {
    true
}

/// One member entry inside `TeamConfig.members`. `position` is
/// the airplane-style grid coordinate; `None` selects
/// tabs-in-current-Hybrid layout. `env` carries per-tab
/// environment overrides (e.g. `CHAN_TAB_NAME`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub handle: String,
    pub command: String,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub is_lead: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

/// Airplane-style grid coordinate. Row + column are zero-based.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub row: u32,
    pub col: u32,
}

/// Create a new team under `drafts_dir`. Creates the
/// `team-{name}/` directory with `config.toml` and empty
/// `events/` / `docs/`. Errors when `team_name` is invalid
/// (empty, contains a path separator) or the directory already
/// exists.
pub fn create(drafts_dir: &Path, config: &TeamConfig) -> Result<TeamRef> {
    validate_name(&config.team_name)?;
    let dir_name = format!("{TEAM_DIR_PREFIX}{}", config.team_name);
    let abs = drafts_dir.join(&dir_name);
    if abs.exists() {
        return Err(ChanError::Io(format!(
            "team `{name}` already exists at {path}",
            name = config.team_name,
            path = abs.display()
        )));
    }
    fs::create_dir_all(&abs).map_err(|e| {
        ChanError::Io(format!(
            "failed to create team directory {}: {e}",
            abs.display()
        ))
    })?;
    fs::create_dir_all(abs.join("events"))
        .map_err(|e| ChanError::Io(format!("failed to create team events/ dir: {e}")))?;
    fs::create_dir_all(abs.join("docs"))
        .map_err(|e| ChanError::Io(format!("failed to create team docs/ dir: {e}")))?;
    let team_ref = TeamRef {
        name: config.team_name.clone(),
        abs,
    };
    write_config(&team_ref, config)?;
    Ok(team_ref)
}

/// Persist a `TeamConfig` to `team-{name}/config.toml`. Atomic
/// via `fs_ops::atomic_write` so a concurrent reader never sees a
/// partial file.
pub fn write_config(team_ref: &TeamRef, config: &TeamConfig) -> Result<()> {
    let body = toml::to_string_pretty(config)
        .map_err(|e| ChanError::ConfigEncode(format!("serialize team config: {e}")))?;
    crate::fs_ops::atomic_write(&team_ref.config_path(), body.as_bytes())
}

/// Read + parse `config.toml` for the team named `team_name`
/// under `drafts_dir`. Errors when the team directory is missing
/// or the config file is unreadable / malformed.
pub fn load(drafts_dir: &Path, team_name: &str) -> Result<TeamConfig> {
    validate_name(team_name)?;
    let dir_name = format!("{TEAM_DIR_PREFIX}{team_name}");
    let abs = drafts_dir.join(&dir_name);
    if !abs.is_dir() {
        return Err(ChanError::Io(format!(
            "team `{team_name}` not found at {}",
            abs.display()
        )));
    }
    let config_path = abs.join("config.toml");
    let raw = fs::read_to_string(&config_path)
        .map_err(|e| ChanError::Io(format!("read team config {}: {e}", config_path.display())))?;
    toml::from_str(&raw).map_err(|e| ChanError::ConfigDecode {
        path: config_path,
        message: e.to_string(),
    })
}

/// Enumerate teams under `drafts_dir`. Returns only directories
/// whose name starts with `team-`; regular drafts (`untitled-N`)
/// are not included. Sorted by team name.
pub fn list(drafts_dir: &Path) -> Result<Vec<TeamRef>> {
    let rd = match fs::read_dir(drafts_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(ChanError::Io(format!(
                "read drafts dir for teams {}: {e}",
                drafts_dir.display()
            )))
        }
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(team_name) = dir_name.strip_prefix(TEAM_DIR_PREFIX) else {
            continue;
        };
        if team_name.is_empty() {
            continue;
        }
        out.push(TeamRef {
            name: team_name.to_string(),
            abs: path,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// True when a Drafts metadata entry belongs to the team subsystem.
pub fn owns_preflight(name: &str, path: &Path) -> bool {
    let Some(team_name) = name.strip_prefix(TEAM_DIR_PREFIX) else {
        return false;
    };
    if team_name.is_empty() {
        return false;
    }
    fs::symlink_metadata(path)
        .map(|meta| meta.is_dir() && !meta.file_type().is_symlink())
        .unwrap_or(false)
}

/// Verbatim-copy a team under a new name. All files +
/// subdirectories are copied byte-for-byte (per addendum-b
/// clarification #10); the new team's `config.toml` then has its
/// `team_name` field overwritten to `new_name`. Internal paths
/// stay relative so links inside docs/ etc. continue to resolve
/// against the new dir.
pub fn duplicate(drafts_dir: &Path, source_name: &str, new_name: &str) -> Result<TeamRef> {
    validate_name(source_name)?;
    validate_name(new_name)?;
    if source_name == new_name {
        return Err(ChanError::Io(format!(
            "duplicate team: source and new name are identical (`{source_name}`)"
        )));
    }
    let src = drafts_dir.join(format!("{TEAM_DIR_PREFIX}{source_name}"));
    let dst = drafts_dir.join(format!("{TEAM_DIR_PREFIX}{new_name}"));
    if !src.is_dir() {
        return Err(ChanError::Io(format!(
            "duplicate team: source `{source_name}` not found at {}",
            src.display()
        )));
    }
    if dst.exists() {
        return Err(ChanError::Io(format!(
            "duplicate team: new name `{new_name}` already exists at {}",
            dst.display()
        )));
    }
    copy_dir_recursive(&src, &dst)?;
    let team_ref = TeamRef {
        name: new_name.to_string(),
        abs: dst,
    };
    // Rewrite the team_name field in the duplicated config.toml
    // so the new team's identity matches its directory name.
    let mut cfg = load(drafts_dir, new_name)?;
    cfg.team_name = new_name.to_string();
    write_config(&team_ref, &cfg)?;
    Ok(team_ref)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| ChanError::Io(format!("create {}: {e}", dst.display())))?;
    for entry in
        fs::read_dir(src).map_err(|e| ChanError::Io(format!("read_dir {}: {e}", src.display())))?
    {
        let entry = entry
            .map_err(|e| ChanError::Io(format!("read_dir entry under {}: {e}", src.display())))?;
        let ft = entry
            .file_type()
            .map_err(|e| ChanError::Io(format!("file_type {}: {e}", entry.path().display())))?;
        let entry_src = entry.path();
        let entry_dst = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry_src, &entry_dst)?;
        } else if ft.is_file() {
            fs::copy(&entry_src, &entry_dst).map_err(|e| {
                ChanError::Io(format!(
                    "copy {} -> {}: {e}",
                    entry_src.display(),
                    entry_dst.display()
                ))
            })?;
        }
        // Symlinks + other special files are skipped silently to
        // mirror the rest of chan-workspace's special-file refusal.
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ChanError::Io("team name cannot be empty".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(ChanError::Io(format!(
            "team name `{name}` must not contain path separators"
        )));
    }
    if name == "." || name == ".." {
        return Err(ChanError::Io(format!("team name `{name}` is reserved")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_config(name: &str) -> TeamConfig {
        TeamConfig {
            team_name: name.to_string(),
            host_name: "Alex".to_string(),
            host_handle: "@@Alex".to_string(),
            auto_prefix_at: true,
            created_at: "2026-05-22T13:57:00Z".to_string(),
            members: vec![
                Member {
                    handle: "@@Architect".to_string(),
                    command: "claude".to_string(),
                    env: std::collections::BTreeMap::from([(
                        "CHAN_TAB_NAME".to_string(),
                        "@@Architect".to_string(),
                    )]),
                    is_lead: true,
                    position: Some(Position { row: 0, col: 0 }),
                },
                Member {
                    handle: "@@FullStackA".to_string(),
                    command: "claude".to_string(),
                    env: std::collections::BTreeMap::new(),
                    is_lead: false,
                    position: Some(Position { row: 0, col: 1 }),
                },
            ],
        }
    }

    #[test]
    fn create_then_load_roundtrips() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        let team = create(&drafts, &sample_config("marketing")).unwrap();
        assert_eq!(team.name, "marketing");
        assert!(team.abs.is_dir());
        assert!(team.config_path().is_file());
        assert!(team.events_dir().is_dir());
        assert!(team.docs_dir().is_dir());

        let loaded = load(&drafts, "marketing").unwrap();
        assert_eq!(loaded, sample_config("marketing"));
    }

    #[test]
    fn list_filters_to_team_prefix_and_skips_drafts() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        // Throw in some regular drafts that should NOT appear in
        // list_teams.
        crate::drafts::create_dir(&drafts, "untitled-1").unwrap();
        crate::drafts::create_dir(&drafts, "rich-prompt-2").unwrap();
        // Now two teams.
        create(&drafts, &sample_config("alpha")).unwrap();
        create(&drafts, &sample_config("beta")).unwrap();

        let teams = list(&drafts).unwrap();
        let names: Vec<String> = teams.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn duplicate_copies_verbatim_then_rewrites_team_name() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        let src = create(&drafts, &sample_config("alpha")).unwrap();
        // Add a sentinel file inside docs/ to verify verbatim
        // copy semantics.
        fs::write(src.docs_dir().join("README.md"), b"# alpha process\n").unwrap();
        // Add a sentinel inside events/ too.
        fs::write(
            src.events_dir().join("event-1.json"),
            b"{\"sentinel\": true}",
        )
        .unwrap();

        let dup = duplicate(&drafts, "alpha", "alpha-clone").unwrap();
        assert_eq!(dup.name, "alpha-clone");
        assert!(dup.abs.is_dir());

        // Verbatim content: every file copied byte-for-byte EXCEPT
        // config.toml's team_name (rewritten to the new name).
        let dup_readme = fs::read(dup.docs_dir().join("README.md")).unwrap();
        assert_eq!(dup_readme, b"# alpha process\n");
        let dup_event = fs::read(dup.events_dir().join("event-1.json")).unwrap();
        assert_eq!(dup_event, b"{\"sentinel\": true}");

        // Config team_name is updated to the new name.
        let dup_cfg = load(&drafts, "alpha-clone").unwrap();
        assert_eq!(dup_cfg.team_name, "alpha-clone");
        // Other fields preserved verbatim.
        assert_eq!(dup_cfg.host_handle, "@@Alex");
        assert_eq!(dup_cfg.members.len(), 2);
    }

    #[test]
    fn duplicate_rejects_same_name_and_existing_target() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        create(&drafts, &sample_config("alpha")).unwrap();
        assert!(duplicate(&drafts, "alpha", "alpha").is_err());

        create(&drafts, &sample_config("beta")).unwrap();
        // beta already exists -> rejected.
        assert!(duplicate(&drafts, "alpha", "beta").is_err());
    }

    #[test]
    fn create_rejects_invalid_names_and_existing() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        let mut cfg = sample_config("");
        assert!(create(&drafts, &cfg).is_err());
        cfg.team_name = "..".to_string();
        assert!(create(&drafts, &cfg).is_err());
        cfg.team_name = "a/b".to_string();
        assert!(create(&drafts, &cfg).is_err());

        cfg.team_name = "marketing".to_string();
        create(&drafts, &cfg).unwrap();
        // Second create with the same name -> rejected.
        assert!(create(&drafts, &cfg).is_err());
    }

    #[test]
    fn load_rejects_missing_team() {
        let td = TempDir::new().unwrap();
        let drafts = td.path().join("drafts");
        crate::drafts::ensure_root(&drafts).unwrap();
        assert!(load(&drafts, "ghost").is_err());
    }
}
