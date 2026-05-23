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
use chan_drive::ChanError;
use serde::{Deserialize, Serialize};

use crate::bus::make_watch_bridge;
use crate::error::{err, err_from};
use crate::state::AppState;

/// systacean-41: map `chan_drive::ChanError` from
/// `Drive::create_team` / `Drive::duplicate_team` to HTTP. The
/// chan-drive layer returns `ChanError::Io` with descriptive
/// messages for each validation failure; this matcher promotes
/// the relevant variants to 400 per the task spec (`Invalid name
/// (empty, traversal, collision) → 400`). Falls through to the
/// generic `err_from` for everything else.
fn map_team_error(e: &ChanError) -> Response {
    if let ChanError::Io(msg) = e {
        // Validation failures + collisions all promote to 400 per
        // the task body. Source name "not found" on duplicate
        // stays 404 via the existing `err_from` rule.
        let lower = msg.to_lowercase();
        if lower.contains("cannot be empty")
            || lower.contains("must not contain")
            || lower.contains("is reserved")
            || lower.contains("already exists")
            || lower.contains("source and new name are identical")
        {
            return err(StatusCode::BAD_REQUEST, msg.clone());
        }
    }
    err_from(e)
}

/// systacean-41: `POST /api/teams` request body. The outer
/// `name` is authoritative — if `config.team_name` disagrees,
/// the server overwrites `config.team_name` with `name` before
/// calling `Drive::create_team`. Avoids "which one wins?"
/// ambiguity in `-a-79`'s SPA orchestrator.
#[derive(Deserialize)]
pub struct CreateTeamPayload {
    pub name: String,
    pub config: chan_drive::TeamConfig,
}

/// systacean-41: `POST /api/teams/{name}/duplicate` request body.
#[derive(Deserialize)]
pub struct DuplicateTeamPayload {
    pub new_name: String,
}

#[derive(Serialize)]
pub struct TeamRefView {
    pub name: String,
    pub abs: String,
}

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

