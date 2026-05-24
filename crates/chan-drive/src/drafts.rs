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
use crate::fs_ops;
use crate::trash;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftInspection {
    pub name: String,
    pub file_count: usize,
    pub dir_count: usize,
    pub total_size: u64,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftPromoteMode {
    File,
    DirectoryCreated,
    DirectoryMerged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPromoteReport {
    pub name: String,
    pub target_path: String,
    pub mode: DraftPromoteMode,
}

#[derive(Debug, Clone)]
struct DraftScan {
    inspection: DraftInspection,
    src: PathBuf,
    entries: Vec<DraftEntry>,
}

#[derive(Debug, Clone)]
struct DraftEntry {
    rel: PathBuf,
    is_dir: bool,
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

/// Return the draft name from a public `Drafts/<name>/...` path.
pub fn name_from_unified_path(path: &str) -> Result<String> {
    let Some(stripped) = strip_unified_prefix(path) else {
        return Err(ChanError::PathEscape);
    };
    let mut parts = stripped.split('/').filter(|part| !part.is_empty());
    let Some(name) = parts.next() else {
        return Err(ChanError::PathEmpty);
    };
    validate_name(name)?;
    Ok(name.to_string())
}

/// Inspect a draft directory and classify whether it is still a
/// single-file draft or has workspace attachments.
pub fn inspect(drafts_dir: &Path, name: &str) -> Result<DraftInspection> {
    Ok(scan_draft(drafts_dir, name)?.inspection)
}

/// Move a draft workspace into metadata trash.
pub fn discard(drafts_dir: &Path, draft_trash_dir: &Path, name: &str) -> Result<()> {
    validate_name(name)?;
    let src = drafts_dir.join(name);
    let meta = fs::symlink_metadata(&src).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ChanError::Io(format!("not found: draft `{name}` at {}", src.display()))
        } else {
            ChanError::Io(format!("failed to inspect draft `{name}`: {e}"))
        }
    })?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(broken(name, "draft root is not a directory"));
    }
    trash::move_into(
        draft_trash_dir,
        &src,
        &format!("{UNIFIED_DRAFTS_ROOT}/{name}"),
        true,
    )
}

/// Promote a draft into the drive root with explicit no-clobber
/// semantics.
pub fn promote(
    drafts_dir: &Path,
    drive_root: &Path,
    drive_root_canon: &Path,
    name: &str,
    target_rel: &str,
) -> Result<DraftPromoteReport> {
    let scan = scan_draft(drafts_dir, name)?;
    let target_rel_path = fs_ops::validate_rel(target_rel)?;
    let target_rel_str = posix_path(&target_rel_path);
    let target_abs = fs_ops::resolve_safe_strict_canon(drive_root, drive_root_canon, target_rel)?;

    if !fs_ops::is_editable_text(&target_rel_str) && !scan.inspection.has_attachments {
        return Err(ChanError::NotEditableText(target_rel_str));
    }

    if scan.inspection.has_attachments {
        promote_workspace(scan, &target_abs, &target_rel_str)
    } else {
        promote_single_file(scan, &target_abs, &target_rel_str)
    }
}

fn scan_draft(drafts_dir: &Path, name: &str) -> Result<DraftScan> {
    validate_name(name)?;
    let src = drafts_dir.join(name);
    let meta = fs::symlink_metadata(&src).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ChanError::Io(format!("not found: draft `{name}` at {}", src.display()))
        } else {
            ChanError::Io(format!("failed to inspect draft `{name}`: {e}"))
        }
    })?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(broken(name, "draft root is not a directory"));
    }

    let mut entries = Vec::new();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut total_size = 0u64;
    let mut has_draft_md = false;
    scan_entries(
        name,
        &src,
        Path::new(""),
        &mut entries,
        &mut file_count,
        &mut dir_count,
        &mut total_size,
        &mut has_draft_md,
    )?;
    if !has_draft_md {
        return Err(broken(name, "missing draft.md"));
    }
    let has_attachments = !(file_count == 1
        && dir_count == 0
        && entries
            .iter()
            .any(|entry| !entry.is_dir && entry.rel == Path::new("draft.md")));
    Ok(DraftScan {
        inspection: DraftInspection {
            name: name.to_string(),
            file_count,
            dir_count,
            total_size,
            has_attachments,
        },
        src,
        entries,
    })
}

