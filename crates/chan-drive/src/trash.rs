// Per-drive Trash. Soft-delete model: `Drive::remove` moves the
// entry here instead of unlinking it. Apps can list, restore, purge,
// or empty. Expired entries are GC'd lazily on `Drive::open` and on
// every `trash_*` call (no background thread; matches the codebase's
// sync-only API rule).
//
// Layout (under `paths.trash`, which is `state_dir/trash/<key>/`):
//
//   trash/<key>/
//     <id>/
//       payload | payload/   the moved file or directory
//       meta.json            written LAST so a half-written entry
//                            (e.g. crash mid-copy on a cross-fs
//                            drive) has no meta and the next sweep
//                            treats it as junk.
//
// `<id>` is `unix_nanos`, with a `-N` suffix retry on the rare
// same-nanosecond collision. Opaque to callers.
//
// Cross-filesystem note: state_dir and the drive root may be on
// different mounts (external disk, network drive). We try
// `fs::rename` first (atomic on the same fs); on failure we fall
// back to copy-then-remove. The fallback writes meta.json BEFORE
// removing the source, so a remove failure leaves a complete trash
// entry plus a partial source (recoverable) instead of data loss.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};
use crate::fs_ops;

/// 30 days. Hardcoded for v1; promote to a `Library` setting later
/// if users want to tune it.
pub const TRASH_RETENTION_SECS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Meta {
    /// POSIX-style relative path from the drive root the entry came
    /// from. Used as the restore destination.
    original_path: String,
    /// Unix seconds at the time of soft-delete.
    deleted_at: i64,
    /// True iff the trashed item is a directory.
    is_dir: bool,
    /// File length, or summed lengths for a directory tree.
    size: u64,
}

/// One entry visible to callers. Owned strings + primitives so the
/// type round-trips cleanly through uniffi later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    pub id: String,
    pub original_path: String,
    pub deleted_at: i64,
    pub is_dir: bool,
    pub size: u64,
}

/// Move `src_abs` (an absolute path inside the drive) into the trash
/// at `trash_dir`, recording `original_rel` as the restore target.
///
/// Same-fs path: one atomic `rename`. Cross-fs path: copy, write
/// meta, then remove the source (so a failure to delete the source
/// leaves a complete trash entry the user can purge or restore from).
pub fn move_into(trash_dir: &Path, src_abs: &Path, original_rel: &str, is_dir: bool) -> Result<()> {
    fs::create_dir_all(trash_dir)?;
    let id = allocate_id(trash_dir)?;
    let entry_dir = trash_dir.join(&id);
    fs::create_dir(&entry_dir)?;
    let payload = entry_dir.join("payload");

    let size = if is_dir {
        dir_size(src_abs).unwrap_or(0)
    } else {
        fs::symlink_metadata(src_abs).map(|m| m.len()).unwrap_or(0)
    };

    if fs::rename(src_abs, &payload).is_ok() {
        // Atomic same-fs move. Source is gone; payload is in place.
        write_meta(&entry_dir, original_rel, size, is_dir)?;
        return Ok(());
    }

    // Cross-fs fallback. Copy first, write meta, then drop source.
    if is_dir {
        copy_dir(src_abs, &payload)?;
    } else {
        fs::copy(src_abs, &payload)?;
    }
    write_meta(&entry_dir, original_rel, size, is_dir)?;
    if is_dir {
        fs::remove_dir_all(src_abs)?;
    } else {
        fs::remove_file(src_abs)?;
    }
    Ok(())
}

pub fn list(trash_dir: &Path) -> Result<Vec<TrashEntry>> {
    let mut out = Vec::new();
    let rd = match fs::read_dir(trash_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in rd.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        let meta_path = entry.path().join("meta.json");
        // Half-written or corrupt entries are silently skipped here;
        // sweep_expired's mtime-blind cleanup catches them later.
        let raw = match fs::read(&meta_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let meta: Meta = match serde_json::from_slice(&raw) {
            Ok(m) => m,
            Err(_) => continue,
        };
        out.push(TrashEntry {
            id,
            original_path: meta.original_path,
            deleted_at: meta.deleted_at,
            is_dir: meta.is_dir,
            size: meta.size,
        });
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.deleted_at));
    Ok(out)
}

