// Cross-process advisory locks for per-workspace state.
//
// Two processes (e.g. `chan serve` running on a workspace that the
// native desktop app then opens) must not both try to write the
// search index or graph DB at once. We use file-based advisory
// locks via fs4 (Unix flock + Windows LockFileEx).
//
// `WorkspaceLock` is the writer lock: held for the lifetime of a Workspace
// open in writer mode. Reading callers don't take any lock; tantivy
// and sqlite handle their own multi-reader concurrency.
//
// The lock file body carries a JSON [`LockRecord`] — the holder's pid,
// canonical path, and start time — written right after the advisory
// lock is won. It serves two jobs: a contender can tell a live holder
// (refuse) from a stale record a dead one left behind (steal), and
// `chan unserve` reads it to find the process serving a path. The
// record shape is a cross-lane contract (`chan unserve` parses it).

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::{ChanError, Result};

/// Identity written into `writer.lock` by the holder immediately after
/// it wins the advisory lock.
///
/// Cross-lane on-disk shape (frozen in the round's `contracts.md`):
/// `chan unserve` parses it to discover the serving process. Keep the
/// field set and `started_at`'s RFC3339 format stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRecord {
    /// OS pid of the holder.
    pub pid: u32,
    /// Canonical absolute workspace root: a sanity check that the lock
    /// dir belongs to the workspace we think it does, plus a human hint.
    pub path: String,
    /// RFC3339 time the lock was acquired (the "held by pid N since …"
    /// diagnostic).
    pub started_at: String,
}

/// Process-wide advisory lock on a per-workspace lockfile. Drop to
/// release. Cross-platform via fs4 (flock on Unix, LockFileEx on
/// Windows).
pub struct WorkspaceLock {
    /// Holds the lock; the file lives as long as this struct.
    file: File,
}

impl WorkspaceLock {
    /// Acquire the writer lock for `lock_dir`, recording the holder's
    /// identity (`workspace_root`, this pid, now).
    ///
    /// Fast path: the OS advisory lock is free → take it and (over)write
    /// our [`LockRecord`].
    ///
    /// Contended path: the OS lock is held. We read the record and only
    /// **steal** when the recorded holder is **provably dead** (its pid
    /// no longer exists) and the record names this same workspace — the
    /// case where a dead `chan serve`'s lock fd was inherited by a still-
    /// living child and pins the flock with no real writer behind it. In
    /// every uncertain case — record missing, unparseable, for a
    /// different path, the holder alive, or liveness indeterminate — we
    /// refuse with [`ChanError::WorkspaceLocked`] rather than risk two
    /// writers corrupting the index.
    ///
    /// On Unix a normally-dead holder's flock is auto-released, so the
    /// contended path is reached only for that leaked-fd case; on Windows
    /// the steal degrades to a refuse (a LockFileEx handle can't be
    /// unlinked without `FILE_SHARE_DELETE`).
    pub fn acquire(lock_dir: &Path, workspace_root: &Path) -> Result<Self> {
        fs::create_dir_all(lock_dir)?;
        let path = lock_dir.join("writer.lock");
        let file = open_lock_file(&path)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                write_record(&file, workspace_root)?;
                Ok(Self { file })
            }
            Err(e) if is_contended(&e) => Self::try_steal(&path, workspace_root),
            Err(e) => Err(ChanError::Io(e.to_string())),
        }
    }

    /// Reclaim a contended lock iff the recorded holder is provably
    /// dead. Returns `WorkspaceLocked` whenever the steal isn't provably
    /// safe (the conservative default).
    fn try_steal(path: &Path, workspace_root: &Path) -> Result<Self> {
        let record = read_record_at(path);
        let our_path = canonical_string(workspace_root);
        let stealable = match &record {
            // Missing/torn record ⇒ holder is mid-write or unknown ⇒
            // treat as alive.
            Some(r) => r.path == our_path && holder_liveness(r.pid) == Liveness::Dead,
            None => false,
        };
        if !stealable {
            if let Some(r) = &record {
                tracing::warn!(pid = r.pid, since = %r.started_at, "workspace locked by a live holder");
            }
            return Err(ChanError::WorkspaceLocked);
        }
        // The recorded holder is dead but a leaked fd still pins the OS
        // lock on this inode. Unlink to orphan that inode, recreate the
        // file, and lock the fresh inode. The leaked fd keeps its now-
        // nameless inode; future acquirers contend on the new one.
        let dead_pid = record.as_ref().map_or(0, |r| r.pid);
        let _ = fs::remove_file(path);
        let file = open_lock_file(path)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                tracing::warn!(
                    stolen_from = dead_pid,
                    "stole writer lock from a dead holder"
                );
                write_record(&file, workspace_root)?;
                Ok(Self { file })
            }
            // Lost a race to break the stale lock; treat as locked.
            Err(e) if is_contended(&e) => Err(ChanError::WorkspaceLocked),
            Err(e) => Err(ChanError::Io(e.to_string())),
        }
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

