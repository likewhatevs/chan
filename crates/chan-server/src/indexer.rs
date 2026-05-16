// Background indexer driven by the existing watcher bridge.
//
// Two responsibilities:
//
//   1. On server start, kick off a full `Drive::reindex` if the
//      drive's index is empty (cold drive / fresh schema bump).
//      Runs on the tokio blocking pool so the rest of `chan serve`
//      keeps responding.
//   2. Subscribe to the watcher's `WatchEvent` broadcast and
//      debounce per-path file changes into incremental
//      `Drive::index_file` / `Drive::forget_file` calls.
//
// Status is exposed through a `Mutex<IndexStatus>` snapshot the
// `/api/index/status` endpoint reads. We deliberately don't push
// status over the WS in v1: polling the status endpoint every few
// seconds while the user is on the Settings panel is simpler and
// the payload is tiny.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chan_drive::{Drive, ProgressCallback, ProgressEvent, ProgressStage, WatchEvent, WatchKind};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

/// Snapshot of indexer state. Returned verbatim by
/// `/api/index/status` (the frontend's IndexStatus tagged union).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum IndexStatus {
    /// Initial scan in progress. `current` is 1-based, `total`
    /// counts the markdown files we found at scan start.
    Building {
        current: usize,
        total: usize,
        file: String,
    },
    /// One incremental re-index after a watcher event.
    Reindexing { file: String },
    /// Steady state. Counters mirror `Drive::index_stats`.
    Idle {
        indexed_docs: u64,
        indexed_vectors: u64,
        model: String,
    },
    /// The last operation failed; users are still allowed to query
    /// (over the previous index state).
    Error { message: String },
}

/// Handle to the background indexer. Drop it (or call `shutdown`)
/// to stop both the watcher loop and the in-flight initial build.
pub struct Indexer {
    status: Arc<Mutex<IndexStatus>>,
    rebuild_tx: mpsc::UnboundedSender<()>,
    /// Set to true on shutdown so the in-flight `Drive::reindex`
    /// blocking task bails at its next per-file check. Without this
    /// the runtime drop after `serve()` returns would have to wait
    /// for the rebuild to finish naturally; on a large drive that's
    /// minutes. Cancelled rebuilds leave the index in a clean
    /// "empty" state (no commit, graph cleared but not refilled),
    /// so the on-boot `indexed_docs == 0` trigger re-fires next run.
    cancel: Arc<AtomicBool>,
    /// Held to keep the spawned tasks alive for as long as the
    /// indexer is. Aborted on drop.
    _watcher_task: JoinHandle<()>,
    _coordinator_task: JoinHandle<()>,
}

impl std::fmt::Debug for Indexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Indexer").finish()
    }
}

impl Indexer {
    /// Spawn the indexer over `drive`, tied to `watch_events`. If
    /// `initial_build` is true and the drive's index reports zero
    /// chunks, kicks off a full rebuild on boot. `progress_sink` is
    /// the WS fan-out (see `bus::make_progress_broadcast`); per-file
    /// progress events forward there in addition to updating the
    /// local `IndexStatus` mutex behind `/api/index/status`.
    pub fn spawn(
        drive: Arc<Drive>,
        watch_events: broadcast::Receiver<WatchEvent>,
        initial_build: bool,
        progress_sink: Arc<dyn ProgressCallback>,
    ) -> Self {
        let stats = drive.index_stats().unwrap_or_else(|e| {
            tracing::warn!("indexer: initial stats failed: {e}");
            chan_drive::IndexStats {
                ready: false,
                indexed_docs: 0,
                indexed_vectors: 0,
                model: chan_drive::DEFAULT_MODEL.to_owned(),
            }
        });
        let status = Arc::new(Mutex::new(IndexStatus::Idle {
            indexed_docs: stats.indexed_docs,
            indexed_vectors: stats.indexed_vectors,
            model: stats.model.clone(),
        }));

        // Coordinator task: serializes "rebuild now" requests so
        // the watcher loop and the on-boot trigger can't both ask
        // for a full rebuild concurrently. Listening on an
        // unbounded mpsc since the bursts are tiny (one or two
        // requests per session) and dropping a request would just
        // leave the index stale.
        let cancel = Arc::new(AtomicBool::new(false));
        let (rebuild_tx, rebuild_rx) = mpsc::unbounded_channel::<()>();
        let coordinator_task = spawn_coordinator(
            drive.clone(),
            status.clone(),
            rebuild_rx,
            cancel.clone(),
            progress_sink.clone(),
        );
        // Trigger a full rebuild when either side of the index is
        // empty. Checking BM25 alone misses the case where a prior
        // rebuild was killed mid-graph-pass: the graph DB stays
        // empty (cancellation leaves it cleared, see Drive::reindex
        // doc) while BM25 still carries data from a much earlier
        // run, so without the graph check the server would never
        // notice and `/api/graph` would keep returning 0 nodes.
        let graph_empty = drive
            .graph()
            .and_then(|g| g.files().map(|fs| fs.is_empty()))
            .unwrap_or_else(|e| {
                tracing::warn!("indexer: initial graph check failed: {e}");
                false
            });
        if initial_build && (stats.indexed_docs == 0 || graph_empty) {
            // Best-effort: if the channel is full we already
            // queued a rebuild and the redundant request is fine
            // to drop.
            let _ = rebuild_tx.send(());
        }

        let watcher_task = spawn_watcher_loop(
            drive,
            status.clone(),
            watch_events,
            rebuild_tx.clone(),
            cancel.clone(),
        );

        Self {
            status,
            rebuild_tx,
            cancel,
            _watcher_task: watcher_task,
            _coordinator_task: coordinator_task,
        }
    }

