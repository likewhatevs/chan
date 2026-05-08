// Filesystem watcher.
//
// Callback-based on purpose: makes the API uniffi-friendly (the
// FFI client passes a Swift / Kotlin object that implements the
// callback trait, no closures across the boundary). The native
// implementation uses `notify` and runs the watcher on its own
// thread; events are filtered through `is_chan_internal` and
// `walk_drive`'s pruning rules so `.chan/` and `.git/` activity
// never reaches the callback.

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub kind: WatchKind,
    /// Path relative to the drive root, POSIX-style. None when the
    /// event refers to a path outside the drive root (rare; emitted
    /// only on best-effort).
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
                Err(e) => tracing::warn!("watch error: {e}"),
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

    // Skip drive-internal noise unconditionally.
    if let Some(rel) = from_rel.as_deref() {
        if is_chan_internal(rel) || rel.starts_with(".git/") || rel == ".git" {
            return;
        }
    }

    cb.on_event(WatchEvent {
        kind,
        path: from_rel,
        to: to_rel,
    });
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
