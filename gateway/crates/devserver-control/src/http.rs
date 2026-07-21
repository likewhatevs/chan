use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use gateway_common::validators::valid_username;
use serde::Serialize;
use subtle::ConstantTimeEq;
use tokio::sync::watch;

use crate::{ActorError, CommandOutcome, ControllerHandle, KillPlan, ProxyView, TunnelView};

#[derive(Clone)]
struct AppState {
    controller: ControllerHandle,
    admin_token: Arc<[u8]>,
}

pub fn router(controller: ControllerHandle, admin_token: String) -> Router {
    let state = AppState {
        controller,
        admin_token: Arc::from(admin_token.into_bytes()),
    };
    let admin = Router::new()
        .route("/admin/v1/tunnels", get(list_tunnels))
        .route("/admin/v1/tunnels/watch", get(watch_tunnels))
        .route(
            "/admin/v1/tunnels/{user}/{devserver_id}/kill",
            post(kill_tunnel),
        )
        .route("/admin/v1/users/{user}/tunnels", get(list_user_tunnels))
        .route(
            "/admin/v1/users/{user}/tunnels/kill",
            post(kill_user_tunnels),
        )
        .route("/admin/v1/proxies", get(list_proxies))
        .route("/admin/v1/proxies/watch", get(watch_proxies))
        .route_layer(middleware::from_fn_with_state(state.clone(), admin_auth));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(admin)
        .with_state(state)
}

async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok\n")
}

async fn readyz(State(state): State<AppState>) -> (StatusCode, &'static str) {
    match state.controller.is_ready().await {
        Ok(true) => (StatusCode::OK, "ready\n"),
        Ok(false) | Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "warming\n"),
    }
}

async fn list_tunnels(State(state): State<AppState>) -> Result<Json<Vec<TunnelView>>, StatusCode> {
    state
        .controller
        .tunnels()
        .await
        .map(Json)
        .map_err(admin_error)
}

async fn list_proxies(State(state): State<AppState>) -> Result<Json<Vec<ProxyView>>, StatusCode> {
    state
        .controller
        .proxies()
        .await
        .map(Json)
        .map_err(admin_error)
}

/// Per-user aggregate snapshot. A well-formed user with nothing live
/// returns an empty array, not a 404, so callers do not special-case the
/// steady state; a malformed username is a probe and gets the same 404
/// shape as any unknown target.
async fn list_user_tunnels(
    State(state): State<AppState>,
    Path(user): Path<String>,
) -> Result<Json<Vec<TunnelView>>, Response> {
    if !valid_username(&user) {
        return Err(not_found());
    }
    let tunnels = state
        .controller
        .tunnels()
        .await
        .map_err(|error| admin_error(error).into_response())?;
    Ok(Json(
        tunnels.into_iter().filter(|row| row.user == user).collect(),
    ))
}

/// Exact kill of one aggregate row. The state machine routes the command
/// by the registration UUID read at issue time, so a delayed command
/// cannot kill a successor registration for the same key.
async fn kill_tunnel(
    State(state): State<AppState>,
    Path((user, devserver_id)): Path<(String, String)>,
) -> Result<StatusCode, Response> {
    let plan = state
        .controller
        .plan_tunnel_kill(&user, &devserver_id)
        .await
        .map_err(|error| admin_error(error).into_response())?;
    let KillPlan::Issued(confirmations) = plan else {
        return Err(not_found());
    };
    let confirmation = confirmations
        .into_iter()
        .next()
        .expect("exact kill issues exactly one command");
    match confirmation.await {
        Ok(CommandOutcome::Confirmed { .. }) => Ok(StatusCode::NO_CONTENT),
        // A dropped sender means the command settled without a reachable
        // waiter (actor restart); the proxy may have executed the kill,
        // so the honest answer is partial rather than success.
        Ok(_) | Err(_) => Err(partial_kill(0)),
    }
}