    /// Signal an in-flight rebuild to bail. Idempotent. Safe to call
    /// from any task; takes effect on the rebuild's next per-file
    /// check.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Snapshot the current status. Cheap.
    pub fn snapshot(&self) -> IndexStatus {
        self.status.lock().unwrap().clone()
    }

    /// Ask the indexer to run a full rebuild. Returns immediately;
    /// the actual work runs on the blocking pool. The status flips
    /// to `Building` when the worker picks the request up.
    pub fn request_rebuild(&self) {
        // Channel unbounded; only the receiver-dropped variant
        // would error and at that point the indexer is gone.
        let _ = self.rebuild_tx.send(());
    }
}

/// Coordinator task: blocks on the rebuild channel and runs one
/// full reindex per request. Drives `Drive::reindex_with` with a
/// callback that updates the local status mutex AND forwards each
/// tick to the WS fan-out so the frontend's status pill animates
/// in real time. Without the WS forward we'd be polling
/// `/api/index/status` at a coarse cadence; with it we get every
/// per-file event.
fn spawn_coordinator(
    drive: Arc<Drive>,
    status: Arc<Mutex<IndexStatus>>,
    mut rx: mpsc::UnboundedReceiver<()>,
    cancel: Arc<AtomicBool>,
    progress_sink: Arc<dyn ProgressCallback>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            // Drain any extra requests that piled up so we run one
            // rebuild for the whole burst.
            while rx.try_recv().is_ok() {}
            if cancel.load(Ordering::Relaxed) {
                continue;
            }
            let drive_w = drive.clone();
            let status_w = status.clone();
            let cancel_w = cancel.clone();
            let progress_w = progress_sink.clone();
            *status_w.lock().unwrap() = IndexStatus::Building {
                current: 0,
                total: 0,
                file: String::new(),
            };
            let result = tokio::task::spawn_blocking(move || {
                let progress = StatusUpdater {
                    status: status_w,
                    forward: progress_w,
                };
                drive_w.reindex_with(Some(&cancel_w), &progress)
            })
            .await;
            match result {
                Ok(Ok(_summary)) => set_idle(&drive, &status),
                Ok(Err(chan_drive::ChanError::Cancelled)) => {
                    // Shutdown path: don't surface a user-visible
                    // error; the next boot will pick up the empty
                    // index and rebuild.
                    tracing::info!("indexer: rebuild cancelled");
                }
                Ok(Err(e)) => {
                    *status.lock().unwrap() = IndexStatus::Error {
                        message: e.to_string(),
                    };
                }
                Err(e) => {
                    *status.lock().unwrap() = IndexStatus::Error {
                        message: format!("rebuild task: {e}"),
                    };
                }
            }
        }
    })
}

