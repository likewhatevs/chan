//! A tiny on-disk blob store for workspace-LESS terminal tenants.
//!
//! A standalone terminal window has no workspace dir, so its per-window
//! session (pane/tab layout) blob can't ride the workspace `sessions/` store.
//! For a *persisted* devserver terminal we want that layout to survive a
//! devserver restart, so we mirror the workspace store — atomic tmp+rename,
//! flat keys — at the launcher scope (`~/.chan/devserver/terminals/`). Keys
//! are the `?w=<window-label>` ids; the blobs are opaque SPA layout bytes.
//!
//! A terminal tenant with no store dir keeps using the in-memory
//! `AppState::ephemeral_sessions` (transient: a control terminal, or a
//! desktop-local terminal whose layout lives in the desktop `Config`).

use std::path::{Path, PathBuf};

/// Validate a flat blob key, mirroring `chan_workspace::blob::validate_key`
/// so a terminal window's `?w=` label persists exactly like a workspace
/// window's: ASCII alphanumeric plus `-`/`_`/`.`, length 1..=255, and no
/// leading `.` (would write a hidden file) or `-` (would look like a CLI
/// flag). It is a single safe path segment, so a hostile `?w=` (e.g.
/// `../../etc/passwd`) can never escape the store dir.
fn safe_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 255
        && !key.starts_with('.')
        && !key.starts_with('-')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn key_path(dir: &Path, key: &str) -> Option<PathBuf> {
    safe_key(key).then(|| dir.join(key))
}

/// Write `content` for `key` atomically (tmp + fsync + rename), creating
/// `dir`. An invalid key is rejected rather than written.
pub fn put(dir: &Path, key: &str, content: &[u8]) -> std::io::Result<()> {
    let Some(path) = key_path(dir, key) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid session key",
        ));
    };
    std::fs::create_dir_all(dir)?;
    // Leading-dot tmp: a valid key can never start with `.`, so this never
    // collides with another key's blob (even a key like `foo.bar`), and
    // `list` skips it (it fails `safe_key`).
    let tmp = dir.join(format!(".{key}.tmp"));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &path)?;
    // Best-effort dirent durability, matching the workspace store + the
    // devserver config write.
    let _ = chan_workspace::fs_ops::sync_dir(dir);
    Ok(())
}

/// Read `key`'s blob, or `None` when it is absent or the key is invalid.
pub fn get(dir: &Path, key: &str) -> std::io::Result<Option<Vec<u8>>> {
    let Some(path) = key_path(dir, key) else {
        return Ok(None);
    };
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Sorted flat keys present in `dir` (empty when the dir is absent). Skips
/// the `.tmp` write file and anything that isn't a valid key.
pub fn list(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut keys = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(rd) => {
            for entry in rd.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if safe_key(name) {
                        keys.push(name.to_string());
                    }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    keys.sort();
    Ok(keys)
}

/// Idempotent delete; a missing key (or an invalid one) is `Ok(())`.
pub fn delete(dir: &Path, key: &str) -> std::io::Result<()> {
    let Some(path) = key_path(dir, key) else {
        return Ok(());
    };
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_list_delete_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        assert!(get(d, "terminal-1").unwrap().is_none());
        assert!(list(d).unwrap().is_empty());

        put(d, "terminal-1", b"layout-a").unwrap();
        put(d, "terminal-2", b"layout-b").unwrap();
        assert_eq!(
            get(d, "terminal-1").unwrap().as_deref(),
            Some(&b"layout-a"[..])
        );
        assert_eq!(
            list(d).unwrap(),
            vec!["terminal-1".to_string(), "terminal-2".to_string()]
        );
        // Atomic write leaves no (leading-dot) tmp behind, and it never shows
        // up in `list`.
        assert!(!d.join(".terminal-1.tmp").exists());

        delete(d, "terminal-1").unwrap();
        assert!(get(d, "terminal-1").unwrap().is_none());
        assert_eq!(list(d).unwrap(), vec!["terminal-2".to_string()]);
        // Idempotent delete.
        delete(d, "terminal-1").unwrap();
    }

    #[test]
    fn accepts_the_real_window_label_formats() {
        // The desktop's actual labels (serve.rs): `outbound-<16hex>-<seq>` and
        // `terminal-win-<n>`, plus a dotted key (validate_key allows `.`).
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for k in [
            "outbound-1a2b3c4d5e6f7890-3",
            "terminal-win-7",
            "a.b.c",
            "ok_key.v2",
        ] {
            put(d, k, b"x").unwrap_or_else(|e| panic!("should accept {k:?}: {e}"));
            assert_eq!(get(d, k).unwrap().as_deref(), Some(&b"x"[..]), "{k:?}");
        }
    }

    #[test]
    fn rejects_path_traversal_and_unsafe_keys() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        // Separators, traversal, leading `.` (hidden file) / `-` (CLI-flag),
        // empty, and illegal chars — mirrors chan_workspace::blob::validate_key.
        for bad in [
            "../escape",
            "a/b",
            "..",
            ".",
            ".hidden",
            "-flag",
            "",
            "with space",
        ] {
            assert!(put(d, bad, b"x").is_err(), "should reject put {bad:?}");
            assert!(get(d, bad).unwrap().is_none(), "should reject get {bad:?}");
            // delete of a bad key is a silent no-op.
            delete(d, bad).unwrap();
        }
        // Nothing escaped the dir.
        assert!(list(d).unwrap().is_empty());
    }
}
