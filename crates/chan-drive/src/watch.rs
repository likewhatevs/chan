// Filesystem watcher.
//
// Callback-based on purpose: makes the API uniffi-friendly (the
// FFI client passes a Swift / Kotlin object that implements the
// callback trait, no closures across the boundary). The native
// implementation uses `notify` and runs the watcher on its own
// thread; events are filtered through `is_chan_internal` and
// `walk_drive`'s pruning rules so `.chan/` and most `.git/` / `.hg/`
// activity never reaches the callback. A tiny allowlist of VCS
// control files (`.git/HEAD`, `.git/index`, `.hg/dirstate`) is
// forwarded so the server indexer can recognize checkout storms and
// fall back to a full rebuild.
//
// Consumer expectations:
//
//   * No debouncing. A single editor save typically produces a
//     burst of events (Modify -> Rename -> Modify(metadata) etc.
//     on macOS; Create/Modify/Close on Linux). chan-drive forwards
//     every one. Consumers that don't want to re-index per event
//     (most do) should debounce on their side, keyed by
//     `event.path`, with a small wall-clock window (50-200 ms is
//     typical).
//
//   * `WatchEvent.path == None` is a hint to drop caches. It only
//     happens when the backend produced a path the drive can't
//     relativize (the file lives outside the watched root, usually
//     the source side of a rename across mount points). Treat it
//     as "something moved, scope unknown" and reindex the whole
//     drive when feasible.
//
//   * `WatchKind::ProviderError` is the watcher's signal that the
//     event stream is no longer trustworthy: inotify queue
//     overflow, fseventsd hiccup, the watched dir vanishing. The
//     correct response is the same as for the "scope unknown"
//     case: drop caches and trigger a full reindex. The handle is
//     still alive after this event; further events may resume, or
//     the consumer can rebuild the watcher entirely.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fs_ops::is_chan_internal;
use crate::vcs::is_vcs_control_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchKind {
    Created,
    Modified,
    Removed,
    Renamed,
    /// The watcher backend itself errored. The consumer's callback
    /// is the only signal that the stream is no longer reliable;
    /// callers should treat this as a hint to drop their cached
    /// view (search index freshness, autocomplete) and trigger a
    /// full reindex. Common triggers: inotify watch limit hit,
    /// fseventsd hiccup, the watched directory being unmounted.
    /// `path` carries the backend's error message; `to` is unused.
    ProviderError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub kind: WatchKind,
    /// Path relative to the drive root, POSIX-style. None when the
    /// event refers to a path outside the drive root (rare; emitted
    /// only on best-effort). For `ProviderError` this carries the
    /// backend error message instead of a path.
    pub path: Option<String>,
    /// For Renamed events, the destination relative path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// Implement on the consumer side. `Send + Sync` because events
/// arrive on the watcher's worker thread.
pub trait WatchCallback: Send + Sync {
    fn on_event(&self, event: WatchEvent);
}

/// systacean-25: one of possibly several roots the watcher is
/// attached to. `abs` is the absolute filesystem path being
/// watched recursively; `prefix` is the optional keyspace prefix
/// prepended to relative paths emitted for events under this
/// root. Drive root passes `prefix: None`; the Drafts subtree
/// passes `prefix: Some("Drafts".into())` so events emerge in
/// the indexer with a unified `Drafts/<name>/...` keyspace.
#[derive(Debug, Clone)]
pub struct WatchRoot {
    pub abs: PathBuf,
    pub prefix: Option<String>,
}

impl WatchRoot {
    /// Drive-root convenience: no keyspace prefix.
    pub fn drive(abs: &Path) -> Self {
        Self {
            abs: abs.to_path_buf(),
            prefix: None,
        }
    }

    /// Drafts-root convenience: `Drafts/` keyspace prefix.
    pub fn drafts(abs: &Path) -> Self {
        Self {
            abs: abs.to_path_buf(),
            prefix: Some("Drafts".to_string()),
        }
    }
}

/// Holds the underlying watcher; drop to stop watching.
pub struct WatchHandle {
    /// Kept alive so the watcher thread doesn't exit. Field is
    /// `_watcher` because we don't access it after construction.
    _watcher: RecommendedWatcher,
}

impl WatchHandle {
    /// systacean-25: attach the notify backend to one or more
    /// roots. Each root carries an optional keyspace prefix; the
    /// dispatcher relativizes events against whichever root they
    /// emerge under and prepends the prefix when set so the
    /// indexer sees paths in a unified namespace
    /// (`<rel>` for drive-root events; `Drafts/<rel>` for
    /// drafts-root events). Existing single-root callers pass
    /// `&[WatchRoot::drive(drive_root)]`.
    pub(crate) fn start(roots: &[WatchRoot], cb: Arc<dyn WatchCallback>) -> Result<Self> {
        if roots.is_empty() {
            return Err(crate::error::ChanError::Io(
                "WatchHandle::start: at least one root required".into(),
            ));
        }
        let dispatch_roots: Arc<Vec<WatchRoot>> = Arc::new(roots.to_vec());
        let cb_clone = cb.clone();
        let dispatch_roots_for_cb = Arc::clone(&dispatch_roots);
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => dispatch(&dispatch_roots_for_cb, event, &*cb_clone),
                Err(e) => {
                    // notify backend errors (inotify queue overflow,
                    // fseventsd disconnect, watch path vanishing)
                    // mean the event stream is no longer trustworthy.
                    // Surface to the consumer so they can fall back
                    // to a full reindex; the previous behavior of
                    // logging-and-continuing left consumers silently
                    // stale.
                    tracing::warn!("watch error: {e}");
                    safe_call(
                        &*cb_clone,
                        WatchEvent {
                            kind: WatchKind::ProviderError,
                            path: Some(e.to_string()),
                            to: None,
                        },
                    );
                }
            })?;
        for root in dispatch_roots.iter() {
            watcher.watch(&root.abs, RecursiveMode::Recursive)?;
        }
        Ok(Self { _watcher: watcher })
    }
}