/// Read and parse the [`LockRecord`] in `<lock_dir>/writer.lock`, if
/// present and well-formed. `chan unserve` uses this to discover the
/// process serving a workspace path.
pub fn read_lock_record(lock_dir: &Path) -> Option<LockRecord> {
    read_record_at(&lock_dir.join("writer.lock"))
}

/// Probe whether the writer lock for `lock_dir` is currently free,
/// without taking it or touching the record. `false` means some open
/// file description still holds it — including an in-flight
/// `Workspace::drop` whose flock release has not completed yet.
///
/// The close→reopen handoff uses this to confirm the prior holder's
/// flock actually released before a reopen races it: an `Arc`'s strong
/// count reaches zero *before* `Workspace::drop` runs the `_lock` drop,
/// so "no strong refs" is not the same as "flock free".
pub fn is_free(lock_dir: &Path) -> bool {
    let path = lock_dir.join("writer.lock");
    let Ok(file) = open_lock_file(&path) else {
        // Can't even open the lockfile → treat as not-free (conservative).
        return false;
    };
    match FileExt::try_lock_exclusive(&file) {
        // Held only for this probe; `file` drops here and the OS releases it.
        Ok(()) => true,
        Err(e) if is_contended(&e) => false,
        // An unexpected error is not a free lock.
        Err(_) => false,
    }
}

fn open_lock_file(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|e| ChanError::Io(e.to_string()))
}

