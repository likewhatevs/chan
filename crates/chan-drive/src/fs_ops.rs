// Filesystem helpers for the drive:
//   - atomic_write: tmpfile + fsync + rename. Used everywhere that
//     touches a file on behalf of the user. We never want a half-
//     written note.
//   - resolve_safe / resolve_safe_strict: normalize a request path
//     and reject anything that escapes the drive root via `..` or
//     a symlink pointing outside.
//   - ensure_regular_file: lstat-based gate that rejects symlinks,
//     FIFOs, sockets, char/block devices, and directories before
//     we open a path for read or write. Without it, opening a FIFO
//     blocks waiting for a writer; opening /dev/zero never returns;
//     opening through a symlink can escape the drive sandbox.
//   - is_editable_text: extension whitelist gate.
//   - walk_drive / list_tree: recursive listing scoped to the drive,
//     skipping `.git/` and `.chan/` at any depth and dropping non-
//     regular non-dir entries (symlinks, devices, sockets) so the
//     UI tree and the indexer never see them.
//
// Symlink policy: we never traverse a symlink that points outside
// the drive's canonical root. Symlinks that resolve back inside
// the drive are also rejected by default for the read/write API
// (final-component check via lstat) so a user's intentional
// `today.md -> 2026-05-06.md` doesn't silently get clobbered by
// atomic_write, and so reads always see real files. A future
// follower-mode could relax this once we've thought through the
// editor UX.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use crate::error::{ChanError, Result};

/// True for paths inside the drive-internal `.chan/` dir. chan-drive
/// never writes there now (per-drive state lives outside the user's
/// notes tree). The check stays as a defensive filter so a stray
/// `.chan/` from an older install or a third-party tool never
/// surfaces in the file tree or gets indexed.
pub fn is_chan_internal(rel: &str) -> bool {
    rel == ".chan" || rel.starts_with(".chan/")
}

/// True for paths whose extension marks them as plain-text content
/// the editor can safely round-trip through a UTF-8 buffer.
/// Whitelisted by extension to prevent corrupting binary files.
pub fn is_editable_text(rel: &str) -> bool {
    let ext = match rel.rsplit_once('.') {
        Some((_, e)) if !e.is_empty() => e,
        _ => return false,
    };
    matches!(ext.to_ascii_lowercase().as_str(), "md" | "txt")
}

/// Recursive walker rooted at `root` that:
///   - skips `.git/` and `.chan/` at any depth;
///   - never follows symlinks (`walkdir` default; we set it
///     explicitly so a future maintainer cannot flip it without
///     understanding what they're trading away);
///   - drops non-regular non-directory entries (symlinks, FIFOs,
///     sockets, char/block devices) at iteration time so the
///     listing and the indexer only ever see real files and dirs.
///
/// Per-entry errors are logged and skipped.
pub fn walk_drive(root: &Path) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !(e.file_type().is_dir() && (n == ".git" || n == ".chan"))
        })
        .filter_map(|res| match res {
            Ok(e) => Some(e),
            Err(e) => {
                tracing::warn!("walkdir error: {e}");
                None
            }
        })
        .filter(|e| {
            let ft = e.file_type();
            // Keep dirs (we descend into them) and regular files.
            // Drop symlinks (regardless of where they point),
            // devices, sockets, and FIFOs.
            ft.is_dir() || ft.is_file()
        })
}

/// Atomically write `bytes` to `path`. Creates parent directories.
///
/// 1) Open a NamedTempFile in the same directory as `path`.
/// 2) Write all bytes.
/// 3) fsync the tempfile so the data is durable before the rename.
/// 4) Atomically rename over `path`.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    use std::io::Write;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)
        .map_err(|e| ChanError::Io(e.error.to_string()))?;
    Ok(())
}

/// Human-readable name for a file type, used in error messages and
/// log lines. Covers the unix-only special types behind cfg(unix);
/// other platforms collapse them under "unknown" since std doesn't
/// surface them through `FileType` directly.
pub fn describe_file_kind(ft: &std::fs::FileType) -> &'static str {
    if ft.is_dir() {
        return "directory";
    }
    if ft.is_symlink() {
        return "symlink";
    }
    if ft.is_file() {
        return "regular";
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if ft.is_fifo() {
            return "fifo";
        }
        if ft.is_socket() {
            return "socket";
        }
        if ft.is_char_device() {
            return "char_device";
        }
        if ft.is_block_device() {
            return "block_device";
        }
    }
    "unknown"
}

