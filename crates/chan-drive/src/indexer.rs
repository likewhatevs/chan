// Built-in graph indexer. Owns a watcher subscription, debounces
// per-path events, and drives `Drive::index_file` / `forget_file`
// / `reconcile` so consumers (the CLI, chan-server, and the future
// Swift / Kotlin shells) do not each reinvent the same queue.
//
// Threading model:
//
//   * One worker thread per indexer, named "chan-drive::indexer".
//   * The watcher's notify thread is the producer. It hands events
//     to the indexer through an mpsc channel via an internal
//     `WatchCallback` so the indexer thread is the only one
//     touching graph + index.
//   * Drop the `GraphIndexer` (or call `stop()`) to tear down both
//     the watcher and the worker. Cleanup is synchronous: the
//     watcher is dropped first to close the channel, then the
//     worker joined.
//
// Debouncing:
//
//   * Per-path, trailing-edge. The first event for a path schedules
//     a deadline at `now + debounce`; subsequent events for the same
//     path push the deadline forward. The deadline maturing is the
//     trigger to run `index_file`.
//   * `Removed` and the source side of `Renamed` skip the debounce
//     entirely: deletions are immediate because a stale graph row
//     pointing at a missing file is the user-visible failure mode,
//     and re-creating the same path inside the debounce window is
//     fine (the subsequent `Modified`/`Created` re-schedules the
//     deadline).
//   * `ProviderError` and path-less events (the watcher's "scope
//     unknown" signal) clear the pending map and trigger a full
//     `Drive::reconcile`. The reconcile is the same convergence
//     path used for cold-open and offline-edit catch-up.
//
// Tests in this module exercise the FS path with a real watcher,
// so they are timing-sensitive on slower hosts; bumps to the
// `DEBOUNCE_TEST_MS` constant compound through every test.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::drive::Drive;
use crate::error::{ChanError, Result};
use crate::watch::{WatchCallback, WatchEvent, WatchHandle, WatchKind};

/// Default debounce window for production. 150 ms covers the burst
/// of events a typical editor emits per save (Modify/Modify or
/// Rename/Modify) without making typing feel sluggish to the
/// downstream search index.
pub const DEFAULT_DEBOUNCE_MS: u64 = 150;

/// Handle to a running graph indexer. Cheap to clone (Arc inside).
/// Drop to stop; the watcher is released and the worker thread
/// joined synchronously.
pub struct GraphIndexer {
    inner: Arc<GraphIndexerInner>,
}

impl Clone for GraphIndexer {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct GraphIndexerInner {
    stop: AtomicBool,
    pending: AtomicUsize,
    indexed_total: AtomicU64,
    forgotten_total: AtomicU64,
    reconciles_total: AtomicU64,
    /// Worker thread handle. `Option` so `stop()` can take it.
    thread: Mutex<Option<JoinHandle<()>>>,
    /// Held to keep the watcher producer alive. Dropping this
    /// closes the channel, which lets the worker exit on its next
    /// `recv_timeout`.
    watch: Mutex<Option<WatchHandle>>,
}

impl GraphIndexer {
    /// Start an indexer attached to `drive`. The first call to
    /// `start_on` opens the watcher; subsequent calls on the same
    /// drive are supported (each indexer owns its own watcher
    /// handle and worker), though pairing two indexers against the
    /// same drive in production would only burn CPU.
    pub fn start_on(drive: Arc<Drive>, debounce_ms: u64) -> Result<Self> {
        let inner = Arc::new(GraphIndexerInner {
            stop: AtomicBool::new(false),
            pending: AtomicUsize::new(0),
            indexed_total: AtomicU64::new(0),
            forgotten_total: AtomicU64::new(0),
            reconciles_total: AtomicU64::new(0),
            thread: Mutex::new(None),
            watch: Mutex::new(None),
        });

        let (tx, rx) = mpsc::channel::<WatchEvent>();
        let cb: Arc<dyn WatchCallback> = Arc::new(EventForwarder { tx });
        let watch = drive.watch(cb)?;

        let drive_w = Arc::clone(&drive);
        let inner_w = Arc::clone(&inner);
        let debounce = Duration::from_millis(debounce_ms);
        let thread = std::thread::Builder::new()
            .name("chan-drive::indexer".into())
            .spawn(move || run_loop(drive_w, rx, inner_w, debounce))
            .map_err(|e| ChanError::Io(format!("spawn indexer thread: {e}")))?;

        *inner.thread.lock().unwrap() = Some(thread);
        *inner.watch.lock().unwrap() = Some(watch);

        Ok(Self { inner })
    }

