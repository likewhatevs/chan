//! End-to-end tests for the public router.
//!
//! Wires the full proxy path in-process: a tunnel listener with a
//! stub validator, a chan-tunnel-client registering a chan-serve
//! router on the other end, and `public_router` mounted on its own
//! TCP listener. Each test then drives a real `reqwest` client
//! against the public listener, so request and response shape
//! match what an internet visitor would see.
//!
//! Two features under test:
//!
//! 1. `PublicConfig::response_body_cap` truncates an oversized
//!    upstream response.
//! 2. Slow preview-style responses do not block a small edit-style
//!    request over the same tunnel.
//! 3. `PublicConfig::rate_limit_per_second` returns 429 above the
//!    configured burst on a per-IP basis.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chan_tunnel_client::{dial, ClientConfig};
use chan_tunnel_server::{
    public_router_with, serve_tunnel_listener, PublicConfig, Registry, ServerError, Validated,
    Validator, TUNNEL_SCOPE,
};
use futures::StreamExt;
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;

struct StubValidator {
    token: String,
    username: String,
    scopes: Vec<String>,
}

#[async_trait]
impl Validator for StubValidator {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        if token != self.token {
            return Err(ServerError::InvalidToken);
        }
        Ok(Validated {
            user_id: Uuid::nil(),
            username: self.username.clone(),
            scopes: self.scopes.clone(),
        })
    }
}

/// Test harness wiring: tunnel listener, public listener, and a
/// chan-tunnel-client serving the supplied router.
struct PublicHarness {
    public_addr: SocketAddr,
}

async fn spawn(cfg: PublicConfig, upstream: Router) -> PublicHarness {
    let token = "tok".to_string();
    let username = "alice".to_string();
    let drive = "notes".to_string();

    let validator: Arc<dyn Validator> = Arc::new(StubValidator {
        token: token.clone(),
        username: username.clone(),
        scopes: vec![TUNNEL_SCOPE.into()],
    });
    let registry = Registry::new();

    // Tunnel side: listener + accept loop.
    let tunnel_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tunnel_addr = tunnel_listener.local_addr().unwrap();
    {
        let registry = registry.clone();
        tokio::spawn(async move {
            let _ = serve_tunnel_listener(tunnel_listener, validator, registry, 0).await;
        });
    }

    // chan-tunnel-client dial against the listener, then serve the
    // supplied upstream router over the resulting yamux connection.
    let client_cfg = ClientConfig {
        tunnel_url: Url::parse(&format!(
            "http://127.0.0.1:{}/v1/tunnel",
            tunnel_addr.port()
        ))
        .unwrap(),
        token,
        drive: drive.clone(),
        client_version: "chan/test".into(),
        public: false,
        initial_backoff: Duration::from_millis(50),
        max_backoff: Duration::from_secs(1),
        dial_timeout: Duration::from_secs(5),
        events: None,
        proxy: None,
        max_concurrent_substreams: chan_tunnel_client::DEFAULT_MAX_CONCURRENT_SUBSTREAMS,
    };
    let (_reg, yconn) = dial(&client_cfg).await.expect("dial");
    tokio::spawn(async move {
        let _ = chan_tunnel_client::serve_substreams(yconn, upstream).await;
    });

    // Spin-wait for the registration before binding the public
    // listener so the first GET in the test isn't a 502.
    for _ in 0..100 {
        if registry.get(&username, &drive).is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        registry.get(&username, &drive).is_some(),
        "tunnel did not register"
    );

    // Public side: router + axum::serve on a fresh socket.
    let public_router = public_router_with(registry, cfg);
    let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let public_addr = public_listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(
            public_listener,
            public_router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });

    PublicHarness { public_addr }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        // Each request opens a fresh TCP connection so the
        // governor's per-IP bucket sees consistent peer addrs from
        // the loopback side.
        .pool_max_idle_per_host(0)
        .build()
        .unwrap()
}

#[tokio::test]
async fn response_body_cap_passes_under_cap_payloads_through() {
    // Sanity: upstream body within cap is delivered intact and
    // the cap layer doesn't molest small responses. Locks in that
    // the only path that strips Content-Length is the truncation
    // path; un-truncated responses keep their headers.
    const CAP: usize = 16 * 1024;
    const PAYLOAD: usize = 4 * 1024;

    let upstream = Router::new().route(
        "/small",
        get(|| async {
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(vec![b'a'; PAYLOAD]))
                .unwrap()
        }),
    );
    let cfg = PublicConfig {
        response_body_cap: CAP,
        ..PublicConfig::default()
    };
    let h = spawn(cfg, upstream).await;

    let url = format!("http://{}/alice/notes/small", h.public_addr);
    let resp = client().get(&url).send().await.expect("send");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.bytes().await.expect("body");
    assert_eq!(bytes.len(), PAYLOAD, "full body should pass through");
    assert!(bytes.iter().all(|&b| b == b'a'));
}