/// Reject anything that isn't a regular file. Uses `lstat` semantics
/// (`symlink_metadata`) so a symlink target's kind cannot mask the
/// link itself. Call this before opening a path for read or write
/// so the layer never:
///
///   - blocks forever on a FIFO with no writer;
///   - drains `/dev/zero` or the like into a buffer;
///   - sends ioctl-shaped reads to a char/block device;
///   - follows a symlink and writes through it (atomic_write
///     replaces the symlink itself, but `read_text` and friends
///     would happily resolve through one).
///
/// Returns `Ok(())` when the path is a regular file. `ENOENT`
/// propagates as a normal `Io` error so callers can distinguish
/// "no file" from "wrong file type".
pub fn ensure_regular_file(path: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    let ft = meta.file_type();
    if ft.is_file() && !ft.is_symlink() {
        return Ok(());
    }
    Err(ChanError::SpecialFile {
        kind: describe_file_kind(&ft).to_string(),
        path: path.to_path_buf(),
    })
}

/// Stricter resolve: lexical `resolve_safe` plus a canonical-form
/// check that the deepest existing ancestor still lives under the
/// canonical drive root. Catches the case where a mid-path
/// component is a symlink pointing outside the drive (e.g. a user
/// has `Backup -> /Volumes/external` inside their drive, and a
/// caller asks to write `Backup/today.md`; we refuse).
///
/// For paths that don't exist yet (typical for create/write), we
/// canonicalize the deepest existing ancestor instead of the leaf.
/// This mirrors what the kernel will do when it walks the path
/// during the actual open call.
pub fn resolve_safe_strict(root: &Path, requested: &str) -> Result<PathBuf> {
    let joined = resolve_safe(root, requested)?;
    let root_canon = root
        .canonicalize()
        .map_err(|e| ChanError::Io(format!("canonicalize drive root: {e}")))?;

    // Find the deepest ancestor of `joined` that already exists,
    // canonicalize it, and check it stays under root_canon.
    let mut probe: &Path = &joined;
    let canon_ancestor = loop {
        match probe.canonicalize() {
            Ok(c) => break c,
            Err(_) => match probe.parent() {
                Some(p) => probe = p,
                // We walked past the drive root without finding
                // anything that canonicalizes; treat as escape.
                None => return Err(ChanError::SymlinkEscape(joined)),
            },
        }
    };

    if !canon_ancestor.starts_with(&root_canon) {
        return Err(ChanError::SymlinkEscape(joined));
    }
    Ok(joined)
}

/// Take an untrusted request path (`notes/x.md` or `../etc/passwd`)
/// and join it onto the drive root, rejecting any traversal that
/// escapes the root. Returns the absolute joined path.
///
/// This is a LEXICAL check only: it does not detect mid-path
/// symlinks pointing outside the drive. Use `resolve_safe_strict`
/// for that. We keep this as a fast-path for tests and for the
/// strict variant's first leg.
pub fn resolve_safe(root: &Path, requested: &str) -> Result<PathBuf> {
    let requested = requested.trim_start_matches('/');
    if requested.is_empty() {
        return Err(ChanError::PathEmpty);
    }
    let raw = Path::new(requested);
    let mut joined = PathBuf::from(root);
    for component in raw.components() {
        match component {
            Component::Normal(c) => joined.push(c),
            Component::CurDir => {}
            _ => return Err(ChanError::PathEscape),
        }
    }
    Ok(joined)
}

/// One entry in the file tree. Path is relative to the drive root
/// using `/` separators on all platforms (stable JSON shape).
#[derive(Debug, Clone, Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub is_dir: bool,
    /// Last modification time as Unix seconds. None if unavailable.
    pub mtime: Option<i64>,
    /// File size in bytes. 0 for directories.
    pub size: u64,
}