    /// Files currently waiting on their debounce window.
    pub fn pending_count(&self) -> usize {
        self.inner.pending.load(Ordering::Acquire)
    }

    /// Cumulative number of files successfully indexed since start.
    /// Includes per-event index_file calls AND files indexed by
    /// reconcile.
    pub fn indexed_total(&self) -> u64 {
        self.inner.indexed_total.load(Ordering::Acquire)
    }

    /// Cumulative number of files forgotten since start (per-event
    /// removes AND reconcile-driven forgets).
    pub fn forgotten_total(&self) -> u64 {
        self.inner.forgotten_total.load(Ordering::Acquire)
    }

    /// Cumulative count of full-reconcile passes triggered by
    /// `ProviderError` or path-less events. A high count here
    /// usually means the watcher backend is dropping events
    /// (inotify queue overflow on Linux, FSEvents coalesce on
    /// macOS) and the consumer should consider raising
    /// `fs.inotify.max_queued_events` or similar.
    pub fn reconciles_total(&self) -> u64 {
        self.inner.reconciles_total.load(Ordering::Acquire)
    }

    /// Stop the indexer. Idempotent. Synchronous: returns only after
    /// the worker has finished its current op and the watcher is
    /// torn down. Called automatically on `Drop` of the last clone.
    pub fn stop(&self) {
        self.inner.stop.store(true, Ordering::Release);
        // Drop the watch first so the channel closes; the worker
        // then exits on RecvTimeoutError::Disconnected. Without this
        // we'd race the stop flag against in-flight events.
        let watch = self.inner.watch.lock().unwrap().take();
        drop(watch);
        if let Some(t) = self.inner.thread.lock().unwrap().take() {
            let _ = t.join();
        }
    }
}

impl Drop for GraphIndexer {
    fn drop(&mut self) {
        // Only the last surviving clone triggers the teardown. The
        // strong_count check is best-effort: clones held by other
        // threads can still race. We treat the indexer as a singleton
        // in practice (chan-server holds one); the multi-clone case
        // is allowed but undefined for stop timing.
        if Arc::strong_count(&self.inner) == 1 && !self.inner.stop.load(Ordering::Acquire) {
            self.stop();
        }
    }
}

struct EventForwarder {
    tx: mpsc::Sender<WatchEvent>,
}

impl WatchCallback for EventForwarder {
    fn on_event(&self, event: WatchEvent) {
        // Best-effort: if the receiver is gone the channel is closed
        // (indexer is stopping). Silent drop is correct here; the
        // consumer's signal to retry indexing is the next reconcile,
        // not the watcher.
        let _ = self.tx.send(event);
    }
}

fn run_loop(
    drive: Arc<Drive>,
    rx: mpsc::Receiver<WatchEvent>,
    state: Arc<GraphIndexerInner>,
    debounce: Duration,
) {
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        if state.stop.load(Ordering::Acquire) {
            break;
        }
        // Block until either a new event arrives or the next
        // debounce deadline matures. Floor at 1 ms so a stalled
        // deadline does not pin the CPU at 100%.
        let timeout = match pending.values().min().copied() {
            Some(d) => d
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(1)),
            None => Duration::from_secs(60),
        };
        match rx.recv_timeout(timeout) {
            Ok(event) => apply_event(event, &mut pending, &drive, &state, debounce),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // Process matured pending entries. We collect then mutate
        // so the iterator doesn't borrow `pending` across the
        // mutate-and-call sequence.
        let now = Instant::now();
        let ready: Vec<String> = pending
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(k, _)| k.clone())
            .collect();
        for path in &ready {
            pending.remove(path);
            // Best-effort per-file: a failure to index leaves the
            // file out of the index until the next save or a
            // reconcile. The journal in PR5 already protects
            // against partial-commit drift inside index_file
            // itself.
            match drive.index_file(path) {
                Ok(()) => {
                    state.indexed_total.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    tracing::warn!(path = %path, ?e, "indexer: index_file failed");
                }
            }
        }