/// Listen to the watcher and re-index per file with a 1 s debounce.
/// Multiple events for the same path inside the window collapse
/// into one re-index.
fn spawn_watcher_loop(
    drive: Arc<Drive>,
    status: Arc<Mutex<IndexStatus>>,
    mut rx: broadcast::Receiver<WatchEvent>,
    rebuild_tx: mpsc::UnboundedSender<()>,
    cancel: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let pending: Arc<Mutex<HashMap<String, PendingChange>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_w = pending.clone();
        let drive_w = drive.clone();
        let status_w = status.clone();
        let cancel_w = cancel.clone();

        // Worker: every 200 ms, drain paths whose last event is at
        // least 1 s in the past and apply them. We don't bound the
        // worker to the lifetime of the listener task: dropping the
        // Indexer aborts both join handles via tokio's task drop.
        let worker = tokio::spawn(async move {
            let debounce = Duration::from_secs(1);
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if cancel_w.load(Ordering::Relaxed) {
                    return;
                }
                let due = collect_due(&pending_w, debounce);
                for change in due {
                    *status_w.lock().unwrap() = IndexStatus::Reindexing {
                        file: change.path.clone(),
                    };
                    let drive2 = drive_w.clone();
                    let p = change.path.clone();
                    let deleted = change.deleted;
                    let result = tokio::task::spawn_blocking(move || {
                        apply_watch_change(&drive2, &p, deleted)
                    })
                    .await;
                    match result {
                        Ok(Ok(ApplyOutcome::Indexed)) => set_idle(&drive_w, &status_w),
                        Ok(Ok(ApplyOutcome::Forgotten)) => set_idle(&drive_w, &status_w),
                        Ok(Ok(ApplyOutcome::SkippedSpecial))
                        | Ok(Ok(ApplyOutcome::SkippedMissing)) => {
                            // Symlinks/FIFOs/sockets/devices and "the
                            // file was gone by the time we looked"
                            // are not index health signals. Drop
                            // back to Idle so the dashboard does
                            // not flash "search is broken" on a
                            // legitimate watcher event.
                            set_idle(&drive_w, &status_w);
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                path = %change.path,
                                error = %e,
                                "indexer: per-file apply failed"
                            );
                            *status_w.lock().unwrap() = IndexStatus::Error {
                                message: format!("{}: {e}", change.path),
                            };
                        }
                        Err(e) => {
                            *status_w.lock().unwrap() = IndexStatus::Error {
                                message: format!("join error: {e}"),
                            };
                        }
                    }
                }
            }
        });

        // Listener: feed `pending` from the watcher channel.
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(change) = relevant(&event) {
                        let mut p = pending.lock().unwrap();
                        let entry = p
                            .entry(change.path.clone())
                            .or_insert_with(|| change.clone());
                        // Latest event wins on the deleted flag: a
                        // create-then-delete burst should end as a
                        // delete.
                        entry.deleted = change.deleted;
                        entry.last_seen = Instant::now();
                    } else if matches!(event.kind, WatchKind::Renamed) {
                        // Rename surfaces as one event with both
                        // `path` (from) and `to` (destination).
                        // forget(from) + index(to). Two pending
                        // entries: one delete, one upsert.
                        if let Some(from) = event.path {
                            if from.ends_with(".md") {
                                pending.lock().unwrap().insert(
                                    from.clone(),
                                    PendingChange {
                                        path: from,
                                        deleted: true,
                                        last_seen: Instant::now(),
                                    },
                                );
                            }
                        }
                        if let Some(to) = event.to {
                            if to.ends_with(".md") {
                                pending.lock().unwrap().insert(
                                    to.clone(),
                                    PendingChange {
                                        path: to,
                                        deleted: false,
                                        last_seen: Instant::now(),
                                    },
                                );
                            }
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Dropped events; we've missed `n` of them. The
                    // safest catch-up is a full rebuild request,
                    // which the coordinator coalesces with anything
                    // already queued.
                    tracing::warn!(
                        "indexer: watcher channel lagged ({n} events); requesting rebuild"
                    );
                    let _ = rebuild_tx.send(());
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
        worker.abort();
    })
}

#[derive(Debug, Clone)]
struct PendingChange {
    path: String,
    deleted: bool,
    last_seen: Instant,
}

/// Result of applying one debounced watcher change. Distinguishes
/// real index updates from "the path was never indexable to begin
/// with" cases so the status reporter can stay calm. A user dropping
/// a symlink into their drive must not park the indexer in `Error`
/// forever (see syseng-1 hardening pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyOutcome {
    /// `Drive::index_file` succeeded.
    Indexed,
    /// `Drive::forget_file` succeeded (delete event, or cleanup for
    /// a vanished / replaced-by-symlink path).
    Forgotten,
    /// Path exists but is not a regular file (symlink, FIFO, socket,
    /// device, directory). The chan-drive walker already drops these
    /// from cold-boot indexing; the watch path mirrors that here.
    /// Any prior index entry for the path is best-effort cleared via
    /// `forget_file` in case a regular file was just replaced by a
    /// symlink.
    SkippedSpecial,
    /// Path no longer exists by the time we looked (typical for a
    /// quick create-then-delete burst). Same semantics as a Removed
    /// event: forget any prior index entry.
    SkippedMissing,
}

