//! File-descriptor pressure probes for indexing internals.
//!
//! chan-drive runs inside the editor process, so search indexing must
//! leave room for ordinary editor reads, writes, terminal PTYs, and
//! watcher handles. macOS shells commonly start with a soft `nofile`
//! limit of 256, which is low enough that eager SQLite pools plus
//! Tantivy worker fanout can exhaust the process table during first
//! boot on a large drive.

use std::sync::{Condvar, Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FdSnapshot {
    pub open: u64,
    pub limit: u64,
}

impl FdSnapshot {
    fn remaining(self) -> u64 {
        self.limit.saturating_sub(self.open)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TantivyWriterBudget {
    pub worker_threads: usize,
    pub merge_threads: usize,
}

#[derive(Debug)]
pub(crate) struct DrivePermit {
    _private: (),
}

struct DriveGate {
    state: Mutex<DriveGateState>,
    ready: Condvar,
}

#[derive(Default)]
struct DriveGateState {
    active: usize,
}

const LOW_LIMIT: u64 = 512;
const EFFECTIVE_NOFILE_CEILING: u64 = 4096;
const TIGHT_HEADROOM: u64 = 96;
const MODEST_HEADROOM: u64 = 192;
const MAX_ACTIVE_DRIVES: usize = 64;
const LOW_LIMIT_ACTIVE_DRIVES: usize = 8;
const TIGHT_HEADROOM_ACTIVE_DRIVES: usize = 4;
const MODEST_HEADROOM_ACTIVE_DRIVES: usize = 8;

static DRIVE_GATE: OnceLock<DriveGate> = OnceLock::new();

pub(crate) fn snapshot() -> Option<FdSnapshot> {
    fd_snapshot()
}

pub(crate) fn graph_reader_pool_size(default: u32) -> u32 {
    match snapshot() {
        Some(snap) => graph_reader_pool_size_for(default, snap),
        None => default.max(1),
    }
}

pub(crate) fn cap_index_read_workers(requested: usize) -> usize {
    match snapshot() {
        Some(snap) => cap_index_read_workers_for(requested, snap),
        None => requested.max(1),
    }
}

pub(crate) fn tantivy_writer_budget(default_worker_threads: usize) -> TantivyWriterBudget {
    match snapshot() {
        Some(snap) => tantivy_writer_budget_for(default_worker_threads, snap),
        None => TantivyWriterBudget {
            worker_threads: default_worker_threads.max(1),
            merge_threads: 4,
        },
    }
}

pub(crate) fn acquire_drive_permit() -> DrivePermit {
    let gate = drive_gate();
    let mut state = gate.state.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let capacity = match snapshot() {
            Some(snap) => active_drive_capacity_for(snap),
            None => MAX_ACTIVE_DRIVES,
        };
        if state.active < capacity {
            state.active += 1;
            return DrivePermit { _private: () };
        }
        state = gate.ready.wait(state).unwrap_or_else(|e| e.into_inner());
    }
}

fn graph_reader_pool_size_for(default: u32, snap: FdSnapshot) -> u32 {
    let default = default.max(1);
    if snap.limit <= LOW_LIMIT || snap.remaining() < TIGHT_HEADROOM {
        1
    } else if snap.remaining() < MODEST_HEADROOM {
        default.min(2)
    } else {
        default
    }
}

fn cap_index_read_workers_for(requested: usize, snap: FdSnapshot) -> usize {
    let requested = requested.max(1);
    if snap.limit <= LOW_LIMIT || snap.remaining() < TIGHT_HEADROOM {
        1
    } else if snap.remaining() < MODEST_HEADROOM {
        requested.min(2)
    } else {
        requested
    }
}

fn tantivy_writer_budget_for(
    default_worker_threads: usize,
    snap: FdSnapshot,
) -> TantivyWriterBudget {
    let default_worker_threads = default_worker_threads.max(1);
    if snap.limit <= LOW_LIMIT || snap.remaining() < TIGHT_HEADROOM {
        TantivyWriterBudget {
            worker_threads: 1,
            merge_threads: 1,
        }
    } else if snap.remaining() < MODEST_HEADROOM {
        TantivyWriterBudget {
            worker_threads: default_worker_threads.min(2),
            merge_threads: 1,
        }
    } else {
        TantivyWriterBudget {
            worker_threads: default_worker_threads,
            merge_threads: 4,
        }
    }
}

fn active_drive_capacity_for(snap: FdSnapshot) -> usize {
    if snap.limit <= LOW_LIMIT {
        LOW_LIMIT_ACTIVE_DRIVES
    } else if snap.remaining() < TIGHT_HEADROOM {
        TIGHT_HEADROOM_ACTIVE_DRIVES
    } else if snap.remaining() < MODEST_HEADROOM {
        MODEST_HEADROOM_ACTIVE_DRIVES
    } else {
        MAX_ACTIVE_DRIVES
    }
}

fn drive_gate() -> &'static DriveGate {
    DRIVE_GATE.get_or_init(|| DriveGate {
        state: Mutex::new(DriveGateState::default()),
        ready: Condvar::new(),
    })
}

impl Drop for DrivePermit {
    fn drop(&mut self) {
        let gate = drive_gate();
        let mut state = gate.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active = state.active.saturating_sub(1);
        gate.ready.notify_one();
    }
}

#[cfg(unix)]
fn fd_snapshot() -> Option<FdSnapshot> {
    let open = std::fs::read_dir("/dev/fd").ok()?.count() as u64;
    let limit = nofile_limit()?;
    Some(FdSnapshot { open, limit })
}

#[cfg(not(unix))]
fn fd_snapshot() -> Option<FdSnapshot> {
    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn nofile_limit() -> Option<u64> {
    let current = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
    Some(effective_nofile_limit(current))
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn nofile_limit() -> Option<u64> {
    Some(EFFECTIVE_NOFILE_CEILING)
}

fn effective_nofile_limit(limit: Option<u64>) -> u64 {
    limit
        .unwrap_or(EFFECTIVE_NOFILE_CEILING)
        .min(EFFECTIVE_NOFILE_CEILING)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_pool_shrinks_on_low_soft_limit() {
        let snap = FdSnapshot {
            open: 20,
            limit: 256,
        };
        assert_eq!(graph_reader_pool_size_for(4, snap), 1);
    }

    #[test]
    fn graph_pool_shrinks_when_headroom_is_tight() {
        let snap = FdSnapshot {
            open: 950,
            limit: 1024,
        };
        assert_eq!(graph_reader_pool_size_for(4, snap), 1);
    }

    #[test]
    fn graph_pool_keeps_default_when_headroom_is_clear() {
        let snap = FdSnapshot {
            open: 100,
            limit: 1024,
        };
        assert_eq!(graph_reader_pool_size_for(4, snap), 4);
    }

    #[test]
    fn index_read_workers_are_capped_under_fd_pressure() {
        let snap = FdSnapshot {
            open: 200,
            limit: 256,
        };
        assert_eq!(cap_index_read_workers_for(6, snap), 1);
    }

    #[test]
    fn tantivy_writer_budget_uses_single_thread_under_low_limit() {
        let snap = FdSnapshot {
            open: 20,
            limit: 256,
        };
        assert_eq!(
            tantivy_writer_budget_for(3, snap),
            TantivyWriterBudget {
                worker_threads: 1,
                merge_threads: 1
            }
        );
    }

    #[test]
    fn active_drive_capacity_is_bounded_on_low_soft_limit() {
        let snap = FdSnapshot {
            open: 20,
            limit: 256,
        };
        assert_eq!(active_drive_capacity_for(snap), LOW_LIMIT_ACTIVE_DRIVES);
    }

    #[test]
    fn active_drive_capacity_uses_internal_ceiling_with_clear_headroom() {
        let snap = FdSnapshot {
            open: 20,
            limit: 4096,
        };
        assert_eq!(active_drive_capacity_for(snap), MAX_ACTIVE_DRIVES);
    }

    #[test]
    fn unlimited_nofile_uses_internal_ceiling() {
        assert_eq!(effective_nofile_limit(None), EFFECTIVE_NOFILE_CEILING);
        assert_eq!(
            effective_nofile_limit(Some(EFFECTIVE_NOFILE_CEILING * 4)),
            EFFECTIVE_NOFILE_CEILING
        );
    }
}
