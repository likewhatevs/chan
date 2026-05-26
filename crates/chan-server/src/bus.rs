//! Bridges chan-drive watcher/progress callbacks into the shared
//! `events_tx` JSON broadcast channel.
//!
//! Both producers fan into one channel; each frame carries a `type`
//! discriminator so the frontend can route on it. Watcher events also
//! get forwarded to the indexer's raw-event channel — the indexer
//! does NOT honor the self-write dedupe, since in-app saves must
//! reindex.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chan_drive::{ProgressCallback, ProgressEvent, WatchCallback, WatchEvent};
use tokio::sync::{broadcast, mpsc};

use crate::self_writes::SelfWrites;

/// Construct a watcher bridge. Extracted so /api/storage/reset can
/// rebuild one cheaply when re-attaching the watcher to a fresh
/// Drive instance.
///
/// The bridge fans out every event to two consumers:
///
///   - `events_tx`: pre-serialized JSON frames forwarded to /ws
///     subscribers. Self-write echoes (the editor saving through
///     /api/markdown PUT and then seeing its own save) are
///     suppressed here so the UI doesn't show a phantom external-
///     edit toast.
///   - `index_tx`: raw `WatchEvent` for the background indexer.
///     Self-write suppression DOES NOT apply here: in-app saves
///     must reindex, otherwise search drifts every time the user
///     types. The indexer applies its own debounce.
pub fn make_watch_bridge(
    events_tx: &broadcast::Sender<String>,
    index_tx: &broadcast::Sender<WatchEvent>,
    self_writes: &Arc<SelfWrites>,
    scopes: &Arc<ScopeRegistry>,
) -> Arc<dyn WatchCallback> {
    Arc::new(WatchBroadcast {
        tx: events_tx.clone(),
        index_tx: index_tx.clone(),
        self_writes: self_writes.clone(),
        scopes: scopes.clone(),
    })
}

struct WatchBroadcast {
    tx: broadcast::Sender<String>,
    index_tx: broadcast::Sender<WatchEvent>,
    self_writes: Arc<SelfWrites>,
    scopes: Arc<ScopeRegistry>,
}

impl WatchCallback for WatchBroadcast {
    fn on_event(&self, event: WatchEvent) {
        // Indexer always sees the event. Send-error means there are
        // no subscribers (indexer not spawned yet, or shut down);
        // safe to drop because a no-subscriber channel just keeps
        // events in the ring until one connects.
        let _ = self.index_tx.send(event.clone());
        if event_is_self_echo(&event, &self.self_writes) {
            return;
        }
        // Legacy global frame for the editor's open-document
        // external-edit toast (D2: kept alongside the scoped `fs`
        // frame). Fans out to every /ws socket regardless of scope.
        let frame = serde_json::json!({"type": "watch", "event": event});
        if let Ok(s) = serde_json::to_string(&frame) {
            let _ = self.tx.send(s);
        }
        // Scoped fan-out (D1(b)): derive per-directory `fs` frames
        // from this single recursive feed by first-degree directory
        // match, and deliver them only to the sockets subscribed to
        // the matching scope. No extra OS watchers are attached.
        self.scopes.emit_fs(&event);
    }
}

fn event_is_self_echo(event: &WatchEvent, sw: &SelfWrites) -> bool {
    if let Some(p) = event.path.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    if let Some(p) = event.to.as_deref() {
        if sw.should_suppress(p) {
            return true;
        }
    }
    false
}

/// Unique id for one connected `/ws` socket's scope subscriptions.
/// Allocated on socket accept; every `sub`/`unsub` the socket sends is
/// recorded against this id so a disconnect can drop all of them at
/// once (no leaked scopes on an abrupt close).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(u64);