#[allow(clippy::too_many_arguments)]
fn scan_entries(
    name: &str,
    root: &Path,
    rel_dir: &Path,
    entries: &mut Vec<DraftEntry>,
    file_count: &mut usize,
    dir_count: &mut usize,
    total_size: &mut u64,
    has_draft_md: &mut bool,
) -> Result<()> {
    let dir = root.join(rel_dir);
    let mut read = fs::read_dir(&dir)
        .map_err(|e| broken(name, format!("failed to read {}: {e}", dir.display())))?;
    while let Some(entry) = read
        .next()
        .transpose()
        .map_err(|e| broken(name, format!("failed to read {}: {e}", dir.display())))?
    {
        let entry_name = entry.file_name();
        let rel = rel_dir.join(entry_name);
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| broken(name, format!("failed to inspect {}: {e}", path.display())))?;
        let ft = meta.file_type();
        if ft.is_symlink() {
            return Err(broken(name, format!("refusing symlink {}", rel.display())));
        }
        if ft.is_dir() {
            *dir_count += 1;
            entries.push(DraftEntry {
                rel: rel.clone(),
                is_dir: true,
            });
            scan_entries(
                name,
                root,
                &rel,
                entries,
                file_count,
                dir_count,
                total_size,
                has_draft_md,
            )?;
        } else if ft.is_file() {
            *file_count += 1;
            *total_size = total_size.saturating_add(meta.len());
            if rel == Path::new("draft.md") {
                *has_draft_md = true;
            }
            entries.push(DraftEntry { rel, is_dir: false });
        } else {
            return Err(broken(
                name,
                format!("refusing special file {}", rel.display()),
            ));
        }
    }
    Ok(())
}

fn promote_single_file(
    scan: DraftScan,
    target_abs: &Path,
    target_rel: &str,
) -> Result<DraftPromoteReport> {
    let parent = target_abs.parent().ok_or(ChanError::PathEmpty)?;
    ensure_existing_dir(parent, "target parent")?;
    ensure_absent(target_abs, target_rel)?;
    let src_file = scan.src.join("draft.md");
    copy_file_atomic(&src_file, target_abs)?;
    fs::remove_dir_all(&scan.src).map_err(|e| {
        broken(
            &scan.inspection.name,
            format!("saved to {target_rel} but failed to remove draft workspace: {e}"),
        )
    })?;
    Ok(DraftPromoteReport {
        name: scan.inspection.name,
        target_path: target_rel.to_string(),
        mode: DraftPromoteMode::File,
    })
}

fn promote_workspace(
    scan: DraftScan,
    target_abs: &Path,
    target_rel: &str,
) -> Result<DraftPromoteReport> {
    match fs::symlink_metadata(target_abs) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
            preflight_workspace_merge(&scan, target_abs, target_rel)?;
            copy_workspace_into_existing_dir(scan, target_abs, target_rel)
        }
        Ok(_) => Err(ChanError::PathAlreadyExists(target_rel.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = target_abs.parent().ok_or(ChanError::PathEmpty)?;
            ensure_existing_dir(parent, "target parent")?;
            copy_workspace_to_new_dir(scan, target_abs, target_rel)
        }
        Err(e) => Err(ChanError::Io(format!(
            "failed to inspect target {target_rel}: {e}"
        ))),
    }
}

fn preflight_workspace_merge(scan: &DraftScan, target_abs: &Path, target_rel: &str) -> Result<()> {
    for entry in &scan.entries {
        let dest = target_abs.join(&entry.rel);
        if dest.exists() || fs::symlink_metadata(&dest).is_ok() {
            let rel = format!("{target_rel}/{}", posix_path(&entry.rel));
            return Err(ChanError::PathAlreadyExists(rel));
        }
    }
    Ok(())
}

fn copy_workspace_to_new_dir(
    scan: DraftScan,
    target_abs: &Path,
    target_rel: &str,
) -> Result<DraftPromoteReport> {
    ensure_absent(target_abs, target_rel)?;
    let stage = unique_temp_sibling(target_abs)?;
    if let Err(e) = copy_dir_checked(&scan.src, &stage, &scan.inspection.name) {
        let _ = fs::remove_dir_all(&stage);
        return Err(e);
    }
    fs_ops::sync_tree(&stage)?;
    if let Err(e) = fs::rename(&stage, target_abs) {
        let _ = fs::remove_dir_all(&stage);
        return Err(ChanError::Io(format!(
            "failed to install draft workspace at {target_rel}: {e}"
        )));
    }
    remove_promoted_source(&scan, target_rel)?;
    Ok(DraftPromoteReport {
        name: scan.inspection.name,
        target_path: target_rel.to_string(),
        mode: DraftPromoteMode::DirectoryCreated,
    })
}

fn copy_workspace_into_existing_dir(
    scan: DraftScan,
    target_abs: &Path,
    target_rel: &str,
) -> Result<DraftPromoteReport> {
    let stage = unique_temp_sibling(target_abs)?;
    if let Err(e) = copy_dir_checked(&scan.src, &stage, &scan.inspection.name) {
        let _ = fs::remove_dir_all(&stage);
        return Err(e);
    }
    fs_ops::sync_tree(&stage)?;
    move_children(&stage, target_abs, target_rel)?;
    let _ = fs::remove_dir(&stage);
    remove_promoted_source(&scan, target_rel)?;
    Ok(DraftPromoteReport {
        name: scan.inspection.name,
        target_path: target_rel.to_string(),
        mode: DraftPromoteMode::DirectoryMerged,
    })
}

