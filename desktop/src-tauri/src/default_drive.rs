//! First-launch default drive creation.
//!
//! Fresh desktop installs get a user-owned `Chan` drive under the
//! platform Documents directory, seeded from `docs/manual/`. This
//! path is intentionally narrow: existing registries are migrated only
//! through explicit user choices in the launcher.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use chan_workspace::{FileClass, Library, Registry};
use rust_embed::RustEmbed;
use serde::Serialize;

#[derive(RustEmbed)]
#[folder = "../../docs/manual"]
struct ManualAssets;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedDefaultWorkspace {
    pub root: PathBuf,
    pub seeded_files: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultWorkspaceCandidate {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultWorkspaceStatus {
    pub needs_prompt: bool,
    pub needs_factory_reset: bool,
    pub default_root: Option<String>,
    pub missing_default_root: Option<String>,
    pub suggested_root: String,
    pub drives: Vec<DefaultWorkspaceCandidate>,
}

#[derive(Debug, Clone)]
struct ManualSeedFile {
    rel: String,
    bytes: Cow<'static, [u8]>,
}

/// Create and seed the default drive only when chan metadata has no
/// registered drives and no configured default root.
pub fn ensure_fresh_default_drive() -> Result<Option<CreatedDefaultWorkspace>, String> {
    let config_path = chan_workspace::paths::global_config_path();
    let root = desktop_default_drive_root();
    let files = embedded_manual_files();
    ensure_fresh_default_drive_at(&config_path, &root, files)
}

pub fn status() -> Result<DefaultWorkspaceStatus, String> {
    let config_path = chan_workspace::paths::global_config_path();
    status_at(&config_path, &desktop_default_drive_root())
}

pub fn choose_existing(path: &Path) -> Result<PathBuf, String> {
    let config_path = chan_workspace::paths::global_config_path();
    choose_existing_at(&config_path, path)
}

pub fn create_default_drive() -> Result<CreatedDefaultWorkspace, String> {
    let config_path = chan_workspace::paths::global_config_path();
    let root = desktop_default_drive_root();
    let files = embedded_manual_files();
    create_default_drive_at(&config_path, &root, files)
}

pub fn factory_reset_default_drive() -> Result<CreatedDefaultWorkspace, String> {
    let config_path = chan_workspace::paths::global_config_path();
    let metadata_dir = chan_workspace::paths::config_dir();
    let root = desktop_default_drive_root();
    let files = embedded_manual_files();
    factory_reset_default_drive_at(&config_path, &metadata_dir, &root, files)
}

fn ensure_fresh_default_drive_at(
    config_path: &Path,
    root: &Path,
    files: Vec<ManualSeedFile>,
) -> Result<Option<CreatedDefaultWorkspace>, String> {
    let registry = Registry::load_from(config_path)
        .map_err(|e| format!("loading chan registry {}: {e}", config_path.display()))?;
    if !is_fresh_registry(&registry) {
        return Ok(None);
    }

    create_default_drive_at(config_path, root, files).map(Some)
}

fn create_default_drive_at(
    config_path: &Path,
    root: &Path,
    files: Vec<ManualSeedFile>,
) -> Result<CreatedDefaultWorkspace, String> {
    std::fs::create_dir_all(root)
        .map_err(|e| format!("creating default drive root {}: {e}", root.display()))?;
    let lib = Library::open_at(config_path.to_path_buf())
        .map_err(|e| format!("opening chan library {}: {e}", config_path.display()))?;
    let entry = lib
        .register_workspace(root)
        .map_err(|e| format!("registering default drive {}: {e}", root.display()))?;
    let drive = lib
        .open_workspace(&entry.root_path)
        .map_err(|e| format!("opening default drive {}: {e}", entry.root_path.display()))?;
    let seeded_files = seed_manual_files(&drive, files)?;
    drop(drive);
    lib.set_default_drive_root(Some(entry.root_path.clone()))
        .map_err(|e| {
            format!(
                "persisting default drive root {}: {e}",
                entry.root_path.display()
            )
        })?;

    Ok(CreatedDefaultWorkspace {
        root: entry.root_path,
        seeded_files,
    })
}

fn status_at(config_path: &Path, suggested_root: &Path) -> Result<DefaultWorkspaceStatus, String> {
    let registry = Registry::load_from(config_path)
        .map_err(|e| format!("loading chan registry {}: {e}", config_path.display()))?;
    let missing_default = registry
        .default_workspace_root
        .as_ref()
        .filter(|root| !drive_root_available(root));
    let needs_factory_reset = missing_default
        .map(|root| same_path(root, suggested_root))
        .unwrap_or(false);
    let needs_prompt = if needs_factory_reset {
        false
    } else if registry.default_workspace_root.is_none() {
        !registry.drives.is_empty()
    } else {
        missing_default.is_some()
    };
    let drives: Vec<DefaultWorkspaceCandidate> = registry
        .drives
        .iter()
        .filter(|d| drive_root_available(&d.root_path))
        .map(|d| DefaultWorkspaceCandidate {
            path: d.root_path.display().to_string(),
        })
        .collect();
    Ok(DefaultWorkspaceStatus {
        needs_prompt,
        needs_factory_reset,
        default_root: registry
            .default_workspace_root
            .as_ref()
            .map(|p| p.display().to_string()),
        missing_default_root: missing_default.map(|p| p.display().to_string()),
        suggested_root: suggested_root.display().to_string(),
        drives,
    })
}

fn factory_reset_default_drive_at(
    config_path: &Path,
    metadata_dir: &Path,
    root: &Path,
    files: Vec<ManualSeedFile>,
) -> Result<CreatedDefaultWorkspace, String> {
    let registry = Registry::load_from(config_path)
        .map_err(|e| format!("loading chan registry {}: {e}", config_path.display()))?;
    let Some(default_root) = registry.default_workspace_root else {
        return Err("factory reset requires a configured default drive".into());
    };
    if drive_root_available(&default_root) {
        return Err(format!(
            "default drive {} still exists; factory reset is not needed",
            default_root.display()
        ));
    }
    if !same_path(&default_root, root) {
        return Err(format!(
            "missing default drive {} is not the desktop Chan drive {}",
            default_root.display(),
            root.display()
        ));
    }
    wipe_chan_metadata_contents(metadata_dir)?;
    create_default_drive_at(config_path, root, files)
}

fn choose_existing_at(config_path: &Path, path: &Path) -> Result<PathBuf, String> {
    let registry = Registry::load_from(config_path)
        .map_err(|e| format!("loading chan registry {}: {e}", config_path.display()))?;
    let entry = registry
        .find(path)
        .ok_or_else(|| format!("drive {} is not registered", path.display()))?;
    let root = entry.root_path.clone();
    let lib = Library::open_at(config_path.to_path_buf())
        .map_err(|e| format!("opening chan library {}: {e}", config_path.display()))?;
    lib.set_default_drive_root(Some(root.clone()))
        .map_err(|e| format!("persisting default drive root {}: {e}", root.display()))?;
    Ok(root)
}

fn is_fresh_registry(registry: &Registry) -> bool {
    registry.drives.is_empty() && registry.default_workspace_root.is_none()
}

fn drive_root_available(root: &Path) -> bool {
    std::fs::metadata(root).map(|m| m.is_dir()).unwrap_or(false)
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn wipe_chan_metadata_contents(dir: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(dir) {
        Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
            return Err(format!(
                "refusing to wipe chan metadata path {}; it is not a directory",
                dir.display()
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("creating chan metadata dir {}: {e}", dir.display()))?;
            return Ok(());
        }
        Err(e) => return Err(format!("checking chan metadata dir {}: {e}", dir.display())),
    }

    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("reading chan metadata dir {}: {e}", dir.display()))?
    {
        let entry =
            entry.map_err(|e| format!("reading chan metadata dir {}: {e}", dir.display()))?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|e| format!("checking chan metadata entry {}: {e}", path.display()))?;
        if meta.is_dir() && !meta.file_type().is_symlink() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("removing chan metadata dir {}: {e}", path.display()))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("removing chan metadata file {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn desktop_default_drive_root() -> PathBuf {
    if let Some(docs) = dirs::document_dir() {
        return docs.join("Chan");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join("Documents").join("Chan");
    }
    chan_workspace::paths::default_workspace_root()
}

fn embedded_manual_files() -> Vec<ManualSeedFile> {
    ManualAssets::iter()
        .filter_map(|rel| {
            let asset = ManualAssets::get(rel.as_ref())?;
            Some(ManualSeedFile {
                rel: rel.into_owned(),
                bytes: asset.data,
            })
        })
        .collect()
}

fn seed_manual_files(
    drive: &chan_workspace::Workspace,
    files: Vec<ManualSeedFile>,
) -> Result<usize, String> {
    let mut seeded = 0usize;
    for file in files {
        let rel = validate_manual_rel(&file.rel)?;
        if drive.exists(rel) {
            continue;
        }
        match chan_workspace::classify(rel) {
            FileClass::EditableText | FileClass::Text => match std::str::from_utf8(&file.bytes) {
                Ok(text) => drive
                    .write_text(rel, text)
                    .map_err(|e| format!("seeding manual file {rel}: {e}"))?,
                Err(_) => drive
                    .write_bytes(rel, &file.bytes)
                    .map_err(|e| format!("seeding manual file {rel}: {e}"))?,
            },
            _ => drive
                .write_bytes(rel, &file.bytes)
                .map_err(|e| format!("seeding manual file {rel}: {e}"))?,
        }
        seeded += 1;
    }
    Ok(seeded)
}

fn validate_manual_rel(rel: &str) -> Result<&str, String> {
    if rel.is_empty() || rel.starts_with('/') {
        return Err(format!("invalid embedded manual path {rel:?}"));
    }
    let path = Path::new(rel);
    if !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("invalid embedded manual path {rel:?}"));
    }
    Ok(rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn fresh_registry_requires_no_drives_and_no_default() {
        let mut registry = Registry::default();
        assert!(is_fresh_registry(&registry));
        registry.default_workspace_root = Some(PathBuf::from("/tmp/Chan"));
        assert!(!is_fresh_registry(&registry));
    }

    #[test]
    fn embedded_manual_contains_index() {
        let files = embedded_manual_files();
        assert!(
            files.iter().any(|f| f.rel == "index.md"),
            "manual seed must include index.md",
        );
    }

    #[test]
    fn validate_manual_rel_rejects_absolute_and_parent_paths() {
        assert_eq!(validate_manual_rel("index.md").unwrap(), "index.md");
        assert_eq!(
            validate_manual_rel("search-and-graph.md").unwrap(),
            "search-and-graph.md",
        );
        assert!(validate_manual_rel("/tmp/nope.md").is_err());
        assert!(validate_manual_rel("../nope.md").is_err());
        assert!(validate_manual_rel("docs/../nope.md").is_err());
    }

    #[test]
    fn status_prompts_when_drives_exist_without_default() {
        let cfg = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry::default();
        registry.touch(drive.path());
        registry.save_to(&config_path).unwrap();

        let status = status_at(&config_path, &cfg.path().join("Chan")).unwrap();
        assert!(status.needs_prompt);
        assert!(!status.needs_factory_reset);
        assert_eq!(status.default_root, None);
        assert_eq!(status.drives.len(), 1);
    }

    #[test]
    fn status_requests_factory_reset_when_default_chan_root_is_missing() {
        let cfg = TempDir::new().unwrap();
        let root = cfg.path().join("Documents").join("Chan");
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry {
            default_workspace_root: Some(root.clone()),
            ..Registry::default()
        };
        registry.touch(&root);
        registry.save_to(&config_path).unwrap();

        let status = status_at(&config_path, &root).unwrap();
        assert!(!status.needs_prompt);
        assert!(status.needs_factory_reset);
        assert_eq!(status.default_root, Some(root.display().to_string()));
        assert_eq!(
            status.missing_default_root,
            Some(root.display().to_string()),
        );
        assert!(status.drives.is_empty());
    }

    #[test]
    fn status_prompts_when_non_chan_default_root_is_missing() {
        let cfg = TempDir::new().unwrap();
        let root = cfg.path().join("somewhere-else");
        let suggested = cfg.path().join("Documents").join("Chan");
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry {
            default_workspace_root: Some(root.clone()),
            ..Registry::default()
        };
        registry.touch(&root);
        registry.save_to(&config_path).unwrap();

        let status = status_at(&config_path, &suggested).unwrap();
        assert!(status.needs_prompt);
        assert!(!status.needs_factory_reset);
        assert_eq!(status.default_root, Some(root.display().to_string()));
        assert_eq!(
            status.missing_default_root,
            Some(root.display().to_string()),
        );
        assert!(status.drives.is_empty());
    }

    #[test]
    fn choose_existing_sets_default_root_for_registered_drive() {
        let cfg = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry::default();
        registry.touch(drive.path());
        registry.save_to(&config_path).unwrap();

        let chosen = choose_existing_at(&config_path, drive.path()).unwrap();
        let status = status_at(&config_path, &cfg.path().join("Chan")).unwrap();
        assert_eq!(status.default_root, Some(chosen.display().to_string()));
        assert!(!status.needs_prompt);
    }

    #[test]
    fn create_default_drive_at_keeps_existing_registry_entries() {
        let cfg = TempDir::new().unwrap();
        let existing = TempDir::new().unwrap();
        let root = cfg.path().join("Documents").join("Chan");
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry::default();
        registry.touch(existing.path());
        registry.save_to(&config_path).unwrap();
        let files = vec![ManualSeedFile {
            rel: "index.md".to_string(),
            bytes: Cow::Borrowed(b"# Chan\n"),
        }];

        let created = create_default_drive_at(&config_path, &root, files).unwrap();
        let registry = Registry::load_from(&config_path).unwrap();
        assert_eq!(registry.drives.len(), 2);
        assert_eq!(registry.default_workspace_root, Some(created.root));
    }

    #[test]
    fn factory_reset_refuses_existing_default_drive() {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry {
            default_workspace_root: Some(root.path().to_path_buf()),
            ..Registry::default()
        };
        registry.touch(root.path());
        registry.save_to(&config_path).unwrap();

        let err = factory_reset_default_drive_at(&config_path, cfg.path(), root.path(), Vec::new())
            .unwrap_err();
        assert!(err.contains("still exists"));
    }

    #[test]
    fn factory_reset_refuses_missing_non_chan_default_drive() {
        let cfg = TempDir::new().unwrap();
        let root = cfg.path().join("somewhere-else");
        let suggested = cfg.path().join("Documents").join("Chan");
        let config_path = cfg.path().join("config.toml");
        let mut registry = Registry {
            default_workspace_root: Some(root.clone()),
            ..Registry::default()
        };
        registry.touch(&root);
        registry.save_to(&config_path).unwrap();

        let err = factory_reset_default_drive_at(&config_path, cfg.path(), &suggested, Vec::new())
            .unwrap_err();
        assert!(err.contains("is not the desktop Chan drive"));
    }

    #[test]
    fn wipe_chan_metadata_contents_keeps_root_and_removes_children() {
        let cfg = TempDir::new().unwrap();
        let dir = cfg.path().join(".chan");
        std::fs::create_dir_all(dir.join("drives").join("old")).unwrap();
        std::fs::write(dir.join("config.toml"), "old").unwrap();
        std::fs::write(dir.join("drives").join("old").join("index"), "old").unwrap();

        wipe_chan_metadata_contents(&dir).unwrap();

        assert!(dir.is_dir());
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
    }
}
