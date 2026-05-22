//! systacean-31: multi-team watcher orchestration.
//!
//! Per the addendum-b Team feature spec, each loaded team has its
//! own `WatchHandle` rooted at the team's `events/` subdirectory
//! (`<drafts_dir>/team-{name}/events/`). Events emerge prefixed
//! as `team-{name}/events/<file>` so the SPA event-stream can
//! route per-team.
//!
//! Lifecycle is non-destructive per the
//! [`addendum-b clarification`](../../alex/addendum-b.md)
//! tear-down semantic: `team_unload` drops the watcher but the
//! on-disk workspace (config + events + docs) PERSISTS, and any
//! open terminals stay attached (user-managed). Re-load via the
//! normal Load Team flow at any time.
//!
//! Per-team isolated `WatchHandle` rather than a single shared
//! handle with multi-roots — the architect's spec called out
//! either as acceptable; isolated handles read cleaner for
//! lifecycle (drop = unwatch).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::bus::make_watch_bridge;
use crate::error::{err, err_from};
use crate::state::AppState;

#[derive(Serialize)]
pub struct TeamLoadResponse {
    pub team_name: String,
    /// Absolute path of the team's events/ subdir, for diagnostic
    /// purposes (SPA doesn't strictly need it).
    pub events_dir: String,
}

#[derive(Serialize)]
pub struct TeamLoadedListResponse {
    pub teams: Vec<String>,
}

/// `POST /api/teams/{name}/load` — spin up the per-team watcher.
///
/// Idempotent: re-loading an already-loaded team replaces the
/// existing handle (effectively a refresh). The team must already
/// exist on disk (via `Drive::create_team` from `-30`); a
/// non-existent team errors with 404.
pub async fn api_team_load(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
) -> Response {
    let drive = state.drive().clone();
    let team_name_for_task = team_name.clone();
    let result = tokio::task::spawn_blocking(
        move || -> Result<std::path::PathBuf, chan_drive::ChanError> {
            // Validate the team exists + resolve the events dir.
            let events_dir = drive.team_events_dir(&team_name_for_task)?;
            Ok(events_dir)
        },
    )
    .await;
    let events_dir = match result {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return err_from(&e),
        Err(join) => return err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    };

    // Build the watch bridge (re-uses the same events_tx /
    // index_events_tx fan-out as the drive-root watcher).
    let bridge = make_watch_bridge(&state.events_tx, &state.index_events_tx, &state.self_writes);

    // `Drive::watch_team` wraps the WatchRoot construction +
    // path-prefix logic so chan-server doesn't construct
    // `WatchRoot` directly. Per-event paths emerge prefixed
    // `team-{name}/events/` so the SPA event stream routes
    // per-team.
    let drive = state.drive().clone();
    let watch_handle = match drive.watch_team(&team_name, bridge) {
        Ok(h) => h,
        Err(e) => return err_from(&e),
    };

    let mut loaded = state.loaded_teams.lock().unwrap();
    // Replace any existing handle for this team (drops + closes
    // the old watcher cleanly).
    loaded.insert(team_name.clone(), watch_handle);
    drop(loaded);

    Json(TeamLoadResponse {
        team_name,
        events_dir: events_dir.display().to_string(),
    })
    .into_response()
}

/// `POST /api/teams/{name}/unload` — tear down the per-team
/// watcher. Non-destructive: workspace + terminals persist.
/// Returns 404 if the team isn't currently loaded.
pub async fn api_team_unload(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
) -> Response {
    let mut loaded = state.loaded_teams.lock().unwrap();
    match loaded.remove(&team_name) {
        Some(_handle) => {
            // Dropping the handle releases the notify watcher
            // + the dispatcher thread exits via the closure's
            // weak references.
            Json(TeamLoadResponse {
                team_name,
                events_dir: String::new(),
            })
            .into_response()
        }
        None => err(
            StatusCode::NOT_FOUND,
            format!("team `{team_name}` not loaded"),
        ),
    }
}

/// `GET /api/teams/loaded` — list currently loaded teams.
pub async fn api_team_list_loaded(State(state): State<Arc<AppState>>) -> Response {
    let loaded = state.loaded_teams.lock().unwrap();
    let mut teams: Vec<String> = loaded.keys().cloned().collect();
    drop(loaded);
    teams.sort();
    Json(TeamLoadedListResponse { teams }).into_response()
}