fn copy_dir_checked(src: &Path, dst: &Path, name: &str) -> Result<()> {
    fs::create_dir(dst)?;
    for entry in fs::read_dir(src)
        .map_err(|e| broken(name, format!("failed to read {}: {e}", src.display())))?
    {
        let entry =
            entry.map_err(|e| broken(name, format!("failed to read {}: {e}", src.display())))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_path).map_err(|e| {
            broken(
                name,
                format!("failed to inspect {}: {e}", src_path.display()),
            )
        })?;
        let ft = meta.file_type();
        if ft.is_symlink() {
            return Err(broken(
                name,
                format!("refusing symlink {}", src_path.display()),
            ));
        }
        if ft.is_dir() {
            copy_dir_checked(&src_path, &dst_path, name)?;
        } else if ft.is_file() {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                ChanError::Io(format!(
                    "failed to copy draft file {} to {}: {e}",
                    src_path.display(),
                    dst_path.display()
                ))
            })?;
            fs_ops::sync_file(&dst_path)?;
        } else {
            return Err(broken(
                name,
                format!("refusing special file {}", src_path.display()),
            ));
        }
    }
    Ok(())
}

fn move_children(stage: &Path, target_abs: &Path, target_rel: &str) -> Result<()> {
    for entry in fs::read_dir(stage)? {
        let entry = entry?;
        let dest = target_abs.join(entry.file_name());
        if dest.exists() || fs::symlink_metadata(&dest).is_ok() {
            return Err(ChanError::PathAlreadyExists(format!(
                "{target_rel}/{}",
                entry.file_name().to_string_lossy()
            )));
        }
        fs::rename(entry.path(), &dest).map_err(|e| {
            ChanError::Io(format!(
                "failed to move draft entry into {}: {e}",
                dest.display()
            ))
        })?;
    }
    fs_ops::sync_tree(target_abs)?;
    Ok(())
}

fn remove_promoted_source(scan: &DraftScan, target_rel: &str) -> Result<()> {
    fs::remove_dir_all(&scan.src).map_err(|e| {
        broken(
            &scan.inspection.name,
            format!("saved to {target_rel} but failed to remove draft workspace: {e}"),
        )
    })
}

fn copy_file_atomic(src: &Path, target: &Path) -> Result<()> {
    let tmp = unique_temp_sibling(target)?;
    if let Err(e) = fs::copy(src, &tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(ChanError::Io(format!(
            "failed to copy draft file {} to {}: {e}",
            src.display(),
            target.display()
        )));
    }
    if let Err(e) = fs_ops::sync_file(&tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(ChanError::Io(format!(
            "failed to install draft file at {}: {e}",
            target.display()
        )));
    }
    Ok(())
}

fn unique_temp_sibling(target: &Path) -> Result<PathBuf> {
    let parent = target.parent().ok_or(ChanError::PathEmpty)?;
    let leaf = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(ChanError::PathEmpty)?;
    for i in 0..1000u32 {
        let candidate = parent.join(format!(".{leaf}.chan-draft-tmp-{}-{i}", std::process::id()));
        if !candidate.exists() && fs::symlink_metadata(&candidate).is_err() {
            return Ok(candidate);
        }
    }
    Err(ChanError::Io(format!(
        "failed to allocate temporary sibling for {}",
        target.display()
    )))
}

fn ensure_existing_dir(path: &Path, label: &str) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ChanError::Io(format!("not found: {label} {}", path.display()))
        } else {
            ChanError::Io(format!("failed to inspect {label} {}: {e}", path.display()))
        }
    })?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        return Ok(());
    }
    Err(ChanError::Io(format!(
        "{label} {} is not a directory",
        path.display()
    )))
}

fn ensure_absent(path: &Path, rel: &str) -> Result<()> {
    if path.exists() || fs::symlink_metadata(path).is_ok() {
        return Err(ChanError::PathAlreadyExists(rel.to_string()));
    }
    Ok(())
}

fn broken(name: &str, message: impl Into<String>) -> ChanError {
    ChanError::DraftBroken {
        name: name.to_string(),
        message: message.into(),
    }
}