fn write_record(mut file: &File, workspace_root: &Path) -> Result<()> {
    let record = LockRecord {
        pid: std::process::id(),
        path: canonical_string(workspace_root),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_vec(&record).map_err(|e| ChanError::Io(e.to_string()))?;
    file.set_len(0).map_err(|e| ChanError::Io(e.to_string()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| ChanError::Io(e.to_string()))?;
    file.write_all(&json)
        .map_err(|e| ChanError::Io(e.to_string()))?;
    file.flush().map_err(|e| ChanError::Io(e.to_string()))?;
    Ok(())
}

fn read_record_at(path: &Path) -> Option<LockRecord> {
    let mut buf = String::new();
    File::open(path).ok()?.read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn canonical_string(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

// On non-unix the only liveness verdict is `Indeterminate` (no
// dependency-free probe yet), so `Alive` is never constructed there.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Liveness {
    Alive,
    Dead,
    Indeterminate,
}

/// Is process `pid` alive? Conservative: only `Dead` authorizes a steal,
/// and we return `Dead` solely when the OS says "no such process".
/// Anything ambiguous (permission denied, an unexpected errno, no probe
/// available) is treated as not-dead so a live holder is never stolen.
fn holder_liveness(pid: u32) -> Liveness {
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        use rustix::process::{test_kill_process, Pid};
        let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) else {
            return Liveness::Indeterminate;
        };
        match test_kill_process(pid) {
            Ok(()) => Liveness::Alive,
            Err(Errno::SRCH) => Liveness::Dead,
            Err(Errno::PERM) => Liveness::Alive,
            Err(_) => Liveness::Indeterminate,
        }
    }
    #[cfg(not(unix))]
    {
        // No dependency-free liveness probe on Windows yet, and the
        // unlink-based steal can't break a LockFileEx handle anyway (it
        // would need FILE_SHARE_DELETE). Stay conservative — never steal
        // — until a LockFileEx-aware path lands.
        let _ = pid;
        Liveness::Indeterminate
    }
}

/// Did `try_lock_exclusive` fail because the lock is already held?
/// On Unix fs4 surfaces `WouldBlock`; on Windows it returns
/// `ERROR_LOCK_VIOLATION` / `ERROR_SHARING_VIOLATION`, which std does
/// not decode to `WouldBlock` — the historical "Windows lock-contract
/// gap". Match both so contention maps to `WorkspaceLocked` uniformly.
fn is_contended(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(windows)]
    {
        const ERROR_SHARING_VIOLATION: i32 = 32;
        const ERROR_LOCK_VIOLATION: i32 = 33;
        if matches!(
            e.raw_os_error(),
            Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION)
        ) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // The workspace root for the record; the lock dir is a sibling
    // tempdir so the two never alias.
    fn root(tmp: &TempDir) -> std::path::PathBuf {
        tmp.path().to_path_buf()
    }

    #[test]
    fn acquire_and_release() {
        let tmp = TempDir::new().unwrap();
        let lock = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        drop(lock);
        // Re-acquire after drop must succeed.
        let _lock2 = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
    }

    #[test]
    fn records_holder_identity() {
        let tmp = TempDir::new().unwrap();
        let _lock = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        let rec = read_lock_record(tmp.path()).expect("record written");
        assert_eq!(rec.pid, std::process::id());
        assert_eq!(rec.path, canonical_string(&root(&tmp)));
        assert!(!rec.started_at.is_empty());
    }

    #[test]
    fn is_free_reflects_held_state() {
        let tmp = TempDir::new().unwrap();
        assert!(is_free(tmp.path())); // nothing holds it
        let held = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        assert!(!is_free(tmp.path())); // a live holder
        drop(held);
        assert!(is_free(tmp.path())); // released after drop
    }

    // Un-gated from the old `#[cfg(unix)]`: `is_contended` now maps the
    // Windows LockFileEx error (ERROR_LOCK_VIOLATION) to contention too,
    // so the contract is symmetric. CI runs tests on unix today; the
    // Windows arm is compile-checked via `cargo xwin check`.
    #[test]
    fn second_acquire_fails_while_held() {
        let tmp = TempDir::new().unwrap();
        let _l1 = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        let r2 = WorkspaceLock::acquire(tmp.path(), &root(&tmp));
        assert!(matches!(r2, Err(ChanError::WorkspaceLocked)));
    }

    #[test]
    fn live_holder_is_never_stolen() {
        // The held lock records OUR (alive) pid; a second acquire must
        // refuse rather than steal. Stealing a live holder would corrupt
        // the index — the highest-risk failure this module guards.
        let tmp = TempDir::new().unwrap();
        let _held = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        assert_eq!(
            read_lock_record(tmp.path()).unwrap().pid,
            std::process::id()
        );
        let again = WorkspaceLock::acquire(tmp.path(), &root(&tmp));
        assert!(matches!(again, Err(ChanError::WorkspaceLocked)));
    }

    #[test]
    fn stale_record_does_not_block_a_free_lock() {
        // A dead holder's record left on disk while the OS lock is free
        // (the common `kill -9` case on Unix: flock auto-released). The
        // fast path takes the lock and overwrites the record.
        let tmp = TempDir::new().unwrap();
        let stale = LockRecord {
            pid: 999_999_999,
            path: canonical_string(&root(&tmp)),
            started_at: "2000-01-01T00:00:00Z".to_string(),
        };
        fs::create_dir_all(tmp.path()).unwrap();
        fs::write(
            tmp.path().join("writer.lock"),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();
        let _lock = WorkspaceLock::acquire(tmp.path(), &root(&tmp)).unwrap();
        assert_eq!(
            read_lock_record(tmp.path()).unwrap().pid,
            std::process::id()
        );
    }

    #[cfg(unix)]
    #[test]
    fn our_own_pid_reads_as_alive() {
        assert_eq!(holder_liveness(std::process::id()), Liveness::Alive);
    }

    #[cfg(unix)]
    #[test]
    fn reaped_child_pid_reads_as_dead() {
        // A reaped child is provably dead: `kill(pid, 0)` → ESRCH.
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn /usr/bin/true");
        let pid = child.id();
        child.wait().expect("reap child");
        assert_eq!(holder_liveness(pid), Liveness::Dead);
    }
}