/// Per-file watch apply. Performs an explicit `std::fs::symlink_metadata`
/// check on the drive-relative path and dispatches accordingly.
///
/// Symmetric with `chan_drive::fs_ops::walk_drive_with` — the cold-
/// boot walker drops symlinks/specials, and this helper does the
/// same for the watch path. Without this gate a single user-created
/// symlink would surface `Drive::index_file`'s `SpecialFile` error
/// and stick `IndexStatus::Error` until something else indexed
/// successfully.
fn apply_watch_change(
    drive: &Drive,
    path: &str,
    deleted: bool,
) -> chan_drive::Result<ApplyOutcome> {
    if deleted {
        drive.forget_file(path)?;
        return Ok(ApplyOutcome::Forgotten);
    }
    let abs = match chan_drive::fs_ops::resolve_safe(drive.root(), path) {
        Ok(abs) => abs,
        Err(_) => return Ok(ApplyOutcome::SkippedMissing),
    };
    match std::fs::symlink_metadata(&abs) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            drive.index_file(path)?;
            Ok(ApplyOutcome::Indexed)
        }
        Ok(_) => {
            // Path exists but is not indexable. Drop any stale row
            // in case the path used to be a regular markdown file.
            // forget_file is tolerant of "no such row".
            let _ = drive.forget_file(path);
            Ok(ApplyOutcome::SkippedSpecial)
        }
        Err(_) => {
            // Vanished between the watcher event and our wake-up.
            let _ = drive.forget_file(path);
            Ok(ApplyOutcome::SkippedMissing)
        }
    }
}

/// Translate a `WatchEvent` into a markdown-only "rebuild this
/// file" task. Non-md paths and rename events (handled separately
/// by the caller because rename has both `path` and `to`) are
/// returned as None. Provider-level errors are logged and
/// dropped: the watcher channel itself stays subscribed; chan-core
/// recommends a full reindex in this case but we don't trigger one
/// today (TODO: wire a reindex on watcher loss).
fn relevant(event: &WatchEvent) -> Option<PendingChange> {
    if matches!(event.kind, WatchKind::ProviderError) {
        tracing::warn!(
            backend_message = ?event.path,
            "indexer: filesystem watcher reported a provider error; \
             search index may drift until the next manual reindex"
        );
        return None;
    }
    let path = event.path.as_deref()?;
    if !path.ends_with(".md") {
        return None;
    }
    match event.kind {
        WatchKind::Created | WatchKind::Modified => Some(PendingChange {
            path: path.to_owned(),
            deleted: false,
            last_seen: Instant::now(),
        }),
        WatchKind::Removed => Some(PendingChange {
            path: path.to_owned(),
            deleted: true,
            last_seen: Instant::now(),
        }),
        // Renamed: the caller fans out to forget(from) + index(to).
        WatchKind::Renamed => None,
        // Already handled at the top of the function; listed to
        // keep the match exhaustive on future variant additions.
        WatchKind::ProviderError => None,
    }
}

/// Pull paths whose last event is older than `window` and remove
/// them from the pending map.
fn collect_due(
    pending: &Mutex<HashMap<String, PendingChange>>,
    window: Duration,
) -> Vec<PendingChange> {
    let now = Instant::now();
    let mut p = pending.lock().unwrap();
    let due_paths: Vec<String> = p
        .iter()
        .filter(|(_, c)| now.duration_since(c.last_seen) >= window)
        .map(|(k, _)| k.clone())
        .collect();
    let mut out = Vec::with_capacity(due_paths.len());
    for k in due_paths {
        if let Some(v) = p.remove(&k) {
            out.push(v);
        }
    }
    out
}

/// `ProgressCallback` wrapper that mirrors progress events into two
/// places: the local `IndexStatus` mutex (so `/api/index/status`
/// reflects the in-flight build for clients that poll instead of
/// subscribing to /ws) AND a forwarded sink (the WS broadcast). The
/// status flips to `Building` on file / graph stages; other stages
/// (model load, contact import, reset) are forwarded to /ws but
/// don't override the indexer status — they live on their own
/// frontend surfaces.
struct StatusUpdater {
    status: Arc<Mutex<IndexStatus>>,
    forward: Arc<dyn ProgressCallback>,
}