fn posix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    fn name_from_unified_path_returns_workspace_name() {
        assert_eq!(
            name_from_unified_path("Drafts/untitled-1/draft.md").unwrap(),
            "untitled-1"
        );
        assert!(name_from_unified_path("Drafts").is_err());
        assert!(name_from_unified_path("notes/draft.md").is_err());
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
    fn inspect_classifies_single_file_and_workspace_drafts() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        let single = create_dir(&root, "untitled-1").unwrap();
        fs::write(single.abs.join("draft.md"), b"# hello\n").unwrap();
        let info = inspect(&root, "untitled-1").unwrap();
        assert_eq!(info.file_count, 1);
        assert!(!info.has_attachments);

        let workspace = create_dir(&root, "untitled-2").unwrap();
        fs::create_dir_all(workspace.abs.join("media")).unwrap();
        fs::write(workspace.abs.join("draft.md"), b"# hello\n").unwrap();
        fs::write(workspace.abs.join("media/pasted.png"), [1, 2, 3]).unwrap();
        let info = inspect(&root, "untitled-2").unwrap();
        assert_eq!(info.file_count, 2);
        assert_eq!(info.dir_count, 1);
        assert!(info.has_attachments);
    }

    #[test]
    fn inspect_marks_missing_draft_md_broken() {
        let td = TempDir::new().unwrap();
        let root = td.path().join("drafts");
        ensure_root(&root).unwrap();
        create_dir(&root, "untitled-1").unwrap();
        assert!(matches!(
            inspect(&root, "untitled-1"),
            Err(ChanError::DraftBroken { .. })
        ));
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
        fs::write(draft.abs.join("image.png"), [1, 2, 3]).unwrap();

        let target = drive_root.join("notes").join("untitled-1");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();
        let report = promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "notes/untitled-1",
        )
        .unwrap();

        assert!(!drafts_root.join("untitled-1").exists());
        assert!(target.is_dir());
        assert!(target.join("draft.md").is_file());
        assert!(target.join("image.png").is_file());
        assert_eq!(report.mode, DraftPromoteMode::DirectoryCreated);
    }

    #[test]
    fn promote_single_file_moves_draft_md_to_target_file() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(drive_root.join("notes")).unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();

        let report = promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "notes/draft.md",
        )
        .unwrap();

        assert_eq!(report.mode, DraftPromoteMode::File);
        assert!(!drafts_root.join("untitled-1").exists());
        assert_eq!(
            fs::read_to_string(drive_root.join("notes/draft.md")).unwrap(),
            "# hello\n"
        );
    }

    #[test]
    fn promote_workspace_merges_into_existing_dir_without_clobber() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(drive_root.join("notes")).unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();
        fs::write(draft.abs.join("pasted.png"), [1, 2, 3]).unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();

        let report = promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "notes",
        )
        .unwrap();

        assert_eq!(report.mode, DraftPromoteMode::DirectoryMerged);
        assert!(!drafts_root.join("untitled-1").exists());
        assert!(drive_root.join("notes/draft.md").is_file());
        assert!(drive_root.join("notes/pasted.png").is_file());
    }

    #[test]
    fn promote_workspace_rejects_nested_collision() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(drive_root.join("notes")).unwrap();
        fs::write(drive_root.join("notes/draft.md"), b"existing").unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();
        fs::write(draft.abs.join("pasted.png"), [1, 2, 3]).unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();

        let err = promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "notes",
        )
        .unwrap_err();

        assert!(matches!(err, ChanError::PathAlreadyExists(_)));
        assert!(drafts_root.join("untitled-1").exists());
    }

    #[test]
    fn promote_rejects_target_escape() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(&drive_root).unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();

        let err = promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "../outside.md",
        )
        .unwrap_err();

        assert!(matches!(err, ChanError::PathEscape));
        assert!(drafts_root.join("untitled-1").exists());
    }

    #[test]
    fn discard_moves_draft_to_metadata_trash() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let trash_root = td.path().join("trash");
        ensure_root(&drafts_root).unwrap();
        let draft = create_dir(&drafts_root, "untitled-1").unwrap();
        fs::write(draft.abs.join("draft.md"), b"# hello\n").unwrap();

        discard(&drafts_root, &trash_root, "untitled-1").unwrap();

        assert!(!drafts_root.join("untitled-1").exists());
        let trashed = trash::list(&trash_root).unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].original_path, "Drafts/untitled-1");
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
        let drive_root_canon = drive_root.canonicalize().unwrap();
        assert!(promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "untitled-1",
            "untitled-1",
        )
        .is_err());
        assert!(drafts_root.join("untitled-1").exists());
    }

    #[test]
    fn promote_rejects_missing_draft() {
        let td = TempDir::new().unwrap();
        let drafts_root = td.path().join("drafts");
        let drive_root = td.path().join("drive");
        ensure_root(&drafts_root).unwrap();
        fs::create_dir_all(&drive_root).unwrap();
        let drive_root_canon = drive_root.canonicalize().unwrap();
        assert!(promote(
            &drafts_root,
            &drive_root,
            &drive_root_canon,
            "ghost",
            "untitled-1",
        )
        .is_err());
    }
}