#[tokio::test]
async fn response_body_cap_aborts_oversized_streamed_payload() {
    // Upstream emits a chunked stream of 32 KiB; cap at 4 KiB.
    // Streaming so the upstream has no Content-Length header
    // (the response goes chunked end-to-end). `Limited` errors
    // mid-stream once the cap is hit; the body channel closes
    // before the upstream's terminal chunk, so reqwest surfaces
    // a body-side stream error - exactly the "did not deliver
    // the full payload" outcome we want when chan-serve tries
    // to burn egress.
    const CAP: usize = 4 * 1024;
    const PAYLOAD: usize = 32 * 1024;

    let upstream = Router::new().route(
        "/stream",
        get(|| async {
            // 1 KiB chunks emitted in a stream so no Content-
            // Length is set on the response.
            let chunks: Vec<Result<bytes::Bytes, std::io::Error>> = (0..(PAYLOAD / 1024))
                .map(|_| Ok(bytes::Bytes::from(vec![b'x'; 1024])))
                .collect();
            let stream = futures::stream::iter(chunks);
            Body::from_stream(stream)
        }),
    );
    let cfg = PublicConfig {
        response_body_cap: CAP,
        ..PublicConfig::default()
    };
    let h = spawn(cfg, upstream).await;

    let url = format!("http://{}/alice/notes/stream", h.public_addr);
    let resp = client().get(&url).send().await.expect("send");
    assert_eq!(resp.status(), StatusCode::OK);
    // Either we get a truncated bytes() (<= CAP) or a body
    // stream error; both are the policy in action.
    match resp.bytes().await {
        Ok(bytes) => {
            assert!(
                bytes.len() <= CAP,
                "expected <= {CAP} bytes through the cap, got {}",
                bytes.len(),
            );
            assert!(
                bytes.len() < PAYLOAD,
                "cap should have aborted before the full {PAYLOAD} bytes",
            );
        }
        Err(_) => {
            // OK: body stream errored after the cap. The truncated
            // bytes already left the gateway, but the upstream's
            // remaining KiBs did not.
        }
    }
}

#[tokio::test]
async fn small_request_completes_while_image_preview_stream_is_active() {
    let image_done = Arc::new(AtomicBool::new(false));
    let image_done_for_route = image_done.clone();
    let upstream = Router::new()
        .route(
            "/image",
            get(move || {
                let image_done = image_done_for_route.clone();
                async move {
                    let stream = futures::stream::unfold(0usize, move |idx| {
                        let image_done = image_done.clone();
                        async move {
                            if idx >= 64 {
                                image_done.store(true, Ordering::SeqCst);
                                return None;
                            }
                            tokio::time::sleep(Duration::from_millis(25)).await;
                            Some((
                                Ok::<_, std::io::Error>(bytes::Bytes::from(vec![b'i'; 1024])),
                                idx + 1,
                            ))
                        }
                    });
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "image/png")
                        .body(Body::from_stream(stream))
                        .unwrap()
                }
            }),
        )
        .route("/edit", get(|| async { "edit-ok" }));
    let h = spawn(PublicConfig::default(), upstream).await;
    let cli = client();

    let image_url = format!("http://{}/alice/notes/image", h.public_addr);
    let image_resp = cli.get(&image_url).send().await.expect("image send");
    assert_eq!(image_resp.status(), StatusCode::OK);
    let mut image_body = image_resp.bytes_stream();
    let first = tokio::time::timeout(Duration::from_secs(1), image_body.next())
        .await
        .expect("image stream produced first chunk")
        .expect("image stream ended early")
        .expect("image stream chunk");
    assert!(!first.is_empty());
    assert!(
        !image_done.load(Ordering::SeqCst),
        "image stream should still be active"
    );

    let edit_url = format!("http://{}/alice/notes/edit", h.public_addr);
    let edit_resp = tokio::time::timeout(Duration::from_secs(1), cli.get(&edit_url).send())
        .await
        .expect("small request should not wait behind active image stream")
        .expect("edit send");
    assert_eq!(edit_resp.status(), StatusCode::OK);
    let body = edit_resp.text().await.expect("edit body");
    assert_eq!(body, "edit-ok");

    drop(image_body);
}

#[tokio::test]
async fn rate_limit_returns_429_above_burst() {
    let upstream = Router::new().route("/", get(|| async { "ok" }));
    let cfg = PublicConfig {
        rate_limit_per_second: 1,
        rate_limit_burst: 1,
        ..PublicConfig::default()
    };
    let h = spawn(cfg, upstream).await;
    let url = format!("http://{}/alice/notes/", h.public_addr);
    let cli = client();

    let r1 = cli.get(&url).send().await.expect("r1");
    assert_eq!(r1.status(), StatusCode::OK, "first request should pass");

    // Within the same second, the per-IP bucket is empty; expect
    // 429. tower-governor uses a token bucket so back-to-back
    // requests deterministically hit the throttle.
    let r2 = cli.get(&url).send().await.expect("r2");
    assert_eq!(
        r2.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "second request should be throttled"
    );
}
