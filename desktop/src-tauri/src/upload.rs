//! Native file picker for `cs upload` on the desktop.
//!
//! `cs upload <dir>` reaches the SPA as a `WindowCommand::Upload`, and the SPA
//! would normally raise a hidden `<input type="file">` and `.click()` it. But
//! WKWebView silently drops a programmatic file-input click made outside a user
//! gesture (the same wall as the clipboard-paste quirk in `dropped_paths.rs` /
//! `read_clipboard_text`), so no picker ever appears — the command looks
//! enqueued and nothing happens. On desktop the SPA instead invokes this
//! command, which opens a NATIVE multi-file open panel (the sibling of the
//! launcher's folder picker at `window_ops.rs`), reads the chosen files, and
//! returns their bytes; the SPA wraps them in `File` objects and feeds the SAME
//! `uploadFilesTo` pipeline the Inspector upload pill uses (shared transfer
//! progress, dedup, drafts guard).
//!
//! ACL: scoped (`capabilities/local-upload.json`) to LOCALLY-served
//! workspace/terminal windows and the user's own devserver/tunnel windows
//! (`lib-*`, a registry-configured devserver reached at loopback over a
//! tunnel). It stays off `outbound-*` (ad-hoc remote-URL attach): the panel
//! reads local file bytes, so an untrusted remote-served webview must not be
//! able to pop it. The picker is user-interactive and `cs upload` is
//! user-initiated, so the static tunnel-window grant is acceptable; a
//! per-gesture handshake ACL is the follow-up.

use serde::Serialize;
use tauri::AppHandle;

/// One picked file: its base name and full bytes. Bytes cross the IPC bridge as
/// a JSON number array (fine at notes scale; a very large pick is heavy — the
/// SPA caller wraps these straight into `File` objects without re-reading).
#[derive(Serialize)]
pub struct PickedUploadFile {
    name: String,
    bytes: Vec<u8>,
}

/// Open a native multi-file picker and return the chosen files' bytes. `Ok([])`
/// when the user cancels (the SPA treats an empty result as a no-op).
///
/// The picker is async-callback based and Tauri dialogs must run on the main
/// thread, so we hop onto it, raise the panel, and complete a oneshot from the
/// picker callback with the chosen PATHS; the bytes are then read back here off
/// the main thread, so even a large pick never blocks the UI thread mid-read.
#[tauri::command]
pub async fn pick_upload_files(app: AppHandle) -> Result<Vec<PickedUploadFile>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    let app_for_dialog = app.clone();
    app.run_on_main_thread(move || {
        app_for_dialog.dialog().file().pick_files(move |chosen| {
            // `None` = cancelled -> empty path list -> Ok([]) below.
            let paths: Vec<std::path::PathBuf> = chosen
                .unwrap_or_default()
                .into_iter()
                .filter_map(|fp| fp.into_path().ok())
                .collect();
            let _ = tx.send(paths);
        });
    })
    .map_err(|e| format!("scheduling the upload picker failed: {e}"))?;

    let paths = rx
        .await
        .map_err(|e| format!("upload picker was dropped: {e}"))?;

    let mut picked = Vec::with_capacity(paths.len());
    for path in paths {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| format!("picked path has no file name: {}", path.display()))?;
        let bytes = std::fs::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        picked.push(PickedUploadFile { name, bytes });
    }
    Ok(picked)
}
