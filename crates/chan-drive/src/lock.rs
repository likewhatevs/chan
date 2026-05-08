// Cross-process advisory locks for per-drive state.
//
// Two processes (e.g. `chan serve` running on a drive that the
// native desktop app then opens) must not both try to write the
// search index or graph DB at once. We use file-based advisory
// locks via fs4 (Unix flock + Windows LockFileEx).
//
// `DriveLock` is the writer lock: held for the lifetime of a Drive
// open in writer mode. Reading callers don't take any lock; tantivy
// and sqlite handle their own multi-reader concurrency.

use std::fs::{File, OpenOptions};
use std::path::Path;

use fs4::fs_std::FileExt;

use crate::error::{ChanError, Result};

/// Process-wide advisory lock on a per-drive lockfile. Drop to
/// release. Cross-platform via fs4 (flock on Unix, LockFileEx on
/// Windows).
pub struct DriveLock {
    /// Holds the lock; the file lives as long as this struct.
    file: File,
}

impl DriveLock {
    /// Try to acquire the writer lock for this drive. Fails fast
    /// with `ChanError::DriveLocked` if another process holds it.
    pub fn acquire(lock_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(lock_dir)?;
        let path = lock_dir.join("writer.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Self { file }),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Err(ChanError::DriveLocked),
            Err(e) => Err(ChanError::Io(e.to_string())),
        }
    }
}

impl Drop for DriveLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_and_release() {
        let tmp = TempDir::new().unwrap();
        let lock = DriveLock::acquire(tmp.path()).unwrap();
        drop(lock);
        // Re-acquire after drop must succeed.
        let _lock2 = DriveLock::acquire(tmp.path()).unwrap();
    }

    #[test]
    fn second_acquire_fails_while_held() {
        let tmp = TempDir::new().unwrap();
        let _l1 = DriveLock::acquire(tmp.path()).unwrap();
        let r2 = DriveLock::acquire(tmp.path());
        assert!(matches!(r2, Err(ChanError::DriveLocked)));
    }
}