/// Recursively list everything under `root`. Skips `.git/` and
/// `.chan/` at any depth.
pub fn list_tree(root: &Path) -> Result<Vec<TreeEntry>> {
    let mut out = Vec::new();
    for entry in walk_drive(root) {
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| ChanError::PathEscape)?;
        let path_str = rel.to_string_lossy().replace('\\', "/");
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(?path_str, ?e, "metadata failed; skipping");
                continue;
            }
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        out.push(TreeEntry {
            path: path_str,
            is_dir: meta.is_dir(),
            mtime,
            size: if meta.is_dir() { 0 } else { meta.len() },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_creates_dirs() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a/b/c.txt");
        atomic_write(&p, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_overwrites() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("x.txt");
        atomic_write(&p, b"v1").unwrap();
        atomic_write(&p, b"v2").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "v2");
    }

    #[test]
    fn list_tree_skips_internal_dirs() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".chan")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();
        std::fs::write(tmp.path().join(".chan/x"), b"").unwrap();
        std::fs::write(tmp.path().join(".git/HEAD"), b"").unwrap();
        std::fs::write(tmp.path().join("note.md"), b"hi").unwrap();
        let tree = list_tree(tmp.path()).unwrap();
        let paths: Vec<_> = tree.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"note.md"));
        assert!(!paths.iter().any(|p| p.starts_with(".chan")));
        assert!(!paths.iter().any(|p| p.starts_with(".git")));
    }

    #[test]
    fn resolve_safe_rejects_traversal() {
        let tmp = TempDir::new().unwrap();
        assert!(matches!(
            resolve_safe(tmp.path(), "../escape").unwrap_err(),
            ChanError::PathEscape
        ));
        assert!(matches!(
            resolve_safe(tmp.path(), "a/../b").unwrap_err(),
            ChanError::PathEscape
        ));
    }

    #[test]
    fn resolve_safe_accepts_normal() {
        let tmp = TempDir::new().unwrap();
        let r = resolve_safe(tmp.path(), "notes/x.md").unwrap();
        assert!(r.starts_with(tmp.path()));
        assert!(r.ends_with("notes/x.md"));
    }

    #[test]
    fn is_editable_text_whitelist() {
        assert!(is_editable_text("note.md"));
        assert!(is_editable_text("a/b/c.txt"));
        assert!(is_editable_text("README.MD"));
        assert!(!is_editable_text("image.png"));
        assert!(!is_editable_text(""));
        assert!(!is_editable_text(".gitignore"));
    }

    #[test]
    fn ensure_regular_file_accepts_regular() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a.md");
        std::fs::write(&p, b"hi").unwrap();
        ensure_regular_file(&p).unwrap();
    }

    #[test]
    fn ensure_regular_file_rejects_directory() {
        let tmp = TempDir::new().unwrap();
        let err = ensure_regular_file(tmp.path()).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "directory"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_regular_file_rejects_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.md");
        let link = tmp.path().join("link.md");
        std::fs::write(&target, b"hi").unwrap();
        symlink(&target, &link).unwrap();
        let err = ensure_regular_file(&link).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "symlink"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_regular_file_rejects_unix_socket() {
        use std::os::unix::net::UnixListener;
        let tmp = TempDir::new().unwrap();
        let sock = tmp.path().join("s");
        let _l = UnixListener::bind(&sock).unwrap();
        let err = ensure_regular_file(&sock).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "socket"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolve_safe_strict_rejects_midpath_symlink_to_outside() {
        use std::os::unix::fs::symlink;
        let outside = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        // Backup -> outside dir.
        symlink(outside.path(), drive.path().join("Backup")).unwrap();
        let err = resolve_safe_strict(drive.path(), "Backup/today.md").unwrap_err();
        assert!(matches!(err, ChanError::SymlinkEscape(_)));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_safe_strict_allows_symlink_pointing_inside() {
        use std::os::unix::fs::symlink;
        let drive = TempDir::new().unwrap();
        std::fs::create_dir(drive.path().join("real")).unwrap();
        // alias -> ./real, both under the drive. The strict resolve
        // doesn't reject in-drive symlinks; the per-path lstat gate
        // in Drive::read_text / write_text is what catches them as
        // a final-component policy.
        symlink("real", drive.path().join("alias")).unwrap();
        resolve_safe_strict(drive.path(), "alias/x.md").unwrap();
    }

    #[test]
    fn resolve_safe_strict_passes_normal_path() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("notes")).unwrap();
        resolve_safe_strict(tmp.path(), "notes/x.md").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn walk_drive_drops_symlinks_and_special_files() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("note.md"), b"hi").unwrap();
        symlink("note.md", tmp.path().join("alias.md")).unwrap();
        let _l = UnixListener::bind(tmp.path().join("sock")).unwrap();
        let names: Vec<_> = walk_drive(tmp.path())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"note.md".to_string()));
        assert!(!names.contains(&"alias.md".to_string()));
        assert!(!names.contains(&"sock".to_string()));
    }
}