/// User-wide kill, fanned out as one command per owning proxy. Await
/// every confirmation so a partial failure reports the count that is
/// actually gone; the operation stays idempotent and a retry kills any
/// surviving rows.
async fn kill_user_tunnels(
    State(state): State<AppState>,
    Path(user): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    if !valid_username(&user) {
        return Err(not_found());
    }
    let plan = state
        .controller
        .plan_user_kill(&user)
        .await
        .map_err(|error| admin_error(error).into_response())?;
    let confirmations = match plan {
        KillPlan::Issued(confirmations) => confirmations,
        KillPlan::NotFound => Vec::new(),
    };
    let mut killed = 0;
    let mut partial = false;
    for outcome in futures_util::future::join_all(confirmations).await {
        match outcome {
            Ok(CommandOutcome::Confirmed {
                killed: gone,
                missing,
            }) => killed += gone + missing,
            Ok(_) | Err(_) => partial = true,
        }
    }
    if partial {
        return Err(partial_kill(killed));
    }
    Ok(Json(serde_json::json!({ "killed": killed })))
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not found" })),
    )
        .into_response()
}

fn partial_kill(killed: usize) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({ "error": "partial kill", "killed": killed })),
    )
        .into_response()
}

async fn watch_tunnels(
    State(state): State<AppState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let readiness = state.controller.watch_readiness();
    if !*readiness.borrow() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(
        Sse::new(snapshot_stream(state.controller.watch_tunnels(), readiness))
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15))),
    )
}

async fn watch_proxies(
    State(state): State<AppState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let readiness = state.controller.watch_readiness();
    if !*readiness.borrow() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(
        Sse::new(snapshot_stream(state.controller.watch_proxies(), readiness))
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15))),
    )
}

fn snapshot_stream<T>(
    values: watch::Receiver<Arc<Vec<T>>>,
    readiness: watch::Receiver<bool>,
) -> impl Stream<Item = Result<Event, Infallible>>
where
    T: Serialize + Send + Sync + 'static,
{
    stream::unfold(
        (values, readiness, true),
        |(mut values, mut readiness, first)| async move {
            let mut first = first;
            loop {
                if !*readiness.borrow() {
                    return None;
                }
                if first {
                    first = false;
                    let snapshot = values.borrow_and_update().clone();
                    let event = snapshot_event(snapshot.as_ref())?;
                    return Some((Ok(event), (values, readiness, first)));
                }

                tokio::select! {
                    biased;
                    changed = readiness.changed() => {
                        if changed.is_err() || !*readiness.borrow_and_update() {
                            return None;
                        }
                    }
                    changed = values.changed() => {
                        if changed.is_err() {
                            return None;
                        }
                        let snapshot = values.borrow_and_update().clone();
                        let event = snapshot_event(snapshot.as_ref())?;
                        return Some((Ok(event), (values, readiness, first)));
                    }
                }
            }
        },
    )
}

fn snapshot_event<T: Serialize>(snapshot: &T) -> Option<Event> {
    match Event::default().event("snapshot").json_data(snapshot) {
        Ok(event) => Some(event),
        Err(error) => {
            tracing::error!(error = ?error, "failed to serialize controller watch snapshot");
            None
        }
    }
}