        state.pending.store(pending.len(), Ordering::Release);
    }
    tracing::debug!("graph indexer loop exiting");
}

fn apply_event(
    event: WatchEvent,
    pending: &mut HashMap<String, Instant>,
    drive: &Arc<Drive>,
    state: &GraphIndexerInner,
    debounce: Duration,
) {
    match event.kind {
        WatchKind::ProviderError => {
            // Stream untrusted: pending entries may be wrong, drop
            // them and run reconcile against the live tree.
            pending.clear();
            run_reconcile(drive, state, "ProviderError");
        }
        WatchKind::Modified | WatchKind::Created => match event.path {
            Some(p) => {
                pending.insert(p, Instant::now() + debounce);
            }
            None => {
                pending.clear();
                run_reconcile(drive, state, "path-less event");
            }
        },
        WatchKind::Removed => {
            if let Some(p) = event.path {
                pending.remove(&p);
                match drive.forget_file(&p) {
                    Ok(()) => {
                        state.forgotten_total.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::warn!(path = %p, ?e, "indexer: forget_file failed");
                    }
                }
            }
        }
        WatchKind::Renamed => {
            if let Some(from) = event.path {
                pending.remove(&from);
                match drive.forget_file(&from) {
                    Ok(()) => {
                        state.forgotten_total.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::warn!(path = %from, ?e, "indexer: forget_file failed on rename src");
                    }
                }
            }
            if let Some(to) = event.to {
                pending.insert(to, Instant::now() + debounce);
            }
        }
    }
}