impl ProgressCallback for StatusUpdater {
    fn on_progress(&self, event: ProgressEvent) {
        match event.stage {
            ProgressStage::GraphRebuild | ProgressStage::IndexFile => {
                let file = event.label.clone().unwrap_or_default();
                let current = event.current as usize;
                let total = event.total as usize;
                if let Ok(mut s) = self.status.lock() {
                    *s = IndexStatus::Building {
                        current,
                        total,
                        file,
                    };
                }
            }
            // Embed batch, model load, contact import, reset, rename
            // rewrite, heartbeat: WS subscribers see the event; the
            // local index status mutex stays where it is. Imports
            // have their own status field on the frontend (driven by
            // the import wizard); embed batches are part of an
            // already-Building indexer state.
            _ => {}
        }
        self.forward.on_progress(event);
    }
}

fn set_idle(drive: &Drive, status: &Mutex<IndexStatus>) {
    match drive.index_stats() {
        Ok(s) => {
            *status.lock().unwrap() = IndexStatus::Idle {
                indexed_docs: s.indexed_docs,
                indexed_vectors: s.indexed_vectors,
                model: s.model,
            };
        }
        Err(e) => {
            *status.lock().unwrap() = IndexStatus::Error {
                message: format!("stats: {e}"),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_drive::Library;
    use std::fs;
    use tempfile::TempDir;

    fn setup_drive() -> (TempDir, TempDir, Arc<Drive>) {
        let cfg = TempDir::new().unwrap();
        let drive_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(drive_dir.path(), None).unwrap();
        let drive = lib.open_drive(drive_dir.path()).unwrap();
        (cfg, drive_dir, drive)
    }

    #[test]
    fn apply_watch_change_indexes_regular_file() {
        let (_cfg, dir, drive) = setup_drive();
        fs::write(dir.path().join("a.md"), "# A\n\nbody\n").unwrap();
        let outcome = apply_watch_change(&drive, "a.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::Indexed);
    }

    #[test]
    fn apply_watch_change_forgets_on_delete_flag() {
        let (_cfg, _dir, drive) = setup_drive();
        let outcome = apply_watch_change(&drive, "gone.md", true).unwrap();
        assert_eq!(outcome, ApplyOutcome::Forgotten);
    }

    #[test]
    fn apply_watch_change_skips_missing_path() {
        let (_cfg, _dir, drive) = setup_drive();
        let outcome = apply_watch_change(&drive, "never-existed.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedMissing);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_skips_symlink_to_existing_target() {
        let (_cfg, dir, drive) = setup_drive();
        fs::write(dir.path().join("real.md"), "# Real\n").unwrap();
        std::os::unix::fs::symlink("real.md", dir.path().join("alias.md")).unwrap();
        let outcome = apply_watch_change(&drive, "alias.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedSpecial);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_skips_broken_symlink() {
        let (_cfg, dir, drive) = setup_drive();
        std::os::unix::fs::symlink("does-not-exist.md", dir.path().join("broken.md")).unwrap();
        let outcome = apply_watch_change(&drive, "broken.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedSpecial);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_skips_fifo() {
        // syseng-1 fixture had `attach/named.pipe`; the pre-fix
        // watch path called `index_file` on a FIFO and stuck
        // `IndexStatus::Error`. Probe with `mkfifo`; skip the
        // assertion if the binary is unavailable so test runs on
        // minimal containers stay green.
        let (_cfg, dir, drive) = setup_drive();
        let fifo_path = dir.path().join("attach.fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => return,
        }
        let outcome = apply_watch_change(&drive, "attach.fifo", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedSpecial);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_special_clears_prior_index_entry() {
        // Regression: if a user replaces a regular .md with a symlink
        // of the same name, the apply path should clean out the old
        // index row instead of leaving it stale.
        let (_cfg, dir, drive) = setup_drive();
        fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        assert_eq!(
            apply_watch_change(&drive, "a.md", false).unwrap(),
            ApplyOutcome::Indexed
        );
        let before = drive.index_stats().unwrap().indexed_docs;
        fs::remove_file(dir.path().join("a.md")).unwrap();
        fs::write(dir.path().join("real.md"), "# Real\n").unwrap();
        std::os::unix::fs::symlink("real.md", dir.path().join("a.md")).unwrap();
        assert_eq!(
            apply_watch_change(&drive, "a.md", false).unwrap(),
            ApplyOutcome::SkippedSpecial
        );
        // Best-effort cleanup ran: the prior `a.md` row is gone.
        let after = drive.index_stats().unwrap().indexed_docs;
        assert!(
            after < before,
            "expected indexed_docs to drop after symlink replacement; before={before} after={after}"
        );
    }
}
