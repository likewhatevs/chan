//! Filesystem watcher for the chan registry.
//!
//! chan writes `~/.chan/config.toml` via atomic-rename, so we watch
//! the parent directory rather than the file itself: a `rename(tmp,
//! config.toml)` would otherwise replace the inode under our feet
//! and leave the watch dangling. The debouncer collapses bursts
//! (write + rename + chmod) into a single notification, which we
//! forward to the frontend as a Tauri event.

use std::path::Path;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter};

/// Event name pushed to all webviews when the registry changes.
/// Frontends should re-fetch the merged drive list in response.
pub const REGISTRY_CHANGED: &str = "registry-changed";

/// Spawn a debounced watcher over the chan config directory and emit
/// `REGISTRY_CHANGED` whenever something inside changes. The returned
/// debouncer owns the background thread; drop it to stop watching.
///
/// Errors from `notify` setup are returned. Errors observed during
/// watching are logged and otherwise swallowed:
/// the user-visible failure mode is "the UI doesn't auto-refresh",
/// not "the app crashes".
pub fn spawn(app: AppHandle, registry_path: &Path) -> notify::Result<Debouncer<impl Watcher>> {
    let dir = registry_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    // Best-effort: if the chan config dir doesn't exist yet, create
    // it so we have something to watch. chan will populate the file
    // on first `chan add`. Failure here is non-fatal.
    let _ = std::fs::create_dir_all(&dir);

    let mut debouncer = new_debouncer(
        Duration::from_millis(150),
        move |res: DebounceEventResult| match res {
            Ok(_) => {
                if let Err(e) = app.emit(REGISTRY_CHANGED, ()) {
                    tracing::warn!(event = REGISTRY_CHANGED, error = %e, "registry event emit failed");
                }
            }
            Err(e) => tracing::warn!(error = ?e, "registry watch error"),
        },
    )?;

    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)?;
    Ok(debouncer)
}