fn run_reconcile(drive: &Arc<Drive>, state: &GraphIndexerInner, reason: &'static str) {
    tracing::info!(reason, "indexer: reconciling against live tree");
    match drive.reconcile() {
        Ok(report) => {
            state
                .indexed_total
                .fetch_add(report.upserted.len() as u64, Ordering::Relaxed);
            state
                .forgotten_total
                .fetch_add(report.forgotten.len() as u64, Ordering::Relaxed);
            state.reconciles_total.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            tracing::warn!(?e, "indexer: reconcile failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::SearchMode;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Tight debounce so FS-based tests complete quickly. Production
    /// callers use `DEFAULT_DEBOUNCE_MS` or whatever their UX
    /// requires.
    const DEBOUNCE_TEST_MS: u64 = 30;

    /// Poll a closure until it returns true or `timeout` elapses.
    /// Returns true on success, false on timeout. FS watcher delivery
    /// is asynchronous; tests need a bounded wait rather than a fixed
    /// sleep so they pass quickly on fast hosts and tolerate slower
    /// CI.
    fn wait_for(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if check() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        check()
    }

    fn setup_drive() -> (TempDir, TempDir, Arc<Drive>) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), None).unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        (cfg, drive_dir, drive)
    }

    #[test]
    #[ignore = "requires BGE-small embedding model on disk; run with `cargo test -- --ignored` on a workstation with the model cached (see systacean-18)"]
    fn writes_to_disk_get_indexed_after_debounce() {
        let (_cfg, drive_dir, drive) = setup_drive();
        let indexer = GraphIndexer::start_on(Arc::clone(&drive), DEBOUNCE_TEST_MS).unwrap();

        // Write directly to disk so the watcher's notify backend
        // (not the drive's API) is the trigger. write_text would
        // also work, but bypassing it confirms the watcher path
        // end-to-end.
        std::fs::write(
            drive_dir.path().join("watched.md"),
            "# watched\nwatcher-token here\n",
        )
        .unwrap();

        // Wait for the indexer to pick the event up and clear the
        // debounce window. 5s is generous against macOS FSEvents
        // latency; locally this typically lands in <100 ms.
        let saw = wait_for(Duration::from_secs(5), || indexer.indexed_total() >= 1);
        assert!(saw, "indexer did not pick up the file write");
        let opts = crate::drive::SearchOpts {
            mode: SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let hits = drive.search("watcher-token", &opts).unwrap();
        assert!(
            hits.hits.iter().any(|h| h.path == "watched.md"),
            "expected watched.md in search hits; got {:?}",
            hits.hits
        );

        indexer.stop();
    }

    #[test]
    fn delete_from_disk_drops_file_from_index() {
        let (_cfg, drive_dir, drive) = setup_drive();
        // Pre-populate via the API + reindex; the indexer starts
        // up afterwards and only needs to handle the delete.
        drive
            .write_text("doomed.md", "# doomed\nbye-token here\n")
            .unwrap();
        drive.reindex(None).unwrap();
        let indexer = GraphIndexer::start_on(Arc::clone(&drive), DEBOUNCE_TEST_MS).unwrap();

        std::fs::remove_file(drive_dir.path().join("doomed.md")).unwrap();
        let saw = wait_for(Duration::from_secs(5), || indexer.forgotten_total() >= 1);
        assert!(saw, "indexer did not pick up the delete");

        let opts = crate::drive::SearchOpts {
            mode: SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let hits = drive.search("bye-token", &opts).unwrap();
        assert!(
            hits.hits.is_empty(),
            "deleted file should not appear in search; got {:?}",
            hits.hits
        );

        indexer.stop();
    }

    #[test]
    #[ignore = "requires BGE-small embedding model on disk; run with `cargo test -- --ignored` on a workstation with the model cached (see systacean-18)"]
    fn debounce_coalesces_rapid_writes_into_one_index() {
        // Multiple Modified events on the same path inside the
        // debounce window must collapse into a single index_file.
        // Without this, every keystroke on a never-saved file
        // would re-index, which is exactly the kind of churn the
        // debounce exists to prevent.
        let (_cfg, drive_dir, drive) = setup_drive();
        let indexer = GraphIndexer::start_on(Arc::clone(&drive), DEBOUNCE_TEST_MS).unwrap();
        let file = drive_dir.path().join("rapid.md");
        for i in 0..5 {
            std::fs::write(&file, format!("# rapid\nbody {i}\n")).unwrap();
            std::thread::sleep(Duration::from_millis(DEBOUNCE_TEST_MS / 3));
        }
        // After the burst, give the watcher time to flush and the
        // worker time to settle. We assert that the indexed total
        // is small (1-3, not 5), not that it is exactly 1: notify
        // can split a single write into two events on some
        // backends.
        let saw = wait_for(Duration::from_secs(5), || indexer.indexed_total() >= 1);
        assert!(saw);
        std::thread::sleep(Duration::from_millis(DEBOUNCE_TEST_MS * 3));
        let total = indexer.indexed_total();
        assert!(
            total <= 3,
            "5 rapid writes should debounce to at most a few index passes; got {total}",
        );
        indexer.stop();
    }

    #[test]
    fn stop_is_idempotent_and_releases_resources() {
        let (_cfg, _drive_dir, drive) = setup_drive();
        let indexer = GraphIndexer::start_on(Arc::clone(&drive), DEBOUNCE_TEST_MS).unwrap();
        indexer.stop();
        indexer.stop();
        // After stop, indexer is a husk: counters still readable
        // for last-known values, no new threads or watchers active.
        assert_eq!(indexer.pending_count(), 0);
    }
}