/// Find which `WatchRoot` an absolute filesystem path falls
/// under. Returns the root's index + the relative path beneath
/// it (without prefix yet). When several roots could match (one
/// nested inside another), the longer-path root wins — practical
/// safeguard against drafts_dir being misconfigured under
/// drive_root.
fn locate_root<'a>(roots: &'a [WatchRoot], abs: &Path) -> Option<(&'a WatchRoot, String)> {
    let mut best: Option<(&'a WatchRoot, String, usize)> = None;
    for root in roots {
        if let Some(rel) = relativize(&root.abs, abs) {
            let depth = root.abs.components().count();
            if best.as_ref().map(|(_, _, d)| depth > *d).unwrap_or(true) {
                best = Some((root, rel, depth));
            }
        }
    }
    best.map(|(r, rel, _)| (r, rel))
}

fn apply_prefix(prefix: Option<&str>, rel: String) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!("{p}/{rel}"),
        _ => rel,
    }
}

fn dispatch(roots: &[WatchRoot], event: notify::Event, cb: &dyn WatchCallback) {
    use notify::EventKind;
    let kind = match event.kind {
        EventKind::Create(_) => WatchKind::Created,
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => WatchKind::Renamed,
        EventKind::Modify(_) => WatchKind::Modified,
        EventKind::Remove(_) => WatchKind::Removed,
        _ => return,
    };
    let mut paths = event.paths.into_iter();
    let from = paths.next();
    let to = paths.next();

    let from_resolved = from
        .as_deref()
        .and_then(|p| locate_root(roots, p))
        .map(|(root, rel)| (root.prefix.clone(), rel));
    let to_resolved = to
        .as_deref()
        .and_then(|p| locate_root(roots, p))
        .map(|(root, rel)| (root.prefix.clone(), rel));

    // Apply is_filtered against the RAW relative path (no prefix)
    // so the `.chan/` / `.git/` filters keep matching their
    // canonical shape. The prefixed path is the keyspace shape
    // the indexer consumes downstream.
    if from_resolved
        .as_ref()
        .map(|(_, rel)| is_filtered(rel))
        .unwrap_or(false)
    {
        return;
    }
    if to_resolved
        .as_ref()
        .map(|(_, rel)| is_filtered(rel))
        .unwrap_or(false)
    {
        return;
    }

    let from_rel = from_resolved.map(|(prefix, rel)| apply_prefix(prefix.as_deref(), rel));
    let to_rel = to_resolved.map(|(prefix, rel)| apply_prefix(prefix.as_deref(), rel));

    safe_call(
        cb,
        WatchEvent {
            kind,
            path: from_rel,
            to: to_rel,
        },
    );
}