/// Per-directory scoped watcher pub/sub registry (phase-11 Slice C,
/// Decision D1(b)).
///
/// There is one recursive OS watcher on the drive (it feeds the
/// indexer); this registry does NOT attach per-directory OS watchers.
/// Instead it derives per-directory `fs` frames from that single feed
/// by first-degree directory match and delivers each frame only to the
/// sockets subscribed to the matching scope. "Tear down the watcher"
/// from the round-1 wording maps here to "drop the scope's bookkeeping
/// and stop emitting frames for it" — the lifecycle and the
/// sub1/sub2/unsub1/unsub2 refcount are identical to the real-OS-watcher
/// design, just without the inotify-watch-count pressure on big trees.
///
/// Refcount model: each scope is keyed by a drive-relative directory
/// path (POSIX, `""` is the drive root). A scope's refcount is the size
/// of its subscriber set; the scope entry exists exactly while it has
/// at least one subscriber, so the last `unsub` (or socket close)
/// removes the entry. The scope is NOT tied to its creating
/// subscriber's identity: the original creator unsubscribing while a
/// later subscriber remains keeps the scope alive.
#[derive(Default)]
pub struct ScopeRegistry {
    next_id: AtomicU64,
    inner: Mutex<ScopeInner>,
}

#[derive(Default)]
struct ScopeInner {
    /// dir (drive-relative) -> the set of subscribers watching it.
    /// Entry present iff at least one subscriber. The set size is the
    /// refcount.
    scopes: HashMap<String, HashSet<SubId>>,
    /// SubId -> that socket's scoped-frame outbox + the set of dirs it
    /// is subscribed to (so a disconnect can decrement every scope).
    subscribers: HashMap<SubId, Subscriber>,
}

struct Subscriber {
    /// Scoped `fs` frames for this socket are pushed here; `ws_pump`
    /// drains it into the socket. Unbounded because filesystem bursts
    /// must not block the watcher thread; a slow client is bounded by
    /// the socket send, not this queue.
    outbox: mpsc::UnboundedSender<String>,
    dirs: HashSet<String>,
}

