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
                    cb_clone.on_event(WatchEvent {
                        kind: WatchKind::ProviderError,
                        path: Some(e.to_string()),
                        to: None,
                    });
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

    cb.on_event(WatchEvent {
        kind,
        path: from_rel,
        to: to_rel,
    });
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
