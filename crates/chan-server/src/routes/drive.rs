//! `/api/drive` — drive metadata + the cloud-drives detection helper.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use super::preferences::{preferences_view, PreferencesView};
use crate::error::err;
use crate::state::AppState;

#[derive(Serialize)]
struct DriveInfo {
    /// Absolute drive root, POSIX-style on every platform so the
    /// JSON shape stays stable. Empty string on `--tunnel-public`
    /// runs: the absolute path of the owner's drive would otherwise
    /// reveal the owner's username and filesystem layout to every
    /// anonymous visitor.
    root: String,
    /// Path-derived label for compact UI surfaces. It is not stored
    /// in the registry and cannot be edited through `/api/drive`.
    label: Option<String>,
    /// Stable metadata storage key under `~/.chan/drives/`.
    metadata_key: Option<String>,
    /// Per-device preferences view. The frontend uses this to seed
    /// the editor (fonts, theme, line spacing) without a follow-up
    /// /api/config round-trip. Same shape as
    /// `GlobalConfig.preferences`; assembled by joining EditorPrefs
    /// and ServerConfig.
    preferences: PreferencesView,
}

pub async fn api_get_drive(State(state): State<Arc<AppState>>) -> Response {
    match drive_info(&state) {
        Ok(info) => Json(info).into_response(),
        Err(message) => err(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub async fn api_patch_drive(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    // Kept as a compatibility endpoint while the frontend drops its
    // former drive-name editor. Local drive names are no longer a
    // mutable registry field.
    if body.get("name").is_some() {
        return (StatusCode::BAD_REQUEST, "drive names are not supported").into_response();
    }
    match drive_info(&state) {
        Ok(info) => Json(info).into_response(),
        Err(message) => err(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

#[derive(Serialize)]
struct CloudDriveJson {
    provider: String,
    provider_root: String,
    suggested_root: String,
}

pub async fn api_cloud_drives(State(state): State<Arc<AppState>>) -> Response {
    // The detection walks the owner's home dir for Dropbox / iCloud
    // / Google Drive / OneDrive locations. The result reveals which
    // cloud providers the owner is signed into and the absolute
    // paths of their sync roots. Anonymous visitors get an empty
    // list; the SPA's "register a drive" picker is unreachable
    // anyway when Settings is locked, so the only consumer is the
    // owner running locally.
    if state.tunnel_public {
        return Json(Vec::<CloudDriveJson>::new()).into_response();
    }
    match tokio::task::spawn_blocking(move || {
        let out: Vec<CloudDriveJson> = chan_drive::paths::detected_cloud_drives()
            .into_iter()
            .map(|c| CloudDriveJson {
                provider: c.provider,
                provider_root: c.provider_root.to_string_lossy().into_owned(),
                suggested_root: c.suggested_root.to_string_lossy().into_owned(),
            })
            .collect();
        Json(out).into_response()
    })
    .await
    {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("cloud drives task panicked: {e}"),
        )
            .into_response(),
    }
}

/// Build a `DriveInfo` from current registry state.
///
/// `root` is blanked on `--tunnel-public` runs so the owner's
/// absolute filesystem path does not leak to anonymous visitors.
/// The SPA tolerates an empty `root`: it only uses the field for
/// the Settings panel's "Drive root" line, which is unreachable
/// in tunnel mode anyway.
fn drive_info(state: &AppState) -> Result<DriveInfo, String> {
    let drives = state.library.list_drives();
    // Snapshot the live drive once: each call to `state.drive()`
    // takes the `drive_cell` RwLock and clones the Arc. Two calls
    // worked fine; one call reads slightly cleaner and survives a
    // hypothetical reset-in-flight where the cell could swap
    // between the registry lookup and the path serialization.
    let drive = state.drive();
    let drive_root = drive.root();
    let entry = drives.iter().find(|d| d.root_path.as_path() == drive_root);
    let root = if state.tunnel_public {
        String::new()
    } else {
        drive_root.to_string_lossy().into_owned()
    };
    Ok(DriveInfo {
        root,
        label: entry
            .and_then(|e| e.root_path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_string),
        metadata_key: entry.map(|e| e.metadata_key.clone()),
        preferences: preferences_view(state).map_err(|e| e.to_string())?,
    })
}