async fn admin_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match provided {
        Some(token) if bool::from(token.as_bytes().ct_eq(state.admin_token.as_ref())) => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn admin_error(error: ActorError) -> StatusCode {
    tracing::warn!(error = ?error, "controller admin read unavailable");
    StatusCode::SERVICE_UNAVAILABLE
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use devserver_control_proto::{CanonicalOrigin, ProxyId, ServerFrame, TunnelRow};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn request(app: Router, path: &str, token: Option<&str>) -> Response {
        request_with_method(app, "GET", path, token).await
    }

    async fn post(app: Router, path: &str, token: Option<&str>) -> Response {
        request_with_method(app, "POST", path, token).await
    }

    async fn request_with_method(
        app: Router,
        method: &str,
        path: &str,
        token: Option<&str>,
    ) -> Response {
        let mut request = axum::http::Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        app.oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn row(user: &str, devserver_id: &str, registration_id: Uuid) -> TunnelRow {
        TunnelRow {
            registration_id,
            user: user.into(),
            devserver_id: devserver_id.into(),
            peer_addr: None,
            connected_at: chrono::Utc::now(),
        }
    }

    async fn begin_proxy(
        controller: &ControllerHandle,
        id: &str,
        rows: Vec<TunnelRow>,
    ) -> (ProxyId, crate::ProxyControlSession) {
        let proxy_id = ProxyId::parse(id).unwrap();
        let session = controller
            .begin_session(
                proxy_id.clone(),
                CanonicalOrigin::parse(&format!("https://{id}.proxy.example.test")).unwrap(),
                env!("CARGO_PKG_VERSION").into(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        controller
            .accept_snapshot(proxy_id.clone(), session.incarnation, 0, rows)
            .await
            .unwrap();
        (proxy_id, session)
    }

    async fn keep_alive_until_ready(
        controller: &ControllerHandle,
        sessions: &mut [(ProxyId, crate::ProxyControlSession)],
    ) {
        for _ in 0..6 {
            tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
            for (proxy_id, session) in sessions.iter_mut() {
                let nonce = loop {
                    if let ServerFrame::Ping { nonce } = session.commands.recv().await.unwrap() {
                        break nonce;
                    }
                };
                controller
                    .pong(proxy_id.clone(), session.incarnation, nonce)
                    .await
                    .unwrap();
            }
        }
    }

    async fn ready_controller() -> ControllerHandle {
        ready_controller_with(Vec::new()).await.0
    }

    async fn ready_controller_with(
        rows: Vec<TunnelRow>,
    ) -> (ControllerHandle, crate::ProxyControlSession) {
        let controller = crate::spawn_controller(100);
        let (proxy_id, session) = begin_proxy(&controller, "p1", rows).await;
        let mut sessions = vec![(proxy_id, session)];
        keep_alive_until_ready(&controller, &mut sessions).await;
        assert!(controller.is_ready().await.unwrap());
        let (_, session) = sessions.pop().unwrap();
        (controller, session)
    }

    async fn recv_kill_command(session: &mut crate::ProxyControlSession) -> (Uuid, Vec<Uuid>) {
        loop {
            let frame = session.commands.recv().await.unwrap();
            if let ServerFrame::KillRegistrations {
                command_id,
                registration_ids,
            } = frame
            {
                return (command_id, registration_ids);
            }
        }
    }

    #[tokio::test]
    async fn health_is_live_while_ready_and_admin_reads_are_warming() {
        let app = router(crate::spawn_controller(100), "secret".into());
        assert_eq!(
            request(app.clone(), "/healthz", None).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            request(app.clone(), "/readyz", None).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            request(app.clone(), "/admin/v1/tunnels", None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(app.clone(), "/admin/v1/tunnels", Some("secret"))
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            request(app, "/admin/v1/tunnels/watch", Some("secret"))
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test(start_paused = true)]
    async fn ready_admin_returns_aggregate_snapshots() {
        let app = router(ready_controller().await, "secret".into());
        assert_eq!(
            request(app.clone(), "/readyz", None).await.status(),
            StatusCode::OK
        );

        let tunnels = request(app.clone(), "/admin/v1/tunnels", Some("secret")).await;
        assert_eq!(tunnels.status(), StatusCode::OK);
        assert_eq!(
            tunnels.into_body().collect().await.unwrap().to_bytes(),
            "[]"
        );

        let proxies = request(app, "/admin/v1/proxies", Some("secret")).await;
        assert_eq!(proxies.status(), StatusCode::OK);
        let body = proxies.into_body().collect().await.unwrap().to_bytes();
        let rows: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["proxy_id"], "p1");
    }

    #[tokio::test]
    async fn kill_routes_and_user_read_are_warming_until_ready() {
        let app = router(crate::spawn_controller(100), "secret".into());
        assert_eq!(
            request(app.clone(), "/admin/v1/users/alice/tunnels", Some("secret"))
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            post(
                app.clone(),
                "/admin/v1/tunnels/alice/one/kill",
                Some("secret")
            )
            .await
            .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            post(app, "/admin/v1/users/alice/tunnels/kill", Some("secret"))
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test(start_paused = true)]
    async fn admin_404_matrix() {
        let (controller, _session) =
            ready_controller_with(vec![row("alice", "one", Uuid::new_v4())]).await;
        let app = router(controller, "secret".into());

        let invalid = request(
            app.clone(),
            "/admin/v1/users/Invalid/tunnels",
            Some("secret"),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            invalid.into_body().collect().await.unwrap().to_bytes(),
            r#"{"error":"not found"}"#
        );

        let empty = request(app.clone(), "/admin/v1/users/bob/tunnels", Some("secret")).await;
        assert_eq!(empty.status(), StatusCode::OK);
        assert_eq!(empty.into_body().collect().await.unwrap().to_bytes(), "[]");

        let unknown_user = post(
            app.clone(),
            "/admin/v1/tunnels/bob/one/kill",
            Some("secret"),
        )
        .await;
        assert_eq!(unknown_user.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            unknown_user.into_body().collect().await.unwrap().to_bytes(),
            r#"{"error":"not found"}"#
        );

        let unknown_devserver = post(
            app.clone(),
            "/admin/v1/tunnels/alice/two/kill",
            Some("secret"),
        )
        .await;
        assert_eq!(unknown_devserver.status(), StatusCode::NOT_FOUND);

        let invalid_kill = post(app, "/admin/v1/users/Invalid/tunnels/kill", Some("secret")).await;
        assert_eq!(invalid_kill.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            invalid_kill.into_body().collect().await.unwrap().to_bytes(),
            r#"{"error":"not found"}"#
        );
    }

    #[tokio::test(start_paused = true)]
    async fn exact_kill_returns_204_when_the_proxy_confirms() {
        let registration_id = Uuid::new_v4();
        let (controller, mut session) =
            ready_controller_with(vec![row("alice", "one", registration_id)]).await;
        let proxy_id = ProxyId::parse("p1").unwrap();
        let app = router(controller.clone(), "secret".into());

        let pending = tokio::spawn(post(
            app,
            "/admin/v1/tunnels/alice/one/kill",
            Some("secret"),
        ));
        let (command_id, registration_ids) = recv_kill_command(&mut session).await;
        assert_eq!(registration_ids, vec![registration_id]);
        controller
            .command_result(
                proxy_id,
                session.incarnation,
                command_id,
                registration_ids,
                Vec::new(),
                Vec::new(),
            )
            .await
            .unwrap();
        let response = pending.await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(controller.tunnels().await.unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn user_kill_counts_killed_and_missing_and_retries_empty() {
        let killed = Uuid::new_v4();
        let missing = Uuid::new_v4();
        let bob_registration = Uuid::new_v4();
        let (controller, mut session) = ready_controller_with(vec![
            row("alice", "one", killed),
            row("alice", "two", missing),
            row("bob", "one", bob_registration),
        ])
        .await;
        let proxy_id = ProxyId::parse("p1").unwrap();
        let app = router(controller.clone(), "secret".into());

        let pending = tokio::spawn(post(
            app.clone(),
            "/admin/v1/users/alice/tunnels/kill",
            Some("secret"),
        ));
        let (command_id, registration_ids) = recv_kill_command(&mut session).await;
        assert_eq!(registration_ids.len(), 2);
        assert!(!registration_ids.contains(&bob_registration));
        controller
            .command_result(
                proxy_id,
                session.incarnation,
                command_id,
                vec![killed],
                vec![missing],
                Vec::new(),
            )
            .await
            .unwrap();
        let response = pending.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            r#"{"killed":2}"#
        );

        // A retry has nothing left to kill and is a success, not an error.
        let retry = post(app, "/admin/v1/users/alice/tunnels/kill", Some("secret")).await;
        assert_eq!(retry.status(), StatusCode::OK);
        assert_eq!(
            retry.into_body().collect().await.unwrap().to_bytes(),
            r#"{"killed":0}"#
        );
    }

    #[tokio::test(start_paused = true)]
    async fn user_kill_reports_partial_when_a_proxy_disconnects() {
        let controller = crate::spawn_controller(100);
        let (p1, s1) =
            begin_proxy(&controller, "p1", vec![row("alice", "one", Uuid::new_v4())]).await;
        let (p2, s2) =
            begin_proxy(&controller, "p2", vec![row("alice", "two", Uuid::new_v4())]).await;
        let mut sessions = vec![(p1.clone(), s1), (p2.clone(), s2)];
        keep_alive_until_ready(&controller, &mut sessions).await;
        assert!(controller.is_ready().await.unwrap());
        let mut sessions: std::collections::HashMap<_, _> = sessions.into_iter().collect();
        let app = router(controller.clone(), "secret".into());

        let pending = tokio::spawn(post(
            app,
            "/admin/v1/users/alice/tunnels/kill",
            Some("secret"),
        ));
        let s1 = sessions.get_mut(&p1).unwrap();
        let (command_id, registration_ids) = recv_kill_command(s1).await;
        controller
            .command_result(
                p1.clone(),
                s1.incarnation,
                command_id,
                registration_ids,
                Vec::new(),
                Vec::new(),
            )
            .await
            .unwrap();
        let p2_incarnation = sessions.get(&p2).unwrap().incarnation;
        controller.disconnect(p2, p2_incarnation).await.unwrap();

        let response = pending.await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            r#"{"error":"partial kill","killed":1}"#
        );
    }
}
