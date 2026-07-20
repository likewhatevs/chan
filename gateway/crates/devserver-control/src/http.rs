use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use serde::Serialize;
use subtle::ConstantTimeEq;
use tokio::sync::watch;

use crate::{ActorError, ControllerHandle, ProxyView, TunnelView};

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
    use devserver_control_proto::{CanonicalOrigin, ProxyId, ServerFrame};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn request(app: Router, path: &str, token: Option<&str>) -> Response {
        let mut request = axum::http::Request::builder().uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        app.oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn ready_controller() -> ControllerHandle {
        let controller = crate::spawn_controller(100);
        let proxy_id = ProxyId::parse("p1").unwrap();
        let mut session = controller
            .begin_session(
                proxy_id.clone(),
                CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
                env!("CARGO_PKG_VERSION").into(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        controller
            .accept_snapshot(proxy_id.clone(), session.incarnation, 0, Vec::new())
            .await
            .unwrap();
        for _ in 0..6 {
            tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
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
        assert!(controller.is_ready().await.unwrap());
        controller
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
}
