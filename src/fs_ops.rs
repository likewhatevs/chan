// Filesystem helpers for the drive:
//   - atomic_write: tmpfile + fsync + rename. Used everywhere that
//     touches a file on behalf of the user. We never want a half-
//     written note.
//   - resolve_safe: normalize a request path and reject anything
//     that escapes the drive root via `..` etc.
//   - is_editable_text: extension whitelist gate.
//   - walk_drive / list_tree: recursive listing scoped to the drive,
//     skipping `.git/` and `.chan/` at any depth.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use crate::error::{ChanError, Result};

/// True for paths inside the drive-internal `.chan/` dir. chan-core
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

/// Recursive walker rooted at `root` that skips `.git/` and `.chan/`
/// at any depth. Per-entry errors are logged and skipped.
pub fn walk_drive(root: &Path) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(root)
        .min_depth(1)
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

/// Take an untrusted request path (`notes/x.md` or `../etc/passwd`)
/// and join it onto the drive root, rejecting any traversal that
/// escapes the root. Returns the absolute joined path.
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
}
