//! Cross-process serial gate for the FS-timing test class.
//!
//! SYSTEMIC de-flake (phase-11). A whole class of tests fights the same
//! scarce resources under the full parallel `cargo test`: real `notify`
//! FSEvents delivery, an indexer worker thread, real Tantivy
//! commit/reader-refresh I/O, and (in chan-server) real PTY shell
//! scheduling. Under 12-way CPU saturation any of those slip past a poll
//! deadline and the test flakes, with the failing set shifting run to
//! run. The affected tests:
//!   - chan-workspace  `indexer::tests::*` (real-FSEvent indexer delivery)
//!   - chan-server `indexer::tests::*` boot-walk tests
//!   - chan-server `routes::terminal::tests::*` real-PTY shell probes
//!
//! WHY a FILE lock and not a `static` Mutex: a `static` lock serializes
//! only tests WITHIN one test binary, but `cargo test` runs each crate's
//! test binary as a SEPARATE PROCESS, concurrently. Per-crate `static`
//! locks are therefore islands - chan-workspace's FS tests still race
//! chan-server's boot-walk + PTY tests for the CPU and the kernel
//! FSEvent queue. An OS advisory lock on a well-known temp path is the
//! one primitive that spans process boundaries, so a single named gate
//! serializes the ENTIRE class across both crates: only one heavy timing
//! test runs at a time anywhere in the workspace run, while every other
//! (fast) test still runs fully parallel around it. The generous poll
//! budgets stay as a backstop but should rarely be approached once the
//! competing FS-timing load is gone.
//!
//! `std::fs::File::lock` (stable since Rust 1.89; toolchain pinned at
//! 1.95) keeps this zero-dependency. The lock auto-releases when the
//! returned guard drops at end of the test body, or, as a backstop, when
//! the test process exits.
//!
//! chan-server cannot reach this module's items without a cross-crate
//! test-support feature, so its two test modules each open the same
//! `GATE_FILE` path directly. The mutual exclusion comes from the OS
//! lock on the shared path, not from sharing this code; keep the path
//! string identical across all sites.

#![cfg(test)]

use std::fs::{File, OpenOptions};

/// Well-known lock-file name (under the OS temp dir) for the
/// cross-process FS-timing test gate. Duplicated verbatim in the
/// chan-server indexer + terminal test modules.
pub const GATE_FILE: &str = "chan-fs-timing-test.gate";

/// Acquire the cross-process FS-timing gate. Blocks until no other
/// FS-timing test (in any crate's test binary) holds it. Hold the
/// returned `File` for the duration of the test body; dropping it
/// releases the gate.
pub fn fs_timing_gate() -> File {
    let path = std::env::temp_dir().join(GATE_FILE);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .expect("open FS-timing test gate file");
    file.lock().expect("acquire FS-timing test gate");
    file
}
