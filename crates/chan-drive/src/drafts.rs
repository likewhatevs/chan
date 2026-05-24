//! systacean-24: Drafts metadata folder. Parallel to the existing
//! `trash` subsystem: in-progress drafts live in
//! `state_dir/drafts/<uuid>/` so the drive root stays free of
//! uncommitted scratch work.
//!
//! Each draft is a DIRECTORY (e.g. `untitled-1/draft.md`) so the
//! user can paste images and drop config files alongside the
//! markdown without committing them. Rich Prompt history shares
//! the same machinery, distinguished by directory naming
//! convention (`rich-prompt-N/`).
//!
//! The watcher + indexer integration that makes drafts
//! participate in search + graph is implemented elsewhere (see
//! the `indexer` + chan-server graph route hooks); this module
//! is the filesystem primitive layer only.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ChanError, Result};

pub const UNIFIED_DRAFTS_ROOT: &str = "Drafts";

/// True when `rel` is inside the public `Drafts/` namespace.
///
/// Draft files live in chan metadata, outside the user's drive root,
/// but the rest of chan addresses them as `Drafts/<name>/...`.
/// Centralizing the test keeps callers from inventing inconsistent
/// metadata escape hatches.
pub fn is_unified_drafts_path(rel: &str) -> bool {
    strip_unified_prefix(rel).is_some()
}

/// Strip the public `Drafts` prefix. Returns an empty string for
/// the Drafts root itself.
pub fn strip_unified_prefix(rel: &str) -> Option<&str> {
    let trimmed = rel.trim_matches('/');
    if trimmed == UNIFIED_DRAFTS_ROOT {
        return Some("");
    }
    trimmed.strip_prefix("Drafts/")
}

/// Handle to a single draft directory under `drafts_dir`. `name`
/// is the leaf component (e.g. `"untitled-1"`); `abs` is the
/// absolute path on disk so callers can read / write entries
/// inside without re-joining.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRef {
    pub name: String,
    pub abs: PathBuf,
}

/// Ensure the per-drive drafts directory exists. Caller decides
/// when to invoke; `Drive::open` does it eagerly so
/// `create_draft_dir` etc. don't need to re-check.
pub(crate) fn ensure_root(drafts_dir: &Path) -> Result<()> {
    fs::create_dir_all(drafts_dir).map_err(|e| {
        ChanError::Io(format!(
            "failed to create drafts directory {}: {e}",
            drafts_dir.display()
        ))
    })
}

/// Create a draft directory by name. Returns the `DraftRef` for
/// the newly created entry. Errors when the name is empty,
/// contains a path separator (`/` or `\`), traverses (`..`), or
/// already exists under `drafts_dir`.
pub fn create_dir(drafts_dir: &Path, name: &str) -> Result<DraftRef> {
    validate_name(name)?;
    let abs = drafts_dir.join(name);
    if abs.exists() {
        return Err(ChanError::Io(format!(
            "draft `{name}` already exists at {}",
            abs.display()
        )));
    }
    fs::create_dir_all(&abs).map_err(|e| {
        ChanError::Io(format!(
            "failed to create draft directory {}: {e}",
            abs.display()
        ))
    })?;
    Ok(DraftRef {
        name: name.to_string(),
        abs,
    })
}

