//! `/api/drive` — drive metadata + the cloud-drives detection helper.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::preferences::{preferences_view, PreferencesView};
use crate::error::err_from;
use crate::state::AppState;

#[derive(Serialize)]
struct DriveInfo {
    /// User-facing display name from the registry. None when the
    /// drive has no name set; the frontend falls back to the
    /// basename of `root` for display.
    name: Option<String>,
    /// Absolute drive root, POSIX-style on every platform so the
    /// JSON shape stays stable. Empty string on `--tunnel-public`
    /// runs: the absolute path of the owner's drive would otherwise
    /// reveal the owner's username and filesystem layout to every
    /// anonymous visitor.
    root: String,
    /// Per-device preferences view. The frontend uses this to seed
    /// the editor (fonts, theme, line spacing) without a follow-up
    /// /api/config round-trip. Same shape as
    /// `GlobalConfig.preferences`; assembled by joining
    /// EditorPrefs + ServerConfig + LlmConfig.
    preferences: PreferencesView,
}

pub async fn api_get_drive(State(state): State<Arc<AppState>>) -> Response {
    Json(drive_info(&state)).into_response()
}

#[derive(Deserialize)]
pub struct PatchDriveBody {
    /// Empty string clears the name (the basename takes over for
    /// display). Field absent in the body is a no-op so the same
    /// PATCH endpoint can grow other fields later without each
    /// caller having to pass them.
    #[serde(default)]
    name: Option<String>,
}

pub async fn api_patch_drive(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PatchDriveBody>,
) -> Response {
    // The `tunnel_guard::settings_guard` middleware already refused
    // this call when settings are disabled; no per-handler gate.
    if let Some(name) = body.name {
        let new_name = if name.is_empty() { None } else { Some(name) };
        if let Err(e) = state.library.rename_drive(state.drive().root(), new_name) {
            return err_from(&e);
        }
    }
    Json(drive_info(&state)).into_response()
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
    let out: Vec<CloudDriveJson> = chan_drive::paths::detected_cloud_drives()
        .into_iter()
        .map(|c| CloudDriveJson {
            provider: c.provider,
            provider_root: c.provider_root.to_string_lossy().into_owned(),
            suggested_root: c.suggested_root.to_string_lossy().into_owned(),
        })
        .collect();
    Json(out).into_response()
}

/// Build a `DriveInfo` from current registry state. Re-reads the
/// registry on every call so a CLI-side `chan rename` immediately
/// reflects in the next /api/drive response.
///
/// `root` is blanked on `--tunnel-public` runs so the owner's
/// absolute filesystem path does not leak to anonymous visitors.
/// The SPA tolerates an empty `root`: it only uses the field for
/// the Settings panel's "Drive root" line, which is unreachable
/// in tunnel mode anyway.
fn drive_info(state: &AppState) -> DriveInfo {
    let drives = state.library.list_drives();
    // Snapshot the live drive once: each call to `state.drive()`
    // takes the `drive_cell` RwLock and clones the Arc. Two calls
    // worked fine; one call reads slightly cleaner and survives a
    // hypothetical reset-in-flight where the cell could swap
    // between the registry lookup and the path serialization.
    let drive = state.drive();
    let drive_root = drive.root();
    let entry = drives.iter().find(|d| d.path.as_path() == drive_root);
    let root = if state.tunnel_public {
        String::new()
    } else {
        drive_root.to_string_lossy().into_owned()
    };
    DriveInfo {
        name: entry.and_then(|e| e.name.clone()),
        root,
        preferences: preferences_view(state),
    }
}