pub fn restore(trash_dir: &Path, drive_root: &Path, id: &str) -> Result<()> {
    let entry_dir = trash_dir.join(id);
    let meta_path = entry_dir.join("meta.json");
    let raw = match fs::read(&meta_path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(ChanError::TrashEntryNotFound(id.to_string()));
        }
        Err(e) => return Err(e.into()),
    };
    let meta: Meta = serde_json::from_slice(&raw).map_err(|e| ChanError::TrashCorrupt {
        id: id.to_string(),
        message: format!("meta decode: {e}"),
    })?;

    let payload = entry_dir.join("payload");
    if fs::symlink_metadata(&payload).is_err() {
        return Err(ChanError::TrashCorrupt {
            id: id.to_string(),
            message: "payload missing".into(),
        });
    }

    // The leaf doesn't exist yet (we're restoring), so resolve_safe_strict
    // canonicalizes the deepest existing ancestor. That's enough to catch
    // mid-path symlinks pointing outside the drive.
    let dest = fs_ops::resolve_safe_strict(drive_root, &meta.original_path)?;
    if fs::symlink_metadata(&dest).is_ok() {
        return Err(ChanError::TrashOccupied(meta.original_path.clone()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if fs::rename(&payload, &dest).is_err() {
        // Cross-fs again: copy + drop. Same ordering rationale as
        // move_into but mirrored: payload's removal is best-effort
        // since the trash entry is already considered consumed.
        if meta.is_dir {
            copy_dir(&payload, &dest)?;
            fs::remove_dir_all(&payload)?;
        } else {
            fs::copy(&payload, &dest)?;
            fs::remove_file(&payload)?;
        }
    }

    // Drop the now-empty entry. Best-effort; sweep cleans leftovers.
    let _ = fs::remove_file(&meta_path);
    let _ = fs::remove_dir(&entry_dir);
    Ok(())
}

pub fn purge_one(trash_dir: &Path, id: &str) -> Result<()> {
    let entry_dir = trash_dir.join(id);
    match fs::symlink_metadata(&entry_dir) {
        Ok(_) => {
            fs::remove_dir_all(&entry_dir)?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Err(ChanError::TrashEntryNotFound(id.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

pub fn purge_all(trash_dir: &Path) -> Result<()> {
    let rd = match fs::read_dir(trash_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for entry in rd.flatten() {
        if let Err(e) = fs::remove_dir_all(entry.path()) {
            tracing::warn!(?e, "purge_all: failed to remove entry");
        }
    }
    Ok(())
}

/// Best-effort sweep: drop entries whose `deleted_at + retention_secs`
/// is in the past, plus any entry whose meta is missing or corrupt
/// (those are crash leftovers, not user content).
pub fn sweep_expired(trash_dir: &Path, retention_secs: i64) -> Result<()> {
    let rd = match fs::read_dir(trash_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let cutoff = now_secs() - retention_secs;
    for entry in rd.flatten() {
        let entry_dir = entry.path();
        let meta_path = entry_dir.join("meta.json");
        let expired = match fs::read(&meta_path) {
            Ok(b) => match serde_json::from_slice::<Meta>(&b) {
                Ok(m) => m.deleted_at <= cutoff,
                // Corrupt meta -> treat as junk and reclaim.
                Err(_) => true,
            },
            // Missing meta -> half-written entry. Reclaim.
            Err(_) => true,
        };
        if expired {
            let _ = fs::remove_dir_all(&entry_dir);
        }
    }
    Ok(())
}

fn write_meta(entry_dir: &Path, original_rel: &str, size: u64, is_dir: bool) -> Result<()> {
    let meta = Meta {
        original_path: original_rel.to_string(),
        deleted_at: now_secs(),
        is_dir,
        size,
    };
    let bytes = serde_json::to_vec_pretty(&meta).map_err(|e| ChanError::Io(e.to_string()))?;
    fs_ops::atomic_write(&entry_dir.join("meta.json"), &bytes)
}

fn allocate_id(trash_dir: &Path) -> Result<String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // nanos alone is unique on most clocks; the suffix retry covers
    // the rare same-nanosecond burst (and tests that mock time).
    let mut id = format!("{nanos:032}");
    let mut n = 1u32;
    while trash_dir.join(&id).exists() {
        id = format!("{nanos:032}-{n}");
        n += 1;
    }
    Ok(id)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else if ft.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
        // Symlinks / FIFOs / sockets / devices inside a trashed
        // directory are dropped on the cross-fs path. chan-drive
        // never creates them, and the same-fs rename path preserves
        // them anyway.
    }
    Ok(())
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total = total.saturating_add(dir_size(&entry.path()).unwrap_or(0));
        } else if meta.is_file() {
            total = total.saturating_add(meta.len());
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ts() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let trash = tmp.path().join("trash");
        (tmp, trash)
    }

    #[test]
    fn move_into_then_list_round_trips() {
        let drive = TempDir::new().unwrap();
        let src = drive.path().join("a.md");
        std::fs::write(&src, b"hi").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &src, "a.md", false).unwrap();
        assert!(!src.exists(), "source should be moved");
        let entries = list(&trash).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].original_path, "a.md");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].size, 2);
    }

    #[test]
    fn restore_brings_file_back() {
        let drive = TempDir::new().unwrap();
        let src = drive.path().join("notes/a.md");
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, b"hello").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &src, "notes/a.md", false).unwrap();
        let id = list(&trash).unwrap()[0].id.clone();
        restore(&trash, drive.path(), &id).unwrap();
        assert_eq!(std::fs::read(&src).unwrap(), b"hello");
        assert!(list(&trash).unwrap().is_empty());
    }

    #[test]
    fn restore_refuses_when_dest_exists() {
        let drive = TempDir::new().unwrap();
        let src = drive.path().join("a.md");
        std::fs::write(&src, b"v1").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &src, "a.md", false).unwrap();
        std::fs::write(&src, b"v2").unwrap();
        let id = list(&trash).unwrap()[0].id.clone();
        let err = restore(&trash, drive.path(), &id).unwrap_err();
        assert!(matches!(err, ChanError::TrashOccupied(_)));
        // Trash entry still present; user can purge or pick a new path.
        assert_eq!(list(&trash).unwrap().len(), 1);
        assert_eq!(std::fs::read(&src).unwrap(), b"v2");
    }

    #[test]
    fn move_into_recursive_directory() {
        let drive = TempDir::new().unwrap();
        let dir = drive.path().join("notes");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.md"), b"a").unwrap();
        std::fs::write(dir.join("sub/b.md"), b"bb").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &dir, "notes", true).unwrap();
        assert!(!dir.exists());
        let entries = list(&trash).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].size, 3); // 1 + 2 bytes
        let id = entries[0].id.clone();
        restore(&trash, drive.path(), &id).unwrap();
        assert_eq!(std::fs::read(dir.join("a.md")).unwrap(), b"a");
        assert_eq!(std::fs::read(dir.join("sub/b.md")).unwrap(), b"bb");
    }

    #[test]
    fn purge_one_removes_entry() {
        let drive = TempDir::new().unwrap();
        let src = drive.path().join("a.md");
        std::fs::write(&src, b"x").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &src, "a.md", false).unwrap();
        let id = list(&trash).unwrap()[0].id.clone();
        purge_one(&trash, &id).unwrap();
        assert!(list(&trash).unwrap().is_empty());
        assert!(matches!(
            purge_one(&trash, &id).unwrap_err(),
            ChanError::TrashEntryNotFound(_)
        ));
    }

    #[test]
    fn purge_all_clears_everything() {
        let drive = TempDir::new().unwrap();
        let (_t, trash) = ts();
        for i in 0..3 {
            let src = drive.path().join(format!("f{i}.md"));
            std::fs::write(&src, b"x").unwrap();
            move_into(&trash, &src, &format!("f{i}.md"), false).unwrap();
        }
        assert_eq!(list(&trash).unwrap().len(), 3);
        purge_all(&trash).unwrap();
        assert!(list(&trash).unwrap().is_empty());
    }

    #[test]
    fn sweep_drops_expired_entries() {
        let drive = TempDir::new().unwrap();
        let src = drive.path().join("old.md");
        std::fs::write(&src, b"x").unwrap();
        let (_t, trash) = ts();
        move_into(&trash, &src, "old.md", false).unwrap();
        // Backdate the meta to before the cutoff.
        let id = list(&trash).unwrap()[0].id.clone();
        let meta_path = trash.join(&id).join("meta.json");
        let mut m: Meta = serde_json::from_slice(&fs::read(&meta_path).unwrap()).unwrap();
        m.deleted_at = now_secs() - TRASH_RETENTION_SECS - 1;
        fs::write(&meta_path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        sweep_expired(&trash, TRASH_RETENTION_SECS).unwrap();
        assert!(list(&trash).unwrap().is_empty());
    }

    #[test]
    fn sweep_reclaims_entry_with_missing_meta() {
        let (_t, trash) = ts();
        std::fs::create_dir_all(trash.join("orphan")).unwrap();
        std::fs::write(trash.join("orphan/payload"), b"junk").unwrap();
        sweep_expired(&trash, TRASH_RETENTION_SECS).unwrap();
        assert!(!trash.join("orphan").exists());
    }

    #[test]
    fn restore_unknown_id_errors() {
        let drive = TempDir::new().unwrap();
        let (_t, trash) = ts();
        std::fs::create_dir_all(&trash).unwrap();
        let err = restore(&trash, drive.path(), "missing").unwrap_err();
        assert!(matches!(err, ChanError::TrashEntryNotFound(_)));
    }
}