impl ScopeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a connected socket. Returns its `SubId` and the receiver
    /// end of its scoped-frame outbox; `ws_pump` selects on the receiver
    /// to forward `fs` frames. The socket starts with no scope
    /// subscriptions; it sends `sub` frames to add them.
    pub fn register(&self) -> (SubId, mpsc::UnboundedReceiver<String>) {
        let id = SubId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::unbounded_channel();
        let mut inner = self.lock();
        inner.subscribers.insert(
            id,
            Subscriber {
                outbox: tx,
                dirs: HashSet::new(),
            },
        );
        (id, rx)
    }

    /// Subscribe `id` to `dir`. Idempotent: a repeat `sub` for a dir the
    /// socket already holds does not double-count (the subscriber set is
    /// keyed by `SubId`). The first subscriber for a dir creates the
    /// scope entry; later subscribers reuse it.
    pub fn subscribe(&self, id: SubId, dir: &str) {
        let dir = normalize_dir(dir);
        let mut inner = self.lock();
        let Some(sub) = inner.subscribers.get_mut(&id) else {
            return; // socket already unregistered; drop the late frame.
        };
        sub.dirs.insert(dir.clone());
        inner.scopes.entry(dir).or_default().insert(id);
    }

    /// Unsubscribe `id` from `dir`. The scope stays alive while any
    /// other subscriber remains; the last unsubscribe removes the scope
    /// entry entirely.
    pub fn unsubscribe(&self, id: SubId, dir: &str) {
        let dir = normalize_dir(dir);
        let mut inner = self.lock();
        if let Some(sub) = inner.subscribers.get_mut(&id) {
            sub.dirs.remove(&dir);
        }
        Self::drop_scope_member(&mut inner, &dir, id);
    }

    /// Drop a socket entirely (disconnect). Removes the subscriber and
    /// decrements every scope it held, tearing down any scope whose
    /// refcount reaches zero. A disconnect therefore cannot leak scopes.
    pub fn unregister(&self, id: SubId) {
        let mut inner = self.lock();
        let Some(sub) = inner.subscribers.remove(&id) else {
            return;
        };
        for dir in sub.dirs {
            Self::drop_scope_member(&mut inner, &dir, id);
        }
    }

    /// Current refcount for `dir` (number of distinct subscribers).
    /// Zero when the scope is not present. The test-facing assertion for
    /// the refcount invariant; gated to tests until a server-side reader
    /// (e.g. diagnostics) needs it in production.
    #[cfg(test)]
    pub fn subscriber_count(&self, dir: &str) -> usize {
        let dir = normalize_dir(dir);
        self.lock()
            .scopes
            .get(&dir)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// True when `dir` currently has a live scope entry. The "watcher
    /// exists" predicate the sub1/sub2/unsub1/unsub2 test asserts on;
    /// under D1(b) the scope entry IS the watcher. Test-gated for the
    /// same reason as `subscriber_count`.
    #[cfg(test)]
    pub fn scope_exists(&self, dir: &str) -> bool {
        let dir = normalize_dir(dir);
        self.lock().scopes.contains_key(&dir)
    }

    /// Derive and deliver scoped `fs` frames for one watch event. An
    /// event affecting a path delivers to the scope of the path's
    /// IMMEDIATE parent directory (first-degree only; a subscriber to
    /// `d` sees direct children of `d`, never grandchildren). A rename
    /// whose source and destination sit in different directories
    /// surfaces on both parents' scopes, matching the contract that a
    /// straddling rename is seen by each side.
    pub fn emit_fs(&self, event: &WatchEvent) {
        let inner = self.lock();
        if inner.scopes.is_empty() {
            return;
        }
        // The set of parent dirs this event touches. A non-rename
        // touches one; a straddling rename touches up to two.
        let mut parents: Vec<&str> = Vec::with_capacity(2);
        if let Some(p) = event.path.as_deref() {
            parents.push(parent_dir(p));
        }
        if let Some(p) = event.to.as_deref() {
            let parent = parent_dir(p);
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
        for dir in parents {
            let Some(subs) = inner.scopes.get(dir) else {
                continue;
            };
            if subs.is_empty() {
                continue;
            }
            let frame = serde_json::json!({
                "type": "fs",
                "dir": dir,
                "event": event,
            });
            let Ok(serialized) = serde_json::to_string(&frame) else {
                continue;
            };
            // The outbox is an unbounded mpsc, so `send` is non-blocking;
            // delivering inside the lock keeps the critical section to a
            // few queue pushes. A closed receiver (socket gone but not
            // yet unregistered) just drops the frame.
            for id in subs {
                if let Some(sub) = inner.subscribers.get(id) {
                    let _ = sub.outbox.send(serialized.clone());
                }
            }
        }
    }

    fn drop_scope_member(inner: &mut ScopeInner, dir: &str, id: SubId) {
        if let Some(set) = inner.scopes.get_mut(dir) {
            set.remove(&id);
            if set.is_empty() {
                inner.scopes.remove(dir);
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ScopeInner> {
        // The registry mutex guards only fast in-memory map edits; a
        // poisoned lock means a prior panic while holding it, which for
        // this bookkeeping is unrecoverable. Recover the guard rather
        // than propagate, matching the rest of chan-server's Mutex use.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Normalize a drive-relative directory key: trim leading/trailing
/// slashes so `"notes/"`, `"/notes"`, and `"notes"` map to the same
/// scope, and the drive root is always `""`.
fn normalize_dir(dir: &str) -> String {
    dir.trim_matches('/').to_string()
}

/// The immediate parent directory of a drive-relative POSIX path. A
/// top-level entry (`"a.md"`) has parent `""` (the drive root). Used to
/// route an event to its first-degree scope.
fn parent_dir(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(idx) => &trimmed[..idx],
        None => "",
    }
}

/// Bridge from chan-drive's `ProgressCallback` into the shared
/// JSON-envelope broadcast channel. Every progress tick (per-file
/// during reindex, per-batch during embedding, etc.) lands on the
/// same `/ws` stream every other producer uses, with `type` set to
/// `"progress"` so the frontend can route the frame distinctly.
///
/// `Send + Sync` because `ProgressCallback` can fire from worker
/// threads inside the embedder and graph rebuilders.
pub fn make_progress_broadcast(events_tx: &broadcast::Sender<String>) -> Arc<dyn ProgressCallback> {
    Arc::new(ProgressBroadcast {
        tx: events_tx.clone(),
    })
}

struct ProgressBroadcast {
    tx: broadcast::Sender<String>,
}

impl ProgressCallback for ProgressBroadcast {
    fn on_progress(&self, event: ProgressEvent) {
        let frame = serde_json::json!({"type": "progress", "event": event});
        if let Ok(s) = serde_json::to_string(&frame) {
            // Best-effort: lagged subscribers are dropped by the
            // broadcast channel naturally; a no-subscriber send
            // returns an error we ignore for the same reason as
            // the watch bridge above.
            let _ = self.tx.send(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn recv_json(rx: &mut broadcast::Receiver<String>) -> Value {
        let raw = rx.try_recv().expect("broadcast frame");
        serde_json::from_str(&raw).expect("json frame")
    }

    #[test]
    fn progress_frame_serializes() {
        let (tx, mut rx) = broadcast::channel(8);
        let sink = make_progress_broadcast(&tx);
        sink.on_progress(ProgressEvent {
            stage: chan_drive::ProgressStage::IndexFile,
            current: 1,
            total: 2,
            label: Some("a.md".into()),
            eta_secs: None,
        });

        let frame = recv_json(&mut rx);
        assert_eq!(frame["type"], "progress");
        assert_eq!(frame["event"]["stage"], "IndexFile");
    }

    use chan_drive::WatchKind;

    fn created(path: &str) -> WatchEvent {
        WatchEvent {
            kind: WatchKind::Created,
            path: Some(path.to_string()),
            to: None,
        }
    }

    fn renamed(from: &str, to: &str) -> WatchEvent {
        WatchEvent {
            kind: WatchKind::Renamed,
            path: Some(from.to_string()),
            to: Some(to.to_string()),
        }
    }

    fn try_recv(rx: &mut mpsc::UnboundedReceiver<String>) -> Option<Value> {
        rx.try_recv()
            .ok()
            .map(|s| serde_json::from_str(&s).expect("json frame"))
    }

    #[test]
    fn parent_dir_is_first_degree() {
        assert_eq!(parent_dir("a.md"), "");
        assert_eq!(parent_dir("notes/a.md"), "notes");
        assert_eq!(parent_dir("notes/recipes/a.md"), "notes/recipes");
        // A trailing slash on a directory path does not change its parent.
        assert_eq!(parent_dir("notes/recipes/"), "notes");
    }

    #[test]
    fn normalize_dir_collapses_slashes_to_one_key() {
        assert_eq!(normalize_dir("/notes/"), "notes");
        assert_eq!(normalize_dir("notes"), "notes");
        assert_eq!(normalize_dir(""), "");
        assert_eq!(normalize_dir("/"), "");
    }

    // The required hardening matrix from the spine contract. Under D1(b)
    // the "watcher" for a directory IS its scope entry in the registry, so
    // "the watcher exists / is torn down" is asserted via `scope_exists`,
    // and refcount stability across sub2/unsub1 via `subscriber_count`.
    #[test]
    fn scope_refcount_sub1_sub2_unsub1_unsub2() {
        let reg = ScopeRegistry::new();
        let (s1, _rx1) = reg.register();
        let (s2, _rx2) = reg.register();
        let dir = "notes/recipes";

        // sub1: first subscriber creates the scope, refcount = 1.
        reg.subscribe(s1, dir);
        assert!(reg.scope_exists(dir), "sub1 must create the scope");
        assert_eq!(reg.subscriber_count(dir), 1);

        // sub2: a different socket reuses the SAME scope, refcount = 2.
        // No second scope entry appears; the key is identical.
        reg.subscribe(s2, dir);
        assert!(reg.scope_exists(dir));
        assert_eq!(reg.subscriber_count(dir), 2, "sub2 reuses, refcount = 2");

        // unsub1: the ORIGINAL creator unsubscribes. The scope STAYS
        // alive (it is not tied to the creator's identity), refcount = 1.
        reg.unsubscribe(s1, dir);
        assert!(
            reg.scope_exists(dir),
            "unsub1 by the creator must NOT tear down while s2 remains"
        );
        assert_eq!(reg.subscriber_count(dir), 1);

        // unsub2: last subscriber leaves. Refcount = 0, scope torn down.
        reg.unsubscribe(s2, dir);
        assert!(
            !reg.scope_exists(dir),
            "unsub2 (last) must tear the scope down"
        );
        assert_eq!(reg.subscriber_count(dir), 0);
    }

    #[test]
    fn socket_close_drops_all_of_its_scopes() {
        let reg = ScopeRegistry::new();
        let (s1, _rx1) = reg.register();
        let (s2, _rx2) = reg.register();
        // s1 holds two scopes; s2 shares one of them.
        reg.subscribe(s1, "notes");
        reg.subscribe(s1, "notes/recipes");
        reg.subscribe(s2, "notes");
        assert_eq!(reg.subscriber_count("notes"), 2);
        assert_eq!(reg.subscriber_count("notes/recipes"), 1);

        // Disconnect s1: both of its scopes decrement. The shared one
        // survives (s2 still holds it); the exclusive one is torn down.
        reg.unregister(s1);
        assert_eq!(
            reg.subscriber_count("notes"),
            1,
            "s2 keeps the shared scope"
        );
        assert!(
            !reg.scope_exists("notes/recipes"),
            "s1's exclusive scope is torn down on disconnect"
        );

        // A disconnect cannot leak: dropping s2 empties the registry.
        reg.unregister(s2);
        assert!(!reg.scope_exists("notes"));
        assert_eq!(reg.subscriber_count("notes"), 0);
    }

    #[test]
    fn subscribe_is_idempotent_no_double_count() {
        let reg = ScopeRegistry::new();
        let (s1, _rx1) = reg.register();
        reg.subscribe(s1, "notes");
        reg.subscribe(s1, "notes"); // repeat sub from the same socket
        assert_eq!(
            reg.subscriber_count("notes"),
            1,
            "a repeat sub must not double-count the same socket"
        );
    }

    #[test]
    fn emit_fs_delivers_only_to_first_degree_subscribers() {
        let reg = ScopeRegistry::new();
        let (s1, mut rx1) = reg.register();
        let (s2, mut rx2) = reg.register();
        reg.subscribe(s1, "notes");
        reg.subscribe(s2, "notes/recipes");

        // A direct child of `notes` reaches s1 only (first-degree).
        reg.emit_fs(&created("notes/a.md"));
        let frame = try_recv(&mut rx1).expect("s1 sees its child");
        assert_eq!(frame["type"], "fs");
        assert_eq!(frame["dir"], "notes");
        assert_eq!(frame["event"]["path"], "notes/a.md");
        assert!(try_recv(&mut rx2).is_none(), "s2 is unrelated");

        // A grandchild of `notes` does NOT reach the `notes` subscriber
        // (first-degree only); it reaches the `notes/recipes` subscriber.
        reg.emit_fs(&created("notes/recipes/b.md"));
        assert!(
            try_recv(&mut rx1).is_none(),
            "grandchild must not reach the parent scope"
        );
        let frame = try_recv(&mut rx2).expect("s2 sees its direct child");
        assert_eq!(frame["dir"], "notes/recipes");
    }

    #[test]
    fn emit_fs_root_scope_sees_top_level_entries() {
        let reg = ScopeRegistry::new();
        let (s1, mut rx1) = reg.register();
        reg.subscribe(s1, ""); // drive root
        reg.emit_fs(&created("README.md"));
        let frame = try_recv(&mut rx1).expect("root scope sees top-level files");
        assert_eq!(frame["dir"], "");
        assert_eq!(frame["event"]["path"], "README.md");
    }

    #[test]
    fn emit_fs_straddling_rename_surfaces_on_both_sides() {
        let reg = ScopeRegistry::new();
        let (s1, mut rx1) = reg.register();
        let (s2, mut rx2) = reg.register();
        reg.subscribe(s1, "from");
        reg.subscribe(s2, "to");

        // A rename whose source and destination sit in different dirs is
        // seen by each side's scope, carrying its own `dir`.
        reg.emit_fs(&renamed("from/a.md", "to/a.md"));
        let f1 = try_recv(&mut rx1).expect("source side sees the rename");
        assert_eq!(f1["dir"], "from");
        let f2 = try_recv(&mut rx2).expect("dest side sees the rename");
        assert_eq!(f2["dir"], "to");
    }

    #[test]
    fn emit_fs_with_no_subscribers_is_a_noop() {
        let reg = ScopeRegistry::new();
        // No panic, no work; the early-return on empty scopes covers it.
        reg.emit_fs(&created("notes/a.md"));
        assert_eq!(reg.subscriber_count("notes"), 0);
    }
}