/// Invoke the consumer's callback with one event, catching any
/// panic so the notify worker thread doesn't die. A panic in the
/// callback is turned into a `ProviderError` event so the consumer
/// learns the stream is suspect (the same signal we use for
/// inotify-queue-overflow and friends). Without this, a single
/// `unwrap` in user code stops every subsequent watcher event
/// silently and the editor's incremental indexer goes dark.
fn safe_call(cb: &dyn WatchCallback, event: WatchEvent) {
    // AssertUnwindSafe: we can't constrain consumer callbacks to
    // UnwindSafe (the WatchCallback trait is intentionally minimal),
    // and the alternative -- letting the panic unwind through
    // notify's worker -- silently kills the watcher. The consumer
    // is the one whose state may be left half-mutated by their own
    // panic; surfacing ProviderError gives them the chance to
    // recover via reindex.
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| cb.on_event(event))) {
        let msg = panic_message(&payload);
        tracing::error!("watch callback panicked: {msg}");
        // Best-effort: if the panic-notification itself panics we
        // log and move on. The notify worker stays alive either way.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            cb.on_event(WatchEvent {
                kind: WatchKind::ProviderError,
                path: Some(format!("callback panicked: {msg}")),
                to: None,
            });
        }));
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn is_filtered(rel: &str) -> bool {
    if is_vcs_control_path(rel) {
        return false;
    }
    is_chan_internal(rel)
        || rel == ".git"
        || rel.starts_with(".git/")
        || rel == ".hg"
        || rel.starts_with(".hg/")
}

fn relativize(root: &Path, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Re-exported for callers who want the absolute path that was
/// touched. Not currently surfaced through `WatchEvent`; add when
/// a consumer needs it.
#[allow(dead_code)]
pub(crate) fn _abs(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// A callback that panics on its first event, succeeds on the
    /// second. Used to verify the watcher thread survives a panic
    /// and that a synthetic ProviderError is emitted after.
    struct PanickyOnce {
        calls: AtomicUsize,
        events: Mutex<Vec<WatchEvent>>,
    }

    impl PanickyOnce {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                events: Mutex::new(Vec::new()),
            }
        }
    }

    impl WatchCallback for PanickyOnce {
        fn on_event(&self, event: WatchEvent) {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            // Record every successful invocation, including the
            // ProviderError emitted after the panic so the test can
            // assert it landed.
            self.events.lock().unwrap().push(event.clone());
            if n == 0 {
                panic!("first-call panic");
            }
        }
    }

    #[test]
    fn callback_panic_is_caught_and_surfaces_provider_error() {
        let cb = PanickyOnce::new();
        // First call: panics inside the callback. safe_call must
        // recover; the ProviderError emitted afterwards is the
        // second invocation and lands successfully.
        safe_call(
            &cb,
            WatchEvent {
                kind: WatchKind::Modified,
                path: Some("a.md".into()),
                to: None,
            },
        );
        // Third call: a normal event after the panic. Must land,
        // proving the worker (modeled here as the test thread) is
        // alive.
        safe_call(
            &cb,
            WatchEvent {
                kind: WatchKind::Modified,
                path: Some("b.md".into()),
                to: None,
            },
        );
        let events = cb.events.lock().unwrap();
        assert!(
            events.len() >= 2,
            "expected at least 2 events; got {events:?}"
        );
        // First recorded event is the one that triggered the panic
        // (callback panicked AFTER the push because we record then
        // panic). The next event must be the synthetic ProviderError.
        assert_eq!(events[0].kind, WatchKind::Modified);
        assert_eq!(events[1].kind, WatchKind::ProviderError);
        assert!(
            events[1]
                .path
                .as_deref()
                .map(|s| s.contains("callback panicked"))
                .unwrap_or(false),
            "ProviderError path should describe the panic: {:?}",
            events[1].path,
        );
        // Subsequent normal event lands.
        assert_eq!(events.last().unwrap().kind, WatchKind::Modified);
        assert_eq!(events.last().unwrap().path.as_deref(), Some("b.md"));
    }

    #[test]
    fn filter_allows_vcs_control_paths_but_hides_other_vcs_noise() {
        assert!(!is_filtered(".git/HEAD"));
        assert!(!is_filtered(".git/index"));
        assert!(!is_filtered(".hg/dirstate"));
        assert!(is_filtered(".git/objects/pack/foo"));
        assert!(is_filtered(".hg/store/data/foo"));
    }
}
