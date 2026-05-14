//! Route-scoped middleware enforcing the server-side half of the
//! tunnel-mode lockdown.
//!
//!   - `settings_guard` returns 403 when `AppState::settings_disabled`
//!     is true (any tunnel run, hosted or public). Applied to the
//!     settings-write routes (drive rename, prefs / server-config
//!     PATCH, LLM key set / clear, storage reset, index rebuild).
//!   - `tunnel_public_guard` returns 403 when `AppState::tunnel_public`
//!     is true (only `--tunnel-public`). Applied to cost-bearing
//!     routes that an anonymous visitor must not be able to drive
//!     against the owner's machine (today: `POST /api/llm/complete`).
//!
//! Both run as middleware on a sub-router rather than as per-handler
//! guards: that way the refusal lands before axum's `Json<...>` /
//! `Query<...>` extractors, so a malformed body cannot leak the
//! request schema via a 422, and any future write route added to
//! the gated sub-router inherits the gate automatically.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::{err_settings_locked, err_tunnel_public_locked};
use crate::state::AppState;

pub async fn settings_guard(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if state.settings_disabled {
        return err_settings_locked();
    }
    next.run(req).await
}

pub async fn tunnel_public_guard(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if state.tunnel_public {
        return err_tunnel_public_locked();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    //! These tests exercise the middleware against a stub router
    //! without spinning up a real `AppState` (which would require
    //! a `chan-drive::Drive` on disk and a tokio runtime hot enough
    //! to run the watcher). The middleware only reads two bool
    //! fields on `AppState`, so a borrowed-bool variant would be a
    //! cleaner contract; for now we lean on the fact that the two
    //! reads happen up-front and are cheap.
    //!
    //! The handler under test panics if the middleware fails to
    //! short-circuit, so a `200 OK` reply proves the request
    //! actually reached the wrapped handler.
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use super::{settings_guard, tunnel_public_guard};
    use crate::state::test_support::make_test_state;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn build_router<F, Fut>(state: Arc<crate::state::AppState>, mw: F) -> Router
    where
        F: Clone
            + Send
            + Sync
            + 'static
            + Fn(
                axum::extract::State<Arc<crate::state::AppState>>,
                Request<Body>,
                middleware::Next,
            ) -> Fut,
        Fut: std::future::Future<Output = axum::response::Response> + Send + 'static,
    {
        Router::new()
            .route("/probe", get(ok_handler))
            .route_layer(middleware::from_fn_with_state(state.clone(), mw))
            .with_state(state)
    }

    async fn status_of(app: Router) -> StatusCode {
        let req = Request::builder()
            .uri("/probe")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.expect("router serve");
        res.status()
    }

    #[tokio::test]
    async fn settings_guard_passes_when_settings_enabled() {
        let state = make_test_state(false, false);
        let app = build_router(state, settings_guard);
        assert_eq!(status_of(app).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn settings_guard_blocks_when_settings_disabled() {
        let state = make_test_state(true, false);
        let app = build_router(state, settings_guard);
        assert_eq!(status_of(app).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn tunnel_public_guard_passes_on_hosted_tunnel() {
        // `settings_disabled = true, tunnel_public = false` proves
        // the two guards are independent: settings can be locked
        // without the public-tunnel restrictions kicking in.
        // (Real configs today bind these together via `public`, but
        // the middleware must not assume that coupling.)
        let state = make_test_state(true, false);
        let app = build_router(state, tunnel_public_guard);
        assert_eq!(status_of(app).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn tunnel_public_guard_blocks_on_public_tunnel() {
        let state = make_test_state(true, true);
        let app = build_router(state, tunnel_public_guard);
        assert_eq!(status_of(app).await, StatusCode::FORBIDDEN);
    }
}