/// systacean-41: `POST /api/teams` — create a new team workspace.
///
/// The outer `name` is authoritative; if the inbound config's
/// `team_name` differs, we overwrite it before calling
/// `Drive::create_team`. Returns the created `TeamRef` so the
/// SPA orchestrator can plumb the path into the subsequent
/// `load` call.
///
/// Errors:
/// * empty / traversal / collision → 400 (per task spec).
/// * other I/O failure → 500 via `err_from`.
///
/// systacean-42: **idempotency contract for the SPA orchestrator.**
/// Calling `POST /api/teams` for a name that ALREADY exists
/// returns **400 with `already exists` in the response body**.
/// This is the option (C) outcome from `-42`'s task body: the
/// SPA detects "already exists" + treats it as a no-op success
/// for the bootstrap-on-existing flow.
///
/// Rationale: a silent no-op-on-existing would mask a real user
/// mistake (typo on team name colliding with an unrelated team)
/// and overwrite-on-existing would corrupt the existing config.
/// Returning a structured error preserves both safety + lets the
/// SPA layer make the call.
pub async fn api_team_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTeamPayload>,
) -> Response {
    let CreateTeamPayload { name, mut config } = payload;
    // Outer `name` is authoritative.
    config.team_name = name;
    let drive = state.drive().clone();
    let result = tokio::task::spawn_blocking(move || drive.create_team(&config)).await;
    match result {
        Ok(Ok(team_ref)) => Json(TeamRefView {
            name: team_ref.name,
            abs: team_ref.abs.display().to_string(),
        })
        .into_response(),
        Ok(Err(e)) => map_team_error(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

/// systacean-41: `POST /api/teams/{name}/duplicate` — copy an
/// existing team workspace.
///
/// The path `{name}` is the source; the request body's
/// `new_name` is the duplicate's name. `Drive::duplicate_team`
/// byte-copies the workspace (config + events + docs) +
/// rewrites the duplicated `config.toml`'s `team_name` to
/// `new_name` so the team's identity matches its directory.
///
/// Errors:
/// * empty / traversal / collision / identical source-and-new
///   → 400 (per task spec).
/// * source team not found → 404 via `err_from`'s "not found"
///   detector.
pub async fn api_team_duplicate(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
    Json(payload): Json<DuplicateTeamPayload>,
) -> Response {
    let DuplicateTeamPayload { new_name } = payload;
    let drive = state.drive().clone();
    let result =
        tokio::task::spawn_blocking(move || drive.duplicate_team(&team_name, &new_name)).await;
    match result {
        Ok(Ok(team_ref)) => Json(TeamRefView {
            name: team_ref.name,
            abs: team_ref.abs.display().to_string(),
        })
        .into_response(),
        Ok(Err(e)) => map_team_error(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

/// systacean-42: `GET /api/teams/:name/config` — read the
/// persisted `TeamConfig` for a team. Backs @@FullStackA's
/// `-a-80 slice 2` Load Team dialog (the dialog populates from
/// this endpoint before the user confirms Bootstrap).
///
/// Returns the same `TeamConfig` JSON shape that `POST
/// /api/teams`'s `config` field expects, so a `GET → mutate →
/// POST` round-trip pipeline (e.g. "edit existing team") works
/// without any client-side adapter layer.
///
/// Errors:
/// * Team directory missing → 404 via `err_from`'s "not found"
///   detector on the underlying `chan_drive::teams::load` error.
/// * Malformed config.toml → 500 via the generic `err_from`
///   fallback.
pub async fn api_team_get_config(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
) -> Response {
    let drive = state.drive().clone();
    let result = tokio::task::spawn_blocking(move || drive.load_team(&team_name)).await;
    match result {
        Ok(Ok(config)) => Json(config).into_response(),
        Ok(Err(e)) => map_team_error(&e),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Mutex, RwLock};

    use axum::body::Body;
    use axum::http::{header, Request};
    use chan_drive::SearchAggression;
    use tempfile::TempDir;
    use tokio::sync::{broadcast, watch};
    use tower::ServiceExt;

    use crate::self_writes::SelfWrites;
    use crate::state::DriveCell;
    use crate::terminal_sessions::{Registry as TerminalRegistry, RegistryConfig};
    use crate::{EditorPrefs, ServerConfig};

    struct RouteTestApp {
        _cfg: TempDir,
        _root: TempDir,
        state: Arc<AppState>,
    }

    fn route_test_app() -> RouteTestApp {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let lib = chan_drive::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_drive(root.path()).unwrap();
        let drive = lib.open_drive(root.path()).unwrap();

        let (events_tx, _) = broadcast::channel::<String>(1);
        let (index_events_tx, _) = broadcast::channel::<chan_drive::WatchEvent>(1);
        let indexer = Arc::new(crate::indexer::Indexer::spawn(
            drive.clone(),
            index_events_tx.subscribe(),
            false,
            SearchAggression::Conservative,
            Arc::new(chan_drive::NoProgress),
        ));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        std::mem::forget(shutdown_tx);

        let state = Arc::new(AppState {
            library: lib,
            drive_root: root.path().to_path_buf(),
            drive_cell: Arc::new(RwLock::new(Some(DriveCell {
                drive,
                watch_handle: None,
                indexer,
            }))),
            token: Some("secret".to_string()),
            prefix: Arc::new(RwLock::new(String::new())),
            settings_disabled: false,
            tunnel_public: false,
            last_activity: Arc::new(AtomicU64::new(0)),
            events_tx,
            index_events_tx,
            server_config: Mutex::new(ServerConfig::default()),
            editor_prefs: Mutex::new(EditorPrefs::default()),
            self_writes: Arc::new(SelfWrites::new()),
            terminal_sessions: Arc::new(TerminalRegistry::new(RegistryConfig {
                drive_root: root.path().to_path_buf(),
                mcp_socket_path: None,
                control_socket_path: None,
                terminal: ServerConfig::default().terminal,
            })),
            shutdown_rx,
            loaded_teams: std::sync::Mutex::new(std::collections::HashMap::new()),
        });

        RouteTestApp {
            _cfg: cfg,
            _root: root,
            state,
        }
    }

    fn sample_config(name: &str) -> serde_json::Value {
        serde_json::json!({
            "team_name": name,
            "host_name": "Alex",
            "host_handle": "@@Alex",
            "auto_prefix_at": true,
            "created_at": "2026-05-23T03:30:00Z",
            "members": [],
        })
    }

    async fn request(
        router: &axum::Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, "Bearer secret");
        let body = if let Some(b) = body {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(b.to_string())
        } else {
            Body::empty()
        };
        let response = router
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn create_team_round_trip_then_load_succeeds() {
        // systacean-41: POST /api/teams creates the workspace +
        // returns the TeamRef. A subsequent POST
        // /api/teams/:name/load watcher attach must succeed on
        // the newly-created team.
        let app = route_test_app();
        let router = crate::router(app.state);

        let (status, body) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "alpha");

        // Verify Load on the newly-created team works.
        let (status, body) = request(&router, "POST", "/api/teams/alpha/load", None).await;
        assert_eq!(status, StatusCode::OK, "load failed: body={body:?}");

        // Verify it shows up in /loaded.
        let (status, body) = request(&router, "GET", "/api/teams/loaded", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["teams"][0], "alpha");
    }

    #[tokio::test]
    async fn duplicate_team_creates_distinct_copy() {
        // systacean-41: duplicating an existing team produces a
        // distinct workspace under the new name. The duplicate's
        // config.team_name is rewritten by chan-drive to match
        // the new directory name.
        let app = route_test_app();
        let router = crate::router(app.state);

        let _ = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;

        let (status, body) = request(
            &router,
            "POST",
            "/api/teams/alpha/duplicate",
            Some(serde_json::json!({"new_name": "beta"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "beta");

        // Load both — they should be independent.
        let (status, _) = request(&router, "POST", "/api/teams/alpha/load", None).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request(&router, "POST", "/api/teams/beta/load", None).await;
        assert_eq!(status, StatusCode::OK);

        let (_, body) = request(&router, "GET", "/api/teams/loaded", None).await;
        let teams: Vec<&str> = body["teams"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(teams.contains(&"alpha"));
        assert!(teams.contains(&"beta"));
    }

    #[tokio::test]
    async fn create_team_rejects_empty_name() {
        // systacean-41: validation failures promote to 400.
        let app = route_test_app();
        let router = crate::router(app.state);
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "",
                "config": sample_config(""),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_team_rejects_path_traversal() {
        // systacean-41: names containing path separators → 400.
        let app = route_test_app();
        let router = crate::router(app.state);
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "evil/escape",
                "config": sample_config("evil/escape"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_team_rejects_collision() {
        // systacean-41: creating a team that already exists → 400.
        let app = route_test_app();
        let router = crate::router(app.state);
        let _ = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn duplicate_team_rejects_identical_source_and_new_name() {
        // systacean-41: chan-drive refuses
        // duplicate(source, source) as a guardrail. Route maps
        // to 400.
        let app = route_test_app();
        let router = crate::router(app.state);
        let _ = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams/alpha/duplicate",
            Some(serde_json::json!({"new_name": "alpha"})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn duplicate_team_rejects_missing_source() {
        // systacean-41: duplicate of a non-existent source → 404
        // via err_from's "not found" detector.
        let app = route_test_app();
        let router = crate::router(app.state);
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams/ghost/duplicate",
            Some(serde_json::json!({"new_name": "newghost"})),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_team_config_round_trips_with_post() {
        // systacean-42: POST + GET produce matching JSON. Pins
        // the SPA-side `api.teamGetConfig(name)` consumer contract.
        let app = route_test_app();
        let router = crate::router(app.state);

        let config = sample_config("alpha");
        let (status, _) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": config.clone(),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = request(&router, "GET", "/api/teams/alpha/config", None).await;
        assert_eq!(status, StatusCode::OK);
        // POST overwrote config.team_name with outer "alpha";
        // the GET response should reflect that.
        assert_eq!(body["team_name"], "alpha");
        assert_eq!(body["host_name"], config["host_name"]);
        assert_eq!(body["host_handle"], config["host_handle"]);
        assert_eq!(body["auto_prefix_at"], config["auto_prefix_at"]);
        assert_eq!(body["created_at"], config["created_at"]);
        assert_eq!(body["members"], config["members"]);
    }

    #[tokio::test]
    async fn get_team_config_returns_404_when_missing() {
        // systacean-42: missing team → 404 via err_from's "not
        // found" detector on the underlying teams::load message.
        let app = route_test_app();
        let router = crate::router(app.state);
        let (status, _) = request(&router, "GET", "/api/teams/ghost/config", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_team_returns_400_on_existing_team_for_spa_idempotency() {
        // systacean-42: PIN the documented idempotency contract.
        // Re-creating an existing team returns 400 with
        // `already exists` in the body. The SPA orchestrator
        // detects this + treats as no-op success for the
        // bootstrap-on-existing flow.
        let app = route_test_app();
        let router = crate::router(app.state);

        let (status, _) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                "config": sample_config("alpha"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // The "already exists" marker must appear in the body so
        // the SPA can detect + treat as no-op.
        let body_str = body.to_string();
        assert!(
            body_str.contains("already exists"),
            "response body must carry the `already exists` marker; got {body_str}"
        );
    }

    #[tokio::test]
    async fn outer_name_overrides_config_team_name() {
        // systacean-41: per the route doc-comment, the outer
        // `name` is authoritative — if the inbound config's
        // `team_name` disagrees, the server overwrites it. Avoids
        // SPA-side "which one wins?" ambiguity in `-a-79`.
        let app = route_test_app();
        let router = crate::router(app.state);
        let (status, body) = request(
            &router,
            "POST",
            "/api/teams",
            Some(serde_json::json!({
                "name": "alpha",
                // config.team_name is intentionally different
                "config": sample_config("DISAGREES"),
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Created team's name matches the outer `name`, not the
        // config's value.
        assert_eq!(body["name"], "alpha");
    }
}
