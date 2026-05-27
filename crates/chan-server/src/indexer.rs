// Background indexer driven by the existing watcher bridge.
//
// Two responsibilities:
//
//   1. On server start, kick off a full `Workspace::reindex` if the
//      workspace's index is empty (cold workspace / fresh schema bump).
//      Runs on the tokio blocking pool so the rest of `chan serve`
//      keeps responding.
//   2. Subscribe to the watcher's `WatchEvent` broadcast and
//      debounce per-path file changes into incremental
//      `Workspace::index_file` / `Workspace::forget_file` calls.
//
// Status is exposed through a `Mutex<IndexStatus>` snapshot the
// `/api/index/status` endpoint reads. We deliberately don't push
// status over the WS in v1: polling the status endpoint every few
// seconds while the user is on the Settings panel is simpler and
// the payload is tiny.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chan_workspace::{
    ProgressCallback, ProgressEvent, ProgressStage, SearchAggression, VcsKind, WatchEvent,
    WatchKind, Workspace,
};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

const VCS_BURST_REBUILD_THRESHOLD: usize = 64;

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
    /// Steady state. Counters mirror `Workspace::index_stats`.
    Idle {
        indexed_docs: u64,
        indexed_vectors: u64,
        model: String,
    },
    /// The last operation failed; users are still allowed to query
    /// (over the previous index state).
    Error { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexerHealthStatus {
    Idle,
    Settling,
    Rebuilding,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IndexerHealth {
    pub status: IndexerHealthStatus,
    pub queue_depth: usize,
    pub last_event_at: Option<i64>,
    pub last_settled_at: Option<i64>,
    pub coalesced_rebuild: bool,
}

#[derive(Debug)]
struct IndexerTelemetry {
    queue_depth: usize,
    last_event_at: Option<i64>,
    last_settled_at: Option<i64>,
    coalesced_rebuild: bool,
}

#[derive(Clone)]
struct IndexerShared {
    status: Arc<Mutex<IndexStatus>>,
    telemetry: Arc<Mutex<IndexerTelemetry>>,
}

/// Handle to the background indexer. Drop it (or call `shutdown`)
/// to stop both the watcher loop and the in-flight initial build.
pub struct Indexer {
    status: Arc<Mutex<IndexStatus>>,
    telemetry: Arc<Mutex<IndexerTelemetry>>,
    rebuild_tx: mpsc::UnboundedSender<()>,
    /// Set to true on shutdown so the in-flight `Workspace::reindex`
    /// blocking task bails at its next per-file check. Without this
    /// the runtime drop after `serve()` returns would have to wait
    /// for the rebuild to finish naturally; on a large workspace that's
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

impl Drop for Indexer {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self._watcher_task.abort();
        self._coordinator_task.abort();
    }
}

impl Indexer {
    /// Spawn the indexer over `workspace`, tied to `watch_events`. If
    /// `initial_build` is true and the workspace's index reports zero
    /// chunks, kicks off a full rebuild on boot. `progress_sink` is
    /// the WS fan-out (see `bus::make_progress_broadcast`); per-file
    /// progress events forward there in addition to updating the
    /// local `IndexStatus` mutex behind `/api/index/status`.
    pub fn spawn(
        workspace: Arc<Workspace>,
        watch_events: broadcast::Receiver<WatchEvent>,
        initial_build: bool,
        search_aggression: SearchAggression,
        progress_sink: Arc<dyn ProgressCallback>,
    ) -> Self {
        let stats = workspace.index_stats().unwrap_or_else(|e| {
            tracing::warn!("indexer: initial stats failed: {e}");
            chan_workspace::IndexStats {
                ready: false,
                indexed_docs: 0,
                indexed_vectors: 0,
                model: chan_workspace::DEFAULT_MODEL.to_owned(),
            }
        });
        let status = Arc::new(Mutex::new(IndexStatus::Idle {
            indexed_docs: stats.indexed_docs,
            indexed_vectors: stats.indexed_vectors,
            model: stats.model.clone(),
        }));
        let telemetry = Arc::new(Mutex::new(IndexerTelemetry {
            queue_depth: 0,
            last_event_at: None,
            last_settled_at: Some(now_unix()),
            coalesced_rebuild: false,
        }));
        let watch_context = WatchContext {
            vcs_kind: chan_workspace::detect_workspace_vcs(workspace.root()),
        };

        // Coordinator task: serializes "rebuild now" requests so
        // the watcher loop and the on-boot trigger can't both ask
        // for a full rebuild concurrently. Listening on an
        // unbounded mpsc since the bursts are tiny (one or two
        // requests per session) and dropping a request would just
        // leave the index stale.
        let cancel = Arc::new(AtomicBool::new(false));
        let (rebuild_tx, rebuild_rx) = mpsc::unbounded_channel::<()>();
        let workspace_weak = Arc::downgrade(&workspace);
        let coordinator_task = spawn_coordinator(
            workspace_weak.clone(),
            status.clone(),
            telemetry.clone(),
            rebuild_rx,
            cancel.clone(),
            search_aggression,
            progress_sink.clone(),
        );
        // Trigger a full rebuild when either side of the index is
        // empty. Checking BM25 alone misses the case where a prior
        // rebuild was killed mid-graph-pass: the graph DB stays
        // empty (cancellation leaves it cleared, see Workspace::reindex
        // doc) while BM25 still carries data from a much earlier
        // run, so without the graph check the server would never
        // notice and `/api/graph` would keep returning 0 nodes.
        let graph_empty = workspace
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
        } else if initial_build {
            // systacean-37: when the full reindex DOESN'T fire
            // (workspace root is non-empty + graph is non-empty),
            // the `-34` drafts walker inside `Workspace::reindex`
            // never runs at boot. Drafts content authored
            // pre-`-36` (when watcher events were silently
            // dropped) stays absent from BM25 + graph. Walk
            // drafts unconditionally on every boot; idempotent
            // (`index_draft_file` overwrites both backends) and
            // O(N) per draft so the cost is bounded by how
            // many drafts the user keeps around. Runs on the
            // blocking pool so a slow drafts subtree doesn't
            // stall the rest of `Indexer::spawn`.
            let workspace_for_drafts = workspace_weak.clone();
            tokio::task::spawn_blocking(move || {
                let Some(workspace_for_drafts) = workspace_for_drafts.upgrade() else {
                    return;
                };
                if let Err(e) = workspace_for_drafts.index_drafts_subtree() {
                    tracing::warn!(
                        error = %e,
                        "indexer: drafts boot walk failed; drafts may be missing from BM25/graph until next save"
                    );
                }
            });
        }

        let watcher_task = spawn_watcher_loop(
            workspace_weak,
            IndexerShared {
                status: status.clone(),
                telemetry: telemetry.clone(),
            },
            watch_events,
            rebuild_tx.clone(),
            cancel.clone(),
            search_aggression,
            watch_context,
        );

        Self {
            status,
            telemetry,
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

    /// Snapshot the lightweight health view used by `/api/health`.
    pub fn health_snapshot(&self) -> IndexerHealth {
        let status = self.status.lock().unwrap().clone();
        let telemetry = self.telemetry.lock().unwrap();
        health_from(&status, &telemetry)
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
/// full reindex per request. Workspaces `Workspace::reindex_with` with a
/// callback that updates the local status mutex AND forwards each
/// tick to the WS fan-out so the frontend's status pill animates
/// in real time. Without the WS forward we'd be polling
/// `/api/index/status` at a coarse cadence; with it we get every
/// per-file event.
fn spawn_coordinator(
    workspace: Weak<Workspace>,
    status: Arc<Mutex<IndexStatus>>,
    telemetry: Arc<Mutex<IndexerTelemetry>>,
    mut rx: mpsc::UnboundedReceiver<()>,
    cancel: Arc<AtomicBool>,
    search_aggression: SearchAggression,
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
            let Some(workspace_w) = workspace.upgrade() else {
                break;
            };
            let status_w = status.clone();
            let cancel_w = cancel.clone();
            let progress_w = progress_sink.clone();
            let aggression = search_aggression;
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
                workspace_w.reindex_with_aggression(Some(&cancel_w), &progress, aggression)
            })
            .await;
            // Bug 9: every resolution of a build MUST move the status
            // out of `Building`, or the status pill is stuck forever
            // (it hides only on `Idle`). The success and cancel arms
            // both reconcile to `Idle` against the live index stats:
            // a cancelled rebuild leaves whatever committed, and the
            // honest steady-state is "idle showing what's indexed",
            // not a frozen progress counter. The error arms set
            // `Error`. The only way to stay `Building` now is an
            // in-flight build that has genuinely not resolved.
            match result {
                Ok(Ok(_summary)) => {
                    reconcile_idle(&workspace, &status, &telemetry);
                }
                Ok(Err(chan_workspace::ChanError::Cancelled)) => {
                    // Shutdown / reset path: don't surface a
                    // user-visible error; the next boot picks up the
                    // (possibly empty) index and rebuilds. Still clear
                    // the pill so a cancel that leaves the process
                    // running does not park `Building` forever.
                    tracing::info!("indexer: rebuild cancelled");
                    reconcile_idle(&workspace, &status, &telemetry);
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
    workspace: Weak<Workspace>,
    shared: IndexerShared,
    mut rx: broadcast::Receiver<WatchEvent>,
    rebuild_tx: mpsc::UnboundedSender<()>,
    cancel: Arc<AtomicBool>,
    search_aggression: SearchAggression,
    watch_context: WatchContext,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let pending: Arc<Mutex<HashMap<String, PendingChange>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_w = pending.clone();
        let workspace_w = workspace.clone();
        let status_w = shared.status.clone();
        let telemetry_w = shared.telemetry.clone();
        let cancel_w = cancel.clone();

        // Worker: every 200 ms, drain paths whose last event is at
        // least the configured debounce in the past and apply them.
        // If the listener task is aborted, this worker exits on the
        // shared cancel flag and only holds a weak workspace reference.
        let worker = tokio::spawn(async move {
            let debounce = search_aggression.debounce();
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if cancel_w.load(Ordering::Relaxed) {
                    return;
                }
                let due = collect_due(&pending_w, debounce);
                update_queue_depth(&pending_w, &telemetry_w);
                for change in due {
                    *status_w.lock().unwrap() = IndexStatus::Reindexing {
                        file: change.path.clone(),
                    };
                    let Some(workspace2) = workspace_w.upgrade() else {
                        return;
                    };
                    let p = change.path.clone();
                    let deleted = change.deleted;
                    let result = tokio::task::spawn_blocking(move || {
                        apply_watch_change(&workspace2, &p, deleted)
                    })
                    .await;
                    match result {
                        Ok(Ok(ApplyOutcome::Indexed)) => {
                            if let Some(workspace) = workspace_w.upgrade() {
                                set_idle(&workspace, &status_w, &telemetry_w)
                            } else {
                                return;
                            }
                        }
                        Ok(Ok(ApplyOutcome::Forgotten)) => {
                            if let Some(workspace) = workspace_w.upgrade() {
                                set_idle(&workspace, &status_w, &telemetry_w)
                            } else {
                                return;
                            }
                        }
                        Ok(Ok(ApplyOutcome::SkippedSpecial))
                        | Ok(Ok(ApplyOutcome::SkippedMissing)) => {
                            // Symlinks/FIFOs/sockets/devices and "the
                            // file was gone by the time we looked"
                            // are not index health signals. Drop
                            // back to Idle so the dashboard does
                            // not flash "search is broken" on a
                            // legitimate watcher event.
                            if let Some(workspace) = workspace_w.upgrade() {
                                set_idle(&workspace, &status_w, &telemetry_w);
                            } else {
                                return;
                            }
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
                    record_watcher_event(&shared.telemetry);
                    match classify_watch_event(&event, watch_context) {
                        WatchAction::Changes(changes) => {
                            let mut p = pending.lock().unwrap();
                            for change in changes {
                                let entry = p
                                    .entry(change.path.clone())
                                    .or_insert_with(|| change.clone());
                                // Latest event wins on the deleted flag:
                                // a create-then-delete burst should end
                                // as a delete.
                                entry.deleted = change.deleted;
                                entry.last_seen = change.last_seen;
                            }
                            if should_rebuild_for_vcs_burst(watch_context, p.len()) {
                                p.clear();
                                mark_coalesced_rebuild(&shared.telemetry);
                                tracing::warn!(
                                threshold = VCS_BURST_REBUILD_THRESHOLD,
                                "indexer: VCS-aware watcher burst exceeded threshold; requesting rebuild"
                            );
                                let _ = rebuild_tx.send(());
                            }
                            drop(p);
                            update_queue_depth(&pending, &shared.telemetry);
                        }
                        WatchAction::Rebuild { reason } => {
                            pending.lock().unwrap().clear();
                            mark_coalesced_rebuild(&shared.telemetry);
                            update_queue_depth(&pending, &shared.telemetry);
                            tracing::warn!(
                                reason,
                                "indexer: watcher event stream lost scope; requesting rebuild"
                            );
                            let _ = rebuild_tx.send(());
                        }
                        WatchAction::Ignore => {}
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Dropped events; we've missed `n` of them. The
                    // safest catch-up is a full rebuild request,
                    // which the coordinator coalesces with anything
                    // already queued.
                    mark_coalesced_rebuild(&shared.telemetry);
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingChange {
    path: String,
    deleted: bool,
    last_seen: Instant,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WatchContext {
    vcs_kind: Option<VcsKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WatchAction {
    Changes(Vec<PendingChange>),
    Rebuild { reason: &'static str },
    Ignore,
}

/// Result of applying one debounced watcher change. Distinguishes
/// real index updates from "the path was never indexable to begin
/// with" cases so the status reporter can stay calm. A user dropping
/// a symlink into their workspace must not park the indexer in `Error`
/// forever (see syseng-1 hardening pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyOutcome {
    /// `Workspace::index_file` succeeded.
    Indexed,
    /// `Workspace::forget_file` succeeded (delete event, or cleanup for
    /// a vanished / replaced-by-symlink path).
    Forgotten,
    /// Path exists but is not a regular file (symlink, FIFO, socket,
    /// device, directory). The chan-workspace walker already drops these
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
/// check on the workspace-relative path and dispatches accordingly.
///
/// Symmetric with `chan_workspace::fs_ops::walk_workspace_with`; the cold-
/// boot walker drops symlinks/specials, and this helper does the
/// same for the watch path. Without this gate a single user-created
/// symlink would surface `Workspace::index_file`'s `SpecialFile` error
/// and stick `IndexStatus::Error` until something else indexed
/// successfully.
fn apply_watch_change(
    workspace: &Workspace,
    path: &str,
    deleted: bool,
) -> chan_workspace::Result<ApplyOutcome> {
    if deleted {
        workspace.forget_file(path)?;
        return Ok(ApplyOutcome::Forgotten);
    }
    // systacean-36: route `Drafts/`-prefixed paths through the
    // drafts cap-std handle via `Workspace::index_draft_file`. The
    // chan-workspace watcher (`-25`) emits drafts events with the
    // `Drafts/` prefix already applied; without this branch the
    // `resolve_safe(workspace.root(), ...)` below would error
    // (drafts dir is at `<state>/drafts/<uuid>/`, NOT under workspace
    // root) + the event would be silently dropped; the root
    // cause of the recurring `-a-66 slice e` PARTIAL despite
    // `-34`'s boot walker.
    if let Some(sub) = path.strip_prefix("Drafts/") {
        if sub.is_empty() {
            // `Drafts/` itself (root of the subtree). Nothing to
            // index; same SkippedSpecial path as for non-file
            // events under workspace root.
            return Ok(ApplyOutcome::SkippedSpecial);
        }
        let abs = workspace.drafts_dir().join(sub);
        match std::fs::symlink_metadata(&abs) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                workspace.index_draft_file(path)?;
                return Ok(ApplyOutcome::Indexed);
            }
            Ok(_) => {
                let _ = workspace.forget_file(path);
                return Ok(ApplyOutcome::SkippedSpecial);
            }
            Err(_) => {
                let _ = workspace.forget_file(path);
                return Ok(ApplyOutcome::SkippedMissing);
            }
        }
    }
    let abs = match chan_workspace::fs_ops::resolve_safe(workspace.root(), path) {
        Ok(abs) => abs,
        Err(_) => return Ok(ApplyOutcome::SkippedMissing),
    };
    match std::fs::symlink_metadata(&abs) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            workspace.index_file(path)?;
            Ok(ApplyOutcome::Indexed)
        }
        Ok(_) => {
            // Path exists but is not indexable. Drop any stale row
            // in case the path used to be a regular markdown file.
            // forget_file is tolerant of "no such row".
            let _ = workspace.forget_file(path);
            Ok(ApplyOutcome::SkippedSpecial)
        }
        Err(_) => {
            // Vanished between the watcher event and our wake-up.
            let _ = workspace.forget_file(path);
            Ok(ApplyOutcome::SkippedMissing)
        }
    }
}

/// Translate a watcher event into indexer work. `Workspace::watch` has
/// already warmed chan-report and runs its report fan-out before the
/// event reaches this scheduler; full rebuilds run graph-first inside
/// `Workspace::reindex_with`, so provider-loss recovery preserves the
/// graph/report-before-search priority boundary.
fn classify_watch_event(event: &WatchEvent, context: WatchContext) -> WatchAction {
    if context.vcs_kind.is_some() && watch_event_touches_vcs_control(event) {
        return WatchAction::Rebuild {
            reason: "vcs-control",
        };
    }
    let now = Instant::now();
    match event.kind {
        WatchKind::ProviderError => WatchAction::Rebuild {
            reason: "provider-error",
        },
        WatchKind::Created | WatchKind::Modified | WatchKind::Removed => {
            let Some(path) = event.path.as_deref() else {
                // macOS FSEvents can emit ordinary path-less
                // create/modify/remove notifications during metadata
                // churn. ProviderError and channel lag are the actual
                // loss-of-scope signals; rebuilding here makes normal
                // Rich Prompt workspace activity look broken.
                return WatchAction::Ignore;
            };
            if !chan_workspace::fs_ops::is_indexable_text(path) {
                return WatchAction::Ignore;
            }
            WatchAction::Changes(vec![PendingChange {
                path: path.to_owned(),
                deleted: matches!(event.kind, WatchKind::Removed),
                last_seen: now,
            }])
        }
        WatchKind::Renamed => {
            let mut changes = Vec::with_capacity(2);
            if let Some(from) = event.path.as_deref() {
                if chan_workspace::fs_ops::is_indexable_text(from) {
                    changes.push(PendingChange {
                        path: from.to_owned(),
                        deleted: true,
                        last_seen: now,
                    });
                }
            }
            if let Some(to) = event.to.as_deref() {
                if chan_workspace::fs_ops::is_indexable_text(to) {
                    changes.push(PendingChange {
                        path: to.to_owned(),
                        deleted: false,
                        last_seen: now,
                    });
                }
            }
            if changes.is_empty() {
                WatchAction::Ignore
            } else {
                WatchAction::Changes(changes)
            }
        }
    }
}

fn watch_event_touches_vcs_control(event: &WatchEvent) -> bool {
    event
        .path
        .as_deref()
        .is_some_and(chan_workspace::is_vcs_control_path)
        || event
            .to
            .as_deref()
            .is_some_and(chan_workspace::is_vcs_control_path)
}

fn should_rebuild_for_vcs_burst(context: WatchContext, pending_len: usize) -> bool {
    context.vcs_kind.is_some() && pending_len >= VCS_BURST_REBUILD_THRESHOLD
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
    // Deletions first: stale graph/search rows disappear before any
    // upserts from the same burst add new rows.
    out.sort_by_key(|c| !c.deleted);
    out
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn health_from(status: &IndexStatus, telemetry: &IndexerTelemetry) -> IndexerHealth {
    let status = match status {
        IndexStatus::Error { .. } => IndexerHealthStatus::Error,
        IndexStatus::Building { .. } | IndexStatus::Reindexing { .. } => {
            IndexerHealthStatus::Rebuilding
        }
        IndexStatus::Idle { .. } if telemetry.queue_depth > 0 => IndexerHealthStatus::Settling,
        IndexStatus::Idle { .. } if telemetry.coalesced_rebuild => IndexerHealthStatus::Rebuilding,
        IndexStatus::Idle { .. } => IndexerHealthStatus::Idle,
    };
    IndexerHealth {
        status,
        queue_depth: telemetry.queue_depth,
        last_event_at: telemetry.last_event_at,
        last_settled_at: telemetry.last_settled_at,
        coalesced_rebuild: telemetry.coalesced_rebuild,
    }
}

fn record_watcher_event(telemetry: &Mutex<IndexerTelemetry>) {
    telemetry.lock().unwrap().last_event_at = Some(now_unix());
}

fn mark_coalesced_rebuild(telemetry: &Mutex<IndexerTelemetry>) {
    let mut telemetry = telemetry.lock().unwrap();
    telemetry.coalesced_rebuild = true;
    telemetry.queue_depth = 0;
}

fn update_queue_depth(
    pending: &Mutex<HashMap<String, PendingChange>>,
    telemetry: &Mutex<IndexerTelemetry>,
) {
    telemetry.lock().unwrap().queue_depth = pending.lock().unwrap().len();
}

/// `ProgressCallback` wrapper that mirrors progress events into two
/// places: the local `IndexStatus` mutex (so `/api/index/status`
/// reflects the in-flight build for clients that poll instead of
/// subscribing to /ws) AND a forwarded sink (the WS broadcast). The
/// status flips to `Building` on file / graph stages; other stages
/// (model load, contact import, reset) are forwarded to /ws but
/// don't override the indexer status; they live on their own
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
            // Bug 9: the embed phase runs AFTER every IndexFile tick has
            // fired (read+chunk+BM25-enqueue first, then the cross-file
            // embedding flushes, then the final commit). Without
            // reflecting EmbedBatch here the pill froze on the last
            // IndexFile label (`total-1/total`) for the entire embed
            // phase, which on a large workspace is minutes and looks exactly
            // like a stuck reindex. Surface the embed progress so the
            // pill keeps moving until the build actually finishes and
            // `set_idle` clears it. `current/total` here are chunk
            // counts within a batch; the label carries the file context
            // (`files=N last=...`). We keep `total` as the batch's
            // chunk total so the bar reads as ongoing work rather than
            // a frozen file counter.
            ProgressStage::EmbedBatch => {
                // The pill template wraps `file` in parens, so keep the
                // embed label a bare phrase ("embedding") rather than
                // re-wrapping the producer's `files=N last=...` blob
                // (which would render as nested parens). The chunk
                // counters still animate the bar; the phrase tells the
                // user this is the embed phase, not a stuck file
                // counter.
                let current = event.current as usize;
                let total = event.total as usize;
                if let Ok(mut s) = self.status.lock() {
                    *s = IndexStatus::Building {
                        current,
                        total,
                        file: "embedding".to_owned(),
                    };
                }
            }
            // Model load, contact import, reset, rename rewrite,
            // heartbeat: WS subscribers see the event; the local index
            // status mutex stays where it is. Imports have their own
            // status field on the frontend (driven by the import
            // wizard).
            _ => {}
        }
        self.forward.on_progress(event);
    }
}

/// Bug 9 clear-path helper for the coordinator: move the status out of
/// `Building` when a rebuild resolves, whether or not the workspace `Weak`
/// still upgrades. With a live workspace this reads fresh stats via
/// `set_idle`. If the workspace was dropped (reset/shutdown swapped the
/// cell), there is nothing to query, but we still must not leave the
/// pill frozen on `Building` for the brief window before the indexer
/// itself is dropped, so we stamp a zeroed idle. Either way the pill
/// hides (it is visible only on non-idle states).
fn reconcile_idle(
    workspace: &Weak<Workspace>,
    status: &Mutex<IndexStatus>,
    telemetry: &Mutex<IndexerTelemetry>,
) {
    match workspace.upgrade() {
        Some(workspace) => set_idle(&workspace, status, telemetry),
        None => {
            if let Ok(mut s) = status.lock() {
                *s = IndexStatus::Idle {
                    indexed_docs: 0,
                    indexed_vectors: 0,
                    model: chan_workspace::DEFAULT_MODEL.to_owned(),
                };
            }
        }
    }
}

fn set_idle(
    workspace: &Workspace,
    status: &Mutex<IndexStatus>,
    telemetry: &Mutex<IndexerTelemetry>,
) {
    match workspace.index_stats() {
        Ok(s) => {
            *status.lock().unwrap() = IndexStatus::Idle {
                indexed_docs: s.indexed_docs,
                indexed_vectors: s.indexed_vectors,
                model: s.model,
            };
            let mut telemetry = telemetry.lock().unwrap();
            telemetry.last_settled_at = Some(now_unix());
            telemetry.coalesced_rebuild = false;
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
    use chan_workspace::{Library, SearchMode, SearchOpts};
    use std::fs;
    use tempfile::TempDir;

    /// Well-known lock-file name (under the OS temp dir) for the
    /// cross-process FS-timing test gate. MUST stay identical to
    /// `chan_workspace::test_gate::GATE_FILE` and the copy in the terminal
    /// test module so every FS-timing test across both crates' separate
    /// test binaries contends on the same OS advisory lock.
    const FS_TIMING_GATE: &str = "chan-fs-timing-test.gate";

    /// Cross-process serial gate for the real-FS boot-walk tests. Each
    /// spins a fresh `Indexer` whose boot walk is a `spawn_blocking`
    /// re-index + a Tantivy commit + a reader refresh, then polls BM25
    /// for the result. Under the FULL parallel `cargo test` run (CI)
    /// every core is saturated, so the boot walk's turn on the CPU and
    /// the commit/refresh cycle slip past a tight poll deadline and the
    /// tests flake.
    ///
    /// WHY a FILE lock and not a `static`/`tokio` Mutex: a `static` lock
    /// serializes only tests WITHIN this test binary, but `cargo test`
    /// runs each crate's test binary as a SEPARATE PROCESS concurrently,
    /// so these boot-walk tests still race chan-workspace's FS-watcher tests
    /// and this crate's PTY tests for the CPU + the kernel FSEvent queue.
    /// An OS advisory lock on a well-known temp path is the one primitive
    /// that spans process boundaries; the SAME `FS_TIMING_GATE` path is
    /// opened in chan-workspace (`crate::test_gate`) + the terminal test
    /// module, so a single named gate serializes the entire FS-timing
    /// class workspace-wide. The `std::fs::File` guard is `Send` (held
    /// across `.await` on the multi-thread runtime is fine) and releases
    /// on drop / process exit.
    fn boot_walk_test_lock() -> std::fs::File {
        let path = std::env::temp_dir().join(FS_TIMING_GATE);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .expect("open FS-timing test gate file");
        file.lock().expect("acquire FS-timing test gate");
        file
    }

    /// Poll budget for the real-FS boot-walk tests. On an idle host the
    /// boot walk completes and BM25 is queryable in well under a second,
    /// so this ceiling is never approached; it only governs the worst
    /// case under the full parallel suite, where the `spawn_blocking`
    /// walk + commit + reader refresh can be delayed by seconds under
    /// CPU contention. The cross-process `boot_walk_test_lock` gate is
    /// the primary fix (it removes the competing FS-timing load); this
    /// budget is the backstop and should rarely be approached now.
    const BOOT_WALK_BUDGET: Duration = Duration::from_secs(30);

    /// Poll a closure until it returns true or `timeout` elapses, yielding
    /// to the runtime between checks. Returns true on success. The boot
    /// walk is asynchronous (`spawn_blocking` + commit + reader refresh);
    /// tests need a bounded wait rather than a fixed sleep so they pass
    /// quickly on fast hosts and tolerate slower CI.
    async fn poll_until(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if check() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        check()
    }

    fn setup_workspace() -> (TempDir, TempDir, Arc<Workspace>) {
        let cfg = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();
        let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(workspace_dir.path()).unwrap();
        let workspace = lib.open_workspace(workspace_dir.path()).unwrap();
        (cfg, workspace_dir, workspace)
    }

    fn ev(kind: WatchKind, path: Option<&str>, to: Option<&str>) -> WatchEvent {
        WatchEvent {
            kind,
            path: path.map(str::to_owned),
            to: to.map(str::to_owned),
        }
    }

    fn classify(event: &WatchEvent) -> WatchAction {
        classify_watch_event(event, WatchContext::default())
    }

    fn classify_vcs(event: &WatchEvent) -> WatchAction {
        classify_watch_event(
            event,
            WatchContext {
                vcs_kind: Some(VcsKind::Git),
            },
        )
    }

    #[test]
    fn classify_watch_event_uses_chan_workspace_indexable_gate() {
        match classify(&ev(WatchKind::Modified, Some("notes/a.txt"), None)) {
            WatchAction::Changes(changes) => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].path, "notes/a.txt");
                assert!(!changes[0].deleted);
            }
            other => panic!("expected .txt change, got {other:?}"),
        }

        assert!(matches!(
            classify(&ev(WatchKind::Modified, Some("src/lib.rs"), None)),
            WatchAction::Ignore
        ));
    }

    #[test]
    fn classify_watch_event_requests_rebuild_on_provider_loss() {
        assert!(matches!(
            classify(&ev(WatchKind::ProviderError, Some("overflow"), None)),
            WatchAction::Rebuild {
                reason: "provider-error"
            }
        ));
    }

    #[test]
    fn classify_watch_event_ignores_pathless_non_provider_noise() {
        assert!(matches!(
            classify(&ev(WatchKind::Modified, None, None)),
            WatchAction::Ignore
        ));
        assert!(matches!(
            classify(&ev(WatchKind::Renamed, None, None)),
            WatchAction::Ignore
        ));
    }

    #[test]
    fn classify_watch_event_splits_indexable_rename() {
        match classify(&ev(WatchKind::Renamed, Some("old.md"), Some("new.txt"))) {
            WatchAction::Changes(changes) => {
                assert_eq!(changes.len(), 2);
                assert_eq!(changes[0].path, "old.md");
                assert!(changes[0].deleted);
                assert_eq!(changes[1].path, "new.txt");
                assert!(!changes[1].deleted);
            }
            other => panic!("expected rename changes, got {other:?}"),
        }
    }

    #[test]
    fn classify_watch_event_requests_rebuild_on_vcs_control_paths() {
        assert!(matches!(
            classify_vcs(&ev(WatchKind::Modified, Some(".git/HEAD"), None)),
            WatchAction::Rebuild {
                reason: "vcs-control"
            }
        ));
        assert!(matches!(
            classify_vcs(&ev(WatchKind::Renamed, Some("tmp"), Some(".hg/dirstate"))),
            WatchAction::Rebuild {
                reason: "vcs-control"
            }
        ));
        assert!(matches!(
            classify(&ev(WatchKind::Modified, Some(".git/HEAD"), None)),
            WatchAction::Ignore
        ));
    }

    #[test]
    fn vcs_burst_threshold_only_applies_to_vcs_aware_workspaces() {
        assert!(!should_rebuild_for_vcs_burst(
            WatchContext::default(),
            VCS_BURST_REBUILD_THRESHOLD
        ));
        assert!(!should_rebuild_for_vcs_burst(
            WatchContext {
                vcs_kind: Some(VcsKind::Git),
            },
            VCS_BURST_REBUILD_THRESHOLD - 1,
        ));
        assert!(should_rebuild_for_vcs_burst(
            WatchContext {
                vcs_kind: Some(VcsKind::Git),
            },
            VCS_BURST_REBUILD_THRESHOLD,
        ));
    }

    #[test]
    fn collect_due_applies_deletions_before_upserts() {
        let pending = Mutex::new(HashMap::from([
            (
                "new.md".to_string(),
                PendingChange {
                    path: "new.md".to_string(),
                    deleted: false,
                    last_seen: Instant::now() - Duration::from_secs(2),
                },
            ),
            (
                "old.md".to_string(),
                PendingChange {
                    path: "old.md".to_string(),
                    deleted: true,
                    last_seen: Instant::now() - Duration::from_secs(2),
                },
            ),
        ]));

        let due = collect_due(&pending, Duration::from_secs(1));
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].path, "old.md");
        assert!(due[0].deleted);
        assert_eq!(due[1].path, "new.md");
        assert!(!due[1].deleted);
    }

    #[test]
    fn health_snapshot_reports_settling_and_rebuilding_transitions() {
        let idle = IndexStatus::Idle {
            indexed_docs: 3,
            indexed_vectors: 0,
            model: "bm25".to_string(),
        };
        let mut telemetry = IndexerTelemetry {
            queue_depth: 0,
            last_event_at: None,
            last_settled_at: Some(10),
            coalesced_rebuild: false,
        };
        assert_eq!(
            health_from(&idle, &telemetry).status,
            IndexerHealthStatus::Idle
        );

        telemetry.queue_depth = 2;
        telemetry.last_event_at = Some(11);
        assert_eq!(
            health_from(&idle, &telemetry),
            IndexerHealth {
                status: IndexerHealthStatus::Settling,
                queue_depth: 2,
                last_event_at: Some(11),
                last_settled_at: Some(10),
                coalesced_rebuild: false,
            }
        );

        telemetry.queue_depth = 0;
        telemetry.coalesced_rebuild = true;
        assert_eq!(
            health_from(&idle, &telemetry).status,
            IndexerHealthStatus::Rebuilding
        );
        assert_eq!(
            health_from(
                &IndexStatus::Reindexing {
                    file: "note.md".to_string()
                },
                &telemetry
            )
            .status,
            IndexerHealthStatus::Rebuilding
        );
    }

    #[test]
    fn apply_watch_change_indexes_regular_file() {
        let (_cfg, dir, workspace) = setup_workspace();
        fs::write(dir.path().join("a.md"), "# A\n\nbody\n").unwrap();
        let outcome = apply_watch_change(&workspace, "a.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::Indexed);
    }

    fn progress_event(
        stage: ProgressStage,
        current: u64,
        total: u64,
        label: &str,
    ) -> ProgressEvent {
        ProgressEvent {
            stage,
            current,
            total,
            label: Some(label.to_owned()),
            eta_secs: None,
        }
    }

    #[test]
    fn embed_batch_progress_keeps_the_pill_moving() {
        // Bug 9: the embed phase fires AFTER the last IndexFile tick.
        // Pre-fix it hit the `_ => {}` arm so the status froze at the
        // final `IndexFile` label (`total-1/total`) for the whole embed
        // pass, which looks like a stuck reindex. The fix maps
        // EmbedBatch onto `Building` so the pill animates until the
        // build resolves to Idle.
        let status = Arc::new(Mutex::new(IndexStatus::Building {
            current: 1999,
            total: 2000,
            file: "notes/note-999.md".to_owned(),
        }));
        let updater = StatusUpdater {
            status: status.clone(),
            forward: Arc::new(chan_workspace::NoProgress),
        };
        updater.on_progress(progress_event(
            ProgressStage::EmbedBatch,
            4096,
            8192,
            "files=512 last=notes/note-1999.md",
        ));
        let snapshot = status.lock().unwrap().clone();
        match snapshot {
            IndexStatus::Building {
                current,
                total,
                file,
            } => {
                assert_eq!(current, 4096);
                assert_eq!(total, 8192);
                assert_eq!(file, "embedding", "embed phase label");
            }
            other => panic!("expected Building during embed, got {other:?}"),
        }
    }

    #[test]
    fn model_load_progress_does_not_clobber_the_index_status() {
        // ModelLoad is a phase boundary on its own surface; it must not
        // overwrite an in-flight Building status.
        let status = Arc::new(Mutex::new(IndexStatus::Building {
            current: 10,
            total: 100,
            file: "x.md".to_owned(),
        }));
        let updater = StatusUpdater {
            status: status.clone(),
            forward: Arc::new(chan_workspace::NoProgress),
        };
        updater.on_progress(progress_event(ProgressStage::ModelLoad, 1, 3, "resolve"));
        assert!(matches!(
            &*status.lock().unwrap(),
            IndexStatus::Building {
                current: 10,
                total: 100,
                ..
            }
        ));
    }

    #[test]
    fn reconcile_idle_clears_pill_when_workspace_is_gone() {
        // Bug 9 clear path: a rebuild that resolves after the workspace
        // cell was swapped out (reset/shutdown) must still leave the
        // status out of `Building`, or the pill is stuck forever.
        let status = Arc::new(Mutex::new(IndexStatus::Building {
            current: 5,
            total: 10,
            file: "y.md".to_owned(),
        }));
        let telemetry = Arc::new(Mutex::new(IndexerTelemetry {
            queue_depth: 0,
            last_event_at: None,
            last_settled_at: None,
            coalesced_rebuild: true,
        }));
        // A Weak that never upgrades: nothing to query, but the status
        // must not stay Building.
        let dead: Weak<Workspace> = Weak::new();
        reconcile_idle(&dead, &status, &telemetry);
        assert!(matches!(&*status.lock().unwrap(), IndexStatus::Idle { .. }));
    }

    #[test]
    fn reconcile_idle_reads_live_stats_when_workspace_present() {
        let (_cfg, dir, workspace) = setup_workspace();
        fs::write(dir.path().join("a.md"), "# A\n\nbody token\n").unwrap();
        apply_watch_change(&workspace, "a.md", false).unwrap();
        let status = Arc::new(Mutex::new(IndexStatus::Building {
            current: 0,
            total: 1,
            file: String::new(),
        }));
        let telemetry = Arc::new(Mutex::new(IndexerTelemetry {
            queue_depth: 3,
            last_event_at: Some(1),
            last_settled_at: None,
            coalesced_rebuild: true,
        }));
        let weak = Arc::downgrade(&workspace);
        reconcile_idle(&weak, &status, &telemetry);
        let snapshot = status.lock().unwrap().clone();
        match snapshot {
            IndexStatus::Idle { indexed_docs, .. } => assert!(indexed_docs >= 1),
            other => panic!("expected Idle, got {other:?}"),
        }
        // set_idle also resets the coalesced-rebuild flag.
        assert!(!telemetry.lock().unwrap().coalesced_rebuild);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn idle_indexer_does_not_keep_workspace_handle_alive() {
        let (_cfg, _dir, workspace) = setup_workspace();
        let (_events_tx, events_rx) = tokio::sync::broadcast::channel(1);
        let indexer = super::Indexer::spawn(
            workspace.clone(),
            events_rx,
            false,
            chan_workspace::SearchAggression::Conservative,
            Arc::new(chan_workspace::NoProgress),
        );
        assert_eq!(Arc::strong_count(&workspace), 1);

        drop(indexer);
        assert_eq!(Arc::strong_count(&workspace), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn webtest_a_repro_drafts_via_write_text_then_boot_walk() {
        // systacean-38: empirical reproduction of @@WebtestA's
        // 5th-round PARTIAL. Uses `Workspace::write_text` (the actual
        // path Cmd+N's `api_create_draft` takes, post-`-26`) +
        // simulates the chan-server boot flow via Indexer::spawn.
        //
        // If `-36` + `-37` are both correctly wired, this test
        // should PASS: the watcher path catches the write +
        // populates BM25; the boot walk re-indexes on restart;
        // search returns the hit.
        //
        // If this test FAILS, we've reproduced the 5th-round gap
        // in-tree + can probe the failure mode without needing
        // a live chan serve.
        //
        // Serialize against the sibling boot-walk test so the two
        // do not stack their `spawn_blocking` re-index load on each
        // other under the full parallel `cargo test` run.
        let _serial = boot_walk_test_lock();
        let (_cfg, workspace_dir, workspace) = setup_workspace();

        // Seed workspace root to force the ELSE IF branch on the
        // SECOND Indexer::spawn (graph + BM25 non-empty after
        // initial reindex).
        std::fs::write(workspace_dir.path().join("seed.md"), "# seed\nbody\n").unwrap();
        workspace.reindex(None).unwrap();
        assert!(workspace.index_stats().unwrap().indexed_docs > 0);

        // Mimic Cmd+N: create draft dir + write file via
        // unified-path API.
        workspace.create_draft_dir("untitled").unwrap();
        workspace
            .write_text(
                "Drafts/untitled/draft.md",
                "# my draft\nUNIQUEMARKER38BM25CLOSURE here\n",
            )
            .unwrap();

        // SIMULATE chan-server restart: spawn the indexer fresh
        // against the same workspace. The boot walk should re-index
        // existing drafts content under the unified key.
        let (_events_tx, events_rx) = tokio::sync::broadcast::channel(64);
        let progress: std::sync::Arc<dyn chan_workspace::ProgressCallback> =
            std::sync::Arc::new(chan_workspace::NoProgress);
        let _indexer = super::Indexer::spawn(
            workspace.clone(),
            events_rx,
            true,
            chan_workspace::SearchAggression::Balanced,
            progress,
        );

        // Poll BM25 outcome for the boot walk + commit + reader
        // refresh cycle to complete. Bounded by BOOT_WALK_BUDGET;
        // returns as soon as the hit lands, so the common path is
        // fast and only the worst case under parallel load waits.
        let opts = chan_workspace::SearchOpts {
            mode: chan_workspace::SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let expected = "Drafts/untitled/draft.md";
        let seen = poll_until(BOOT_WALK_BUDGET, || {
            matches!(
                workspace.search("UNIQUEMARKER38BM25CLOSURE", &opts),
                Ok(hits) if hits.hits.iter().any(|h| h.path == expected)
            )
        })
        .await;
        if !seen {
            // Empirical-audit failure mode: capture diagnostic
            // state for the task tail. Probe the graph + index
            // stats to figure out where the write went.
            let graph_files = workspace.graph().unwrap().files().unwrap();
            let in_graph = graph_files.iter().any(|p| p == expected);
            let stats = workspace.index_stats().unwrap();
            panic!(
                "BM25 search missed the draft after boot walk; \
                 graph_contains_path={in_graph}, stats={stats:?}, \
                 graph_files={graph_files:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn indexer_spawn_walks_drafts_on_boot_when_workspace_root_has_content() {
        // systacean-38: empirical test of `-37`'s unconditional
        // boot walk. Replicates @@WebtestA's repro: workspace has
        // workspace-root content (so reindex would NOT fire on
        // boot), Drafts subtree has content with a unique
        // marker, `Indexer::spawn` is called, content search
        // must return a hit.
        //
        // Pre-`-37` this would fail: reindex skipped (workspace
        // non-empty) -> `-34`'s walker never runs -> drafts not
        // in BM25.
        //
        // Serialize against the sibling boot-walk test so the two
        // do not stack their `spawn_blocking` re-index load on each
        // other under the full parallel `cargo test` run.
        let _serial = boot_walk_test_lock();
        let (_cfg, workspace_dir, workspace) = setup_workspace();

        // Seed workspace root with content so `indexed_docs > 0`
        // after we reindex below.
        std::fs::write(workspace_dir.path().join("seed.md"), "# seed\nbody\n").unwrap();
        workspace.reindex(None).unwrap();
        let stats = workspace.index_stats().unwrap();
        assert!(stats.indexed_docs > 0, "seed not indexed: {stats:?}");

        // Seed Drafts subtree with a marker token. Bypass the
        // watcher (write directly via std::fs into drafts_dir)
        // so the boot walk is the ONLY path that can land this
        // in BM25.
        workspace.create_draft_dir("untitled").unwrap();
        std::fs::write(
            workspace.drafts_dir().join("untitled").join("draft.md"),
            "# my draft\nUNIQUEMARKER37BM25CLOSURE here\n",
        )
        .unwrap();

        // Spawn the indexer (mirrors chan-server `serve` flow).
        // initial_build=true triggers the boot-time `-37` walk.
        let (_events_tx, events_rx) = tokio::sync::broadcast::channel(64);
        let progress: std::sync::Arc<dyn chan_workspace::ProgressCallback> =
            std::sync::Arc::new(chan_workspace::NoProgress);
        let _indexer = super::Indexer::spawn(
            workspace.clone(),
            events_rx,
            true,
            chan_workspace::SearchAggression::Balanced,
            progress,
        );

        // Boot walk is `tokio::task::spawn_blocking`'d; poll for it
        // to complete + BM25 to commit + reader to refresh. Bounded
        // by BOOT_WALK_BUDGET; returns as soon as the hit lands.
        let opts = chan_workspace::SearchOpts {
            mode: chan_workspace::SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let expected = "Drafts/untitled/draft.md";
        let seen = poll_until(BOOT_WALK_BUDGET, || {
            matches!(
                workspace.search("UNIQUEMARKER37BM25CLOSURE", &opts),
                Ok(hits) if hits.hits.iter().any(|h| h.path == expected)
            )
        })
        .await;
        assert!(
            seen,
            "boot walk did not land the Drafts file in BM25 within the budget; \
             search for UNIQUEMARKER37BM25CLOSURE returned no hit"
        );
    }

    #[test]
    fn apply_watch_change_indexes_drafts_prefixed_path() {
        // systacean-36: closes the recurring `-a-66 slice e`
        // PARTIAL. The chan-workspace watcher (`-25`) emits drafts
        // events with the `Drafts/` prefix. Pre-`-36`, this
        // function ran `resolve_safe(workspace.root(), path)` for
        // ALL paths including the prefixed ones; drafts live
        // outside workspace root -> resolve_safe failed -> events
        // silently dropped -> graph + BM25 empty under `Drafts/`
        // despite the watcher being correctly attached.
        //
        // After `-36`, prefixed paths route through
        // `index_draft_file` (parallel to the `Workspace::stat` /
        // `read_text` / `list` unified-path API from
        // `-26`/`-29`/`-32`).
        let (_cfg, _dir, workspace) = setup_workspace();
        workspace.create_draft_dir("untitled-1").unwrap();
        fs::write(
            workspace.drafts_dir().join("untitled-1").join("draft.md"),
            "# hello\napply-watch-marker here\n",
        )
        .unwrap();

        let outcome = apply_watch_change(&workspace, "Drafts/untitled-1/draft.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::Indexed);

        // Verify the side-effect: graph + BM25 now know about
        // the drafted file under the unified key.
        let graph = workspace.graph().unwrap();
        let files = graph.files().unwrap();
        assert!(
            files.iter().any(|p| p == "Drafts/untitled-1/draft.md"),
            "graph should know the prefixed draft path; got {files:?}"
        );

        let opts = chan_workspace::SearchOpts {
            mode: chan_workspace::SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let hits = workspace.search("apply-watch-marker", &opts).unwrap();
        assert!(
            hits.hits
                .iter()
                .any(|h| h.path == "Drafts/untitled-1/draft.md"),
            "BM25 should return the draft hit; got {:?}",
            hits.hits
        );
    }

    #[test]
    fn create_event_admits_new_indexable_file_into_bm25() {
        let (_cfg, dir, workspace) = setup_workspace();
        fs::write(
            dir.path().join("brand.md"),
            "# Brand\n\nnew doc with keyword brandnewprobe\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("brand.txt"),
            "plain text with keyword brandnewprobetxt\n",
        )
        .unwrap();

        for path in ["brand.md", "brand.txt"] {
            let change = match classify(&ev(WatchKind::Created, Some(path), None)) {
                WatchAction::Changes(mut changes) => {
                    assert_eq!(changes.len(), 1);
                    changes.remove(0)
                }
                other => panic!("expected created change for {path}, got {other:?}"),
            };
            assert_eq!(
                apply_watch_change(&workspace, &change.path, change.deleted).unwrap(),
                ApplyOutcome::Indexed
            );
        }

        let stats = workspace.index_stats().unwrap();
        assert_eq!(stats.indexed_docs, 2);

        let opts = SearchOpts {
            mode: SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        assert!(workspace
            .search("brandnewprobe", &opts)
            .unwrap()
            .hits
            .iter()
            .any(|hit| hit.path == "brand.md"));
        assert!(workspace
            .search("brandnewprobetxt", &opts)
            .unwrap()
            .hits
            .iter()
            .any(|hit| hit.path == "brand.txt"));
    }

    #[test]
    fn rapid_modify_burst_indexes_latest_file_body() {
        let (_cfg, dir, workspace) = setup_workspace();
        let path = dir.path().join("rapid.md");
        fs::write(&path, "# Rapid\n\nrapid-token-00\n").unwrap();
        assert_eq!(
            apply_watch_change(&workspace, "rapid.md", false).unwrap(),
            ApplyOutcome::Indexed
        );

        for n in 1..=5 {
            fs::write(&path, format!("# Rapid\n\nrapid-token-{n:02}\n")).unwrap();
        }
        assert_eq!(
            apply_watch_change(&workspace, "rapid.md", false).unwrap(),
            ApplyOutcome::Indexed
        );

        let opts = SearchOpts {
            mode: SearchMode::Bm25,
            limit: 10,
            scope: None,
        };
        let latest = workspace.search("rapid-token-05", &opts).unwrap();
        assert!(
            latest.hits.iter().any(|hit| hit.path == "rapid.md"),
            "latest rapid edit should be searchable; got {:?}",
            latest.hits
        );
        let stale = workspace.search("rapid-token-00", &opts).unwrap();
        assert!(
            stale.hits.is_empty(),
            "stale rapid edit token should not survive; got {:?}",
            stale.hits
        );
    }

    #[test]
    fn apply_watch_change_forgets_on_delete_flag() {
        let (_cfg, _dir, workspace) = setup_workspace();
        let outcome = apply_watch_change(&workspace, "gone.md", true).unwrap();
        assert_eq!(outcome, ApplyOutcome::Forgotten);
    }

    #[test]
    fn apply_watch_change_skips_missing_path() {
        let (_cfg, _dir, workspace) = setup_workspace();
        let outcome = apply_watch_change(&workspace, "never-existed.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedMissing);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_skips_symlink_to_existing_target() {
        let (_cfg, dir, workspace) = setup_workspace();
        fs::write(dir.path().join("real.md"), "# Real\n").unwrap();
        std::os::unix::fs::symlink("real.md", dir.path().join("alias.md")).unwrap();
        let outcome = apply_watch_change(&workspace, "alias.md", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedSpecial);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_skips_broken_symlink() {
        let (_cfg, dir, workspace) = setup_workspace();
        std::os::unix::fs::symlink("does-not-exist.md", dir.path().join("broken.md")).unwrap();
        let outcome = apply_watch_change(&workspace, "broken.md", false).unwrap();
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
        let (_cfg, dir, workspace) = setup_workspace();
        let fifo_path = dir.path().join("attach.fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => return,
        }
        let outcome = apply_watch_change(&workspace, "attach.fifo", false).unwrap();
        assert_eq!(outcome, ApplyOutcome::SkippedSpecial);
    }

    #[cfg(unix)]
    #[test]
    fn apply_watch_change_special_clears_prior_index_entry() {
        // Regression: if a user replaces a regular .md with a symlink
        // of the same name, the apply path should clean out the old
        // index row instead of leaving it stale.
        let (_cfg, dir, workspace) = setup_workspace();
        fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        assert_eq!(
            apply_watch_change(&workspace, "a.md", false).unwrap(),
            ApplyOutcome::Indexed
        );
        let before = workspace.index_stats().unwrap().indexed_docs;
        fs::remove_file(dir.path().join("a.md")).unwrap();
        fs::write(dir.path().join("real.md"), "# Real\n").unwrap();
        std::os::unix::fs::symlink("real.md", dir.path().join("a.md")).unwrap();
        assert_eq!(
            apply_watch_change(&workspace, "a.md", false).unwrap(),
            ApplyOutcome::SkippedSpecial
        );
        // Best-effort cleanup ran: the prior `a.md` row is gone.
        let after = workspace.index_stats().unwrap().indexed_docs;
        assert!(
            after < before,
            "expected indexed_docs to drop after symlink replacement; before={before} after={after}"
        );
    }
}
