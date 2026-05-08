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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chan_drive::{Drive, WatchEvent, WatchKind};
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
    /// chunks, kicks off a full rebuild on boot.
    pub fn spawn(
        drive: Arc<Drive>,
        watch_events: broadcast::Receiver<WatchEvent>,
        initial_build: bool,
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
        let (rebuild_tx, rebuild_rx) = mpsc::unbounded_channel::<()>();
        let coordinator_task = spawn_coordinator(drive.clone(), status.clone(), rebuild_rx);
        if initial_build && stats.indexed_docs == 0 {
            // Best-effort: if the channel is full we already
            // queued a rebuild and the redundant request is fine
            // to drop.
            let _ = rebuild_tx.send(());
        }

        let watcher_task =
            spawn_watcher_loop(drive, status.clone(), watch_events, rebuild_tx.clone());

        Self {
            status,
            rebuild_tx,
            _watcher_task: watcher_task,
            _coordinator_task: coordinator_task,
        }
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
/// full reindex per request. Updates the status mutex through
/// `Drive::reindex`'s own progress hook by polling stats after the
/// fact (chan-drive's reindex is one-shot; the per-file progress
/// callback path is a future enhancement).
fn spawn_coordinator(
    drive: Arc<Drive>,
    status: Arc<Mutex<IndexStatus>>,
    mut rx: mpsc::UnboundedReceiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            // Drain any extra requests that piled up so we run one
            // rebuild for the whole burst.
            while rx.try_recv().is_ok() {}
            let drive_w = drive.clone();
            let status_w = status.clone();
            *status_w.lock().unwrap() = IndexStatus::Building {
                current: 0,
                total: 0,
                file: String::new(),
            };
            let result = tokio::task::spawn_blocking(move || drive_w.reindex()).await;
            match result {
                Ok(Ok(_summary)) => set_idle(&drive, &status),
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
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let pending: Arc<Mutex<HashMap<String, PendingChange>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_w = pending.clone();
        let drive_w = drive.clone();
        let status_w = status.clone();

        // Worker: every 200 ms, drain paths whose last event is at
        // least 1 s in the past and apply them. We don't bound the
        // worker to the lifetime of the listener task: dropping the
        // Indexer aborts both join handles via tokio's task drop.
        let worker = tokio::spawn(async move {
            let debounce = Duration::from_secs(1);
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let due = collect_due(&pending_w, debounce);
                for change in due {
                    *status_w.lock().unwrap() = IndexStatus::Reindexing {
                        file: change.path.clone(),
                    };
                    let drive2 = drive_w.clone();
                    let p = change.path.clone();
                    let result = if change.deleted {
                        tokio::task::spawn_blocking(move || drive2.forget_file(&p)).await
                    } else {
                        tokio::task::spawn_blocking(move || drive2.index_file(&p)).await
                    };
                    match result {
                        Ok(Ok(())) => set_idle(&drive_w, &status_w),
                        Ok(Err(e)) => {
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

/// Translate a `WatchEvent` into a markdown-only "rebuild this
/// file" task. Non-md paths and rename events (handled separately
/// by the caller because rename has both `path` and `to`) are
/// returned as None.
fn relevant(event: &WatchEvent) -> Option<PendingChange> {
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