/// Enumerate drafts under `drafts_dir`. Returns directories only
/// (a stray regular file under drafts is skipped silently so a
/// busted import doesn't take the list path down). Empty when
/// the drafts root doesn't exist yet.
pub fn list(drafts_dir: &Path) -> Result<Vec<DraftRef>> {
    let rd = match fs::read_dir(drafts_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(ChanError::Io(format!(
                "failed to read drafts dir {}: {e}",
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
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        out.push(DraftRef {
            name: name.to_string(),
            abs: path,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Move the draft directory `name` under `drafts_dir` to
/// `target_abs`. Atomic via `fs::rename` when both paths share a
/// filesystem; the caller is responsible for resolving
/// `target_abs` (typically the drive root joined with a relative
/// target path). Errors if the draft is missing, the target
/// already exists, or the parent of `target_abs` doesn't exist.
pub fn promote(drafts_dir: &Path, name: &str, target_abs: &Path) -> Result<()> {
    validate_name(name)?;
    let src = drafts_dir.join(name);
    if !src.is_dir() {
        return Err(ChanError::Io(format!(
            "draft `{name}` not found at {}",
            src.display()
        )));
    }
    if target_abs.exists() {
        return Err(ChanError::Io(format!(
            "target {} already exists",
            target_abs.display()
        )));
    }
    if let Some(parent) = target_abs.parent() {
        if !parent.exists() {
            return Err(ChanError::Io(format!(
                "target parent {} does not exist",
                parent.display()
            )));
        }
    }
    fs::rename(&src, target_abs).map_err(|e| {
        ChanError::Io(format!(
            "failed to promote draft `{name}` to {}: {e}",
            target_abs.display()
        ))
    })?;
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ChanError::Io("draft name cannot be empty".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(ChanError::Io(format!(
            "draft name `{name}` must not contain path separators"
        )));
    }
    if name == "." || name == ".." {
        return Err(ChanError::Io(format!("draft name `{name}` is reserved")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn list_returns_empty_when_root_missing() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        let drafts = list(&root).unwrap();
        assert!(drafts.is_empty());
    }

    #[test]
    fn create_dir_then_list_roundtrips() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        let a = create_dir(&root, "untitled-1").unwrap();
        assert_eq!(a.name, "untitled-1");
        assert!(a.abs.is_dir());
        let b = create_dir(&root, "rich-prompt-3").unwrap();
        assert!(b.abs.is_dir());

        let listed = list(&root).unwrap();
        assert_eq!(listed.len(), 2);
        // list() sorts; rich-prompt-3 before untitled-1.
        assert_eq!(listed[0].name, "rich-prompt-3");
        assert_eq!(listed[1].name, "untitled-1");
    }

    #[test]
    fn create_dir_rejects_traversal_and_separators() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        assert!(create_dir(&root, "").is_err());
        assert!(create_dir(&root, "..").is_err());
        assert!(create_dir(&root, "a/b").is_err());
        assert!(create_dir(&root, "a\\b").is_err());
    }

    #[test]
    fn unified_drafts_path_gate_matches_public_namespace() {
        assert!(is_unified_drafts_path("Drafts"));
        assert!(is_unified_drafts_path("/Drafts/"));
        assert!(is_unified_drafts_path("Drafts/untitled/draft.md"));
        assert_eq!(strip_unified_prefix("Drafts"), Some(""));
        assert_eq!(
            strip_unified_prefix("Drafts/untitled/draft.md"),
            Some("untitled/draft.md")
        );
        assert!(!is_unified_drafts_path(""));
        assert!(!is_unified_drafts_path("Draftsman/note.md"));
        assert!(!is_unified_drafts_path("notes/Drafts/file.md"));
    }

    #[test]
    fn create_dir_rejects_existing() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        create_dir(&root, "untitled-1").unwrap();
        assert!(create_dir(&root, "untitled-1").is_err());
    }

    #[test]
    fn list_skips_non_dir_entries() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        create_dir(&root, "untitled-1").unwrap();
        fs::write(root.join("stray.txt"), b"not a draft").unwrap();
        let listed = list(&root).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "untitled-1");
    }

    #[test]
    fn promote_moves_directory_atomically() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(&drive_root).unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();

        let target = drive_root.join("notes").join("untitled-1");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        promote(&drafts_root, "untitled-1", &target).unwrap();

        assert!(!drafts_root.join("untitled-1").exists());
        assert!(target.is_dir());
        assert!(target.join("draft.md").is_file());
    }

    #[test]
    fn promote_rejects_when_target_exists() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(&drive_root).unwrap();
        create_dir(&drafts_root, "untitled-1").unwrap();
        let target = drive_root.join("untitled-1");
        fs::create_dir_all(&target).unwrap();
        assert!(promote(&drafts_root, "untitled-1", &target).is_err());
        assert!(drafts_root.join("untitled-1").exists());
    }

    #[test]
    fn promote_rejects_missing_draft() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(&drive_root).unwrap();
        let target = drive_root.join("untitled-1");
        assert!(promote(&drafts_root, "ghost", &target).is_err());
    }
}
