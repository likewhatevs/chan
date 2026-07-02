//! Admin endpoints for the chan-gateway-admin CLI and the sibling
//! services (identity reads the per-user snapshot for `/api/me`;
//! identity + profile call the kill routes on revoke / delete / block).
//!
//! Routes under `/admin/v1/`:
//!
//!   * `GET    /tunnels`                         snapshot of every registered tunnel
//!   * `POST   /tunnels/{user}/{workspace}/kill` force one tunnel offline
//!   * `GET    /users/{user}/tunnels`            per-user snapshot
//!   * `POST   /users/{user}/tunnels/kill`       bulk evict for one user
//!   * `GET    /tunnels/watch`                   SSE stream of periodic snapshots
//!
//! All gated by a single bearer token (`DEVSERVER_ADMIN_TOKEN`). The
//! tunnel registry is in-memory and process-local, so admin reads
//! always see exactly what this devserver-proxy instance is serving --
//! there is no cross-process aggregation because the deployment runs
//! a single devserver-proxy.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use futures_util::stream::{self, Stream, StreamExt};
use gateway_common::validators::valid_username;
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::error::Error;
use crate::http::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct TunnelView {
    pub user: String,
    /// The registration's second key: the devserver id (one devserver
    /// per user). identity reads this to mint a `drv` that matches the
    /// owner's LIVE devserver, so a rotated owner's old grants correctly
    /// 404 until re-share.
    pub devserver_id: String,
    pub peer_addr: Option<String>,
    pub connected_at: DateTime<Utc>,
}

pub fn router(state: AppState) -> Router<AppState> {
    // No per-IP rate limit on the admin tree. tower_governor keyed on
    // the connection peer degenerates into a single global bucket
    // behind nginx (all admin traffic arrives from one upstream IP),
    // and a noisy automated guess loop would lock out the operator
    // CLI instead of throttling the attacker. The constant-time
    // bearer compare (admin_auth) already prevents byte-by-byte
    // leaks; the bearer is a 32-byte hex secret so brute-force is
    // bounded by network RTT * 2^256. nginx is the rate-limit layer
    // for this surface; if the surface ever moves off the trusted
    // network, swap in an XFF-aware governor instead.
    Router::new()
        .route("/admin/v1/tunnels", get(list_tunnels))
        .route(
            "/admin/v1/tunnels/{user}/{workspace}/kill",
            post(kill_tunnel),
        )
        .route("/admin/v1/users/{user}/tunnels", get(list_user_tunnels))
        .route(
            "/admin/v1/users/{user}/tunnels/kill",
            post(kill_user_tunnels),
        )
        .route("/admin/v1/tunnels/watch", get(watch_tunnels))
        .route_layer(middleware::from_fn_with_state(state, admin_auth))
}

async fn admin_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> std::result::Result<axum::response::Response, Error> {
    let admin = state
        .cfg
        .admin_token
        .as_deref()
        .ok_or(Error::Unauthorized)?;
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    // Constant-time compare: leaks length only.
    match provided {
        Some(t) if bool::from(t.as_bytes().ct_eq(admin.as_bytes())) => Ok(next.run(request).await),
        _ => Err(Error::Unauthorized),
    }
}

async fn list_tunnels(State(state): State<AppState>) -> Json<Vec<TunnelView>> {
    Json(snapshot(&state))
}

/// Per-user snapshot. Used by identity-service's `/api/me` to merge
/// the live-workspace list into the dashboard response. "User has nothing
/// connected" returns an empty array, not a 404, so callers do not
/// special-case the steady state where a fresh sign-in has zero
/// workspaces.
async fn list_user_tunnels(
    State(state): State<AppState>,
    Path(user): Path<String>,
) -> std::result::Result<Json<Vec<TunnelView>>, Error> {
    // Reject garbage path segments before walking the registry. A
    // well-formed username can never match outside the [a-z0-9-]
    // shape, so anything else is an attempted probe or a misconfigured
    // caller; same 404 we'd return for "no tunnels live" keeps the
    // surface uniform.
    if !valid_username(&user) {
        return Err(Error::NotFound);
    }
    let rows = state
        .registry
        .list_all_tunnels()
        .into_iter()
        .filter(|t| t.user.as_ref() == user)
        .map(|t| TunnelView {
            user: t.user.as_ref().to_string(),
            devserver_id: t.workspace.as_ref().to_string(),
            peer_addr: t.peer_addr.map(|a| a.to_string()),
            connected_at: t.connected_at,
        })
        .collect();
    Ok(Json(rows))
}

async fn kill_tunnel(
    State(state): State<AppState>,
    Path((user, workspace)): Path<(String, String)>,
) -> std::result::Result<StatusCode, Error> {
    if state.registry.evict(&user, &workspace) {
        tracing::info!(%user, %workspace, "tunnel evicted by admin");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound)
    }
}

#[derive(Debug, Serialize)]
struct KillUserTunnelsResponse {
    killed: usize,
}

/// Bulk-evict every tunnel for `user`. Called by identity-service on
/// account-delete; "nothing to kill" is fine (returns 0). Idempotent
/// so a retry after a transient failure is safe.
async fn kill_user_tunnels(
    State(state): State<AppState>,
    Path(user): Path<String>,
) -> std::result::Result<Json<KillUserTunnelsResponse>, Error> {
    if !valid_username(&user) {
        return Err(Error::NotFound);
    }
    let killed = state.registry.evict_all_for_user(&user);
    if killed > 0 {
        tracing::info!(%user, killed, "user tunnels evicted by admin");
    }
    Ok(Json(KillUserTunnelsResponse { killed }))
}

/// Periodic snapshot stream. Workspace-proxy doesn't currently emit
/// register/deregister events to a broadcast channel, so we tick
/// once a second and let the CLI render. Cheap: lock + clone of a
/// small HashMap. Bumps to event-driven when there's evidence the
/// snapshot rate doesn't keep up with operator expectations.
async fn watch_tunnels(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let initial_state = state.clone();
    let initial = stream::once(async move { Ok::<_, Infallible>(snap_event(&initial_state)) });

    let interval = tokio::time::interval(Duration::from_millis(1000));
    let ticks = stream::unfold((state, interval), |(state, mut interval)| async move {
        interval.tick().await;
        let event = snap_event(&state);
        Some((Ok::<_, Infallible>(event), (state, interval)))
    });

    Sse::new(initial.chain(ticks)).keep_alive(KeepAlive::default())
}

fn snap_event(state: &AppState) -> Event {
    Event::default()
        .event("snapshot")
        .json_data(snapshot(state))
        .unwrap_or_else(|_| Event::default().data("[]"))
}

fn snapshot(state: &AppState) -> Vec<TunnelView> {
    state
        .registry
        .list_all_tunnels()
        .into_iter()
        .map(|t| TunnelView {
            user: t.user.as_ref().to_string(),
            devserver_id: t.workspace.as_ref().to_string(),
            peer_addr: t.peer_addr.map(|a| a.to_string()),
            connected_at: t.connected_at,
        })
        .collect()
}
