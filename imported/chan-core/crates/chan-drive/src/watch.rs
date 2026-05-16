// Filesystem watcher.
//
// Callback-based on purpose: makes the API uniffi-friendly (the
// FFI client passes a Swift / Kotlin object that implements the
// callback trait, no closures across the boundary). The native
// implementation uses `notify` and runs the watcher on its own
// thread; events are filtered through `is_chan_internal` and
// `walk_drive`'s pruning rules so `.chan/` and `.git/` activity
// never reaches the callback.
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

/// Holds the underlying watcher; drop to stop watching.
pub struct WatchHandle {
    /// Kept alive so the watcher thread doesn't exit. Field is
    /// `_watcher` because we don't access it after construction.
    _watcher: RecommendedWatcher,
}

impl WatchHandle {
    pub(crate) fn start(drive_root: &Path, cb: Arc<dyn WatchCallback>) -> Result<Self> {
        let root = drive_root.to_path_buf();
        let cb_clone = cb.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => dispatch(&root, event, &*cb_clone),
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
        watcher.watch(drive_root, RecursiveMode::Recursive)?;
        Ok(Self { _watcher: watcher })
    }
}

fn dispatch(root: &Path, event: notify::Event, cb: &dyn WatchCallback) {
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
    let from_rel = from.as_deref().and_then(|p| relativize(root, p));
    let to_rel = to.as_deref().and_then(|p| relativize(root, p));

    // Skip drive-internal noise unconditionally on both legs of a
    // rename. The previous version only filtered `from_rel`, so a
    // pathological rename whose destination landed inside .chan/
    // would leak through.
    if from_rel.as_deref().map(is_filtered).unwrap_or(false) {
        return;
    }
    if to_rel.as_deref().map(is_filtered).unwrap_or(false) {
        return;
    }

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
    is_chan_internal(rel) || rel == ".git" || rel.starts_with(".git/")
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
}
