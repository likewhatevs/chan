//! End-to-end test for the embedded tunnel server.
//!
//! Wires the full stack in-process:
//!
//! 1. `serve_tunnel_listener` with chan-desktop's `LocalValidator`
//!    on `127.0.0.1:0`.
//! 2. `chan_tunnel_client::dial` + `serve_substreams` registering a
//!    workspace under the bearer-token-as-username, serving a tiny axum
//!    upstream that echoes the path it received.
//! 3. `spawn_tenant_listener` from chan-desktop's tunnel module,
//!    producing a per-tenant loopback listener.
//! 4. A plain `reqwest` GET against the per-tenant listener at
//!    `/{workspace}/ping`. Asserts the upstream saw `/ping` (the
//!    `/{label}/{workspace}` prefix was rewritten in and then stripped).
//!
//! Guardrail 5 from the design handoff: pin the URL shape so a
//! future axum bump cannot silently break the rewrite.

use std::sync::Arc;
use std::time::Duration;

use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use chan_tunnel_client::{dial, ClientConfig};
use chan_tunnel_server::{serve_tunnel_listener, Registry, Validator};
use tokio::net::TcpListener;
use url::Url;

// Pull the desktop crate's tunnel module in. The binary crate's
// modules aren't accessible from an integration test target, so
// we re-include the source files via the `path = ...` attribute.
// Mirrors how chan-core's own integration tests reach into the
// crate.
#[path = "../src/tunnel/public.rs"]
mod desktop_public;
#[path = "../src/tunnel/validator.rs"]
mod desktop_validator;

#[tokio::test]
async fn rewritten_path_reaches_registry_looked_up_tunnel() {
    let label = "alex-laptop";
    let workspace_name = "notes";

    // 1. Tunnel listener with chan-desktop's actual validator.
    let registry = Registry::new();
    let tunnel_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();
    let validator: Arc<dyn Validator> = Arc::new(desktop_validator::LocalValidator);
    {
        let registry = registry.clone();
        tokio::spawn(async move {
            let _ = serve_tunnel_listener(tunnel_listener, validator, registry, 0).await;
        });
    }

    // 2. Remote-side client: dial in, then serve a tiny upstream
    //    router that records the path it saw and echoes it back.
    let cfg = ClientConfig {
        tunnel_url: Url::parse(&format!("http://127.0.0.1:{tunnel_port}/v1/tunnel"))
            .expect("hard-coded url"),
        token: label.to_string(),
        workspace: workspace_name.to_string(),
        client_version: "chan-desktop/test".into(),
        public: false,
        initial_backoff: Duration::from_millis(50),
        max_backoff: Duration::from_secs(1),
        dial_timeout: Duration::from_secs(5),
        events: None,
        proxy: None,
        max_concurrent_substreams: ClientConfig::default().max_concurrent_substreams,
    };
    let (_reg, yconn) = dial(&cfg).await.expect("dial");

    let upstream: Router = Router::new().route(
        "/ping",
        any(|req: axum::http::Request<axum::body::Body>| async move {
            let path = req.uri().path().to_string();
            (axum::http::StatusCode::OK, format!("upstream-saw:{path}")).into_response()
        }),
    );
    tokio::spawn(async move {
        let _ = chan_tunnel_client::serve_substreams(yconn, upstream).await;
    });

    // Wait for registration.
    let mut registered = false;
    for _ in 0..100 {
        if registry.get(label, workspace_name).is_some() {
            registered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(registered, "tunnel did not register");

    // 3. Per-tenant listener with the path-prepend layer.
    let (port, _cancel) =
        desktop_public::spawn_tenant_listener(label.to_string(), registry.clone())
            .await
            .expect("spawn_tenant_listener");

    // Give axum::serve a beat to start accepting and the
    // serve_substreams future a beat to start polling inbound
    // substreams. Without this the very first request can race
    // either of them.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 4. GET /{workspace}/ping. Visit URL has no `label` segment.
    let url = format!("http://127.0.0.1:{port}/{workspace_name}/ping");
    let resp = reqwest::Client::builder()
        .build()
        .unwrap()
        .get(&url)
        .send()
        .await
        .expect("public GET");
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "url={url}, body={body}");
    // The upstream chan-serve substitute saw `/ping` — i.e. the
    // layer prepended `/{label}` so the inner router matched
    // `/:user/:workspace/*rest` with (`label`, `workspace`, `ping`), then
    // stripped both `/:user/:workspace` segments before forwarding.
    // Pin the literal string: any future change in axum / upstream
    // proxy that alters the stripped path will trip here.
    assert_eq!(body, "upstream-saw:/ping");
}
