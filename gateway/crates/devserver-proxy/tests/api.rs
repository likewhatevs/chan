//! Integration tests for devserver-proxy.
//!
//! No Postgres in this suite: devserver-proxy holds no sessions and
//! no DB state. The proxy gate is driven by devserver-gate JWTs
//! (HS256, shared `DEVSERVER_GATE_SECRET`), and tests mint those
//! directly via `gateway_common::devserver_gate`.
//!
//! Tunnel registrations exercise the real chan-tunnel handshake
//! (h2c POST, Hello/HelloAck, yamux) against an in-process tunnel
//! listener fed by a stub Validator.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, Request as AxRequest};
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use bytes::Bytes;
use chan_tunnel_proto::{H2Duplex, TUNNEL_PATH};
use chan_tunnel_server::{serve_tunnel_listener, ServerError, Validated, Validator};
use gateway_common::devserver_gate;
use http::Method as HttpMethod;
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tower::ServiceExt;
use uuid::Uuid;

use devserver_proxy::config::Config;
use devserver_proxy::http as dp_http;
use devserver_proxy::identity_validator::CapturingValidator;
use devserver_proxy::registry::Registry;

const ADMIN_TOKEN: &str = "test-admin-token";
const DEVSERVER_GATE_SECRET: &[u8] = b"test-devserver-gate-secret-32-bytes-aa";
const APEX_HOST: &str = "devserver.chan.app";
const WILDCARD_SUFFIX: &str = ".devserver.chan.app";

/// (user_id, username, devserver_id, scopes) row stored per-token in
/// the stub. Aliased so clippy's `type_complexity` lint is happy on the
/// inner `Arc<Mutex<HashMap<...>>>` declaration below.
type StubRow = (Uuid, String, String, Vec<String>);

/// Stub validator: tokens map to (user_id, username, devserver_id,
/// scopes). Used in place of the real IdentityValidator so tests don't
/// need identity-service. The tunnel-server keys the registration on the
/// token-resolved `devserver_id` (server-authoritative), so the stub is
/// what determines the registry's second key. Every token carries the
/// base `tunnel` scope.
#[derive(Clone, Default)]
struct StubValidator {
    by_token: Arc<StdMutex<HashMap<String, StubRow>>>,
}

impl StubValidator {
    fn add(
        &self,
        token: impl Into<String>,
        user_id: Uuid,
        username: impl Into<String>,
        devserver_id: impl Into<String>,
    ) {
        self.by_token.lock().unwrap().insert(
            token.into(),
            (
                user_id,
                username.into(),
                devserver_id.into(),
                vec!["tunnel".to_string()],
            ),
        );
    }
}

#[async_trait]
impl Validator for StubValidator {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        let g = self.by_token.lock().unwrap();
        match g.get(token) {
            Some((uid, username, devserver_id, scopes)) => Ok(Validated {
                user_id: *uid,
                username: username.clone(),
                devserver_id: devserver_id.clone(),
                scopes: scopes.clone(),
            }),
            None => Err(ServerError::InvalidToken),
        }
    }
}

struct TestApp {
    router: Router,
    registry: Registry,
    tunnel_addr: SocketAddr,
    stub: StubValidator,
}

impl TestApp {
    async fn new() -> Self {
        Self::new_with_max_workspaces(0).await
    }

    async fn new_with_max_workspaces(max_workspaces_per_user: usize) -> Self {
        let registry = Registry::new();

        let cfg = Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            tunnel_bind_addr: "127.0.0.1:0".parse().unwrap(),
            apex_host: APEX_HOST.into(),
            wildcard_suffix: WILDCARD_SUFFIX.into(),
            identity_url: "http://127.0.0.1:7000/".parse().unwrap(),
            identity_auth_token: "unused-in-tests".into(),
            dashboard_url: "https://id.chan.app/workspaces".into(),
            workspace_gate_secret: std::str::from_utf8(DEVSERVER_GATE_SECRET).unwrap().into(),
            max_workspaces_per_user,
            admin_token: Some(ADMIN_TOKEN.to_string()),
            max_response_bytes: None,
            max_request_bytes: None,
            request_timeout: None,
            forwarded_proto: "https".into(),
        });

        let router = dp_http::router(cfg, registry.clone());

        // Real tunnel listener fed by a stub validator wrapped in
        // CapturingValidator (mirrors production wiring).
        let stub = StubValidator::default();
        let validator: Arc<dyn Validator> =
            Arc::new(CapturingValidator::new(stub.clone(), registry.clone()));
        let tunnel_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tunnel_addr = tunnel_listener.local_addr().unwrap();
        {
            let tunnels = registry.tunnels();
            tokio::spawn(async move {
                let _ = serve_tunnel_listener(
                    tunnel_listener,
                    validator,
                    tunnels,
                    max_workspaces_per_user,
                )
                .await;
            });
        }

        Self {
            router,
            registry,
            tunnel_addr,
            stub,
        }
    }

    async fn cleanup(self) {
        // Nothing DB-backed; just drop self.
    }

    async fn register_tunnel(&self, username: &str, devserver_id: &str, uid: Uuid, router: Router) {
        let token = format!("tok-{}", Uuid::new_v4().simple());
        // The tunnel-server keys the registration on the token-resolved
        // devserver_id, so the stub returns it; the registry's second key
        // is this value (Hello.workspace is not the identity source).
        self.stub.add(&token, uid, username, devserver_id);
        spawn_tunnel_client(self.tunnel_addr, &token, devserver_id, router).await;
        for _ in 0..50 {
            if self.registry.get(username, devserver_id).is_some() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("tunnel for {username}/{devserver_id} did not register");
    }
}

async fn spawn_tunnel_client(
    tunnel_addr: SocketAddr,
    token: &str,
    workspace: &str,
    router: Router,
) {
    let token = token.to_string();
    let workspace = workspace.to_string();
    tokio::spawn(async move {
        if let Err(e) = run_tunnel_client(tunnel_addr, &token, &workspace, router).await {
            tracing::warn!(error = ?e, "test tunnel client ended");
        }
    });
}

async fn run_tunnel_client(
    tunnel_addr: SocketAddr,
    token: &str,
    workspace: &str,
    router: Router,
) -> anyhow::Result<()> {
    let tcp = TcpStream::connect(tunnel_addr).await?;
    tcp.set_nodelay(true)?;
    let (mut h2, conn) = h2::client::handshake(tcp).await?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = http::Request::builder()
        .method(HttpMethod::POST)
        .uri(format!("https://chan-tunnel{TUNNEL_PATH}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(())
        .unwrap();
    let (response_fut, send_stream) = h2.send_request(req, false)?;
    let response = response_fut.await?;
    if response.status() != http::StatusCode::OK {
        return Err(anyhow::anyhow!("tunnel POST status {}", response.status()));
    }
    let recv_stream = response.into_body();

    let duplex = H2Duplex::new(send_stream, recv_stream);

    let cfg = chan_tunnel_client::ClientConfig {
        tunnel_url: "https://chan-tunnel/v1/tunnel".parse().unwrap(),
        token: token.to_string(),
        workspace: workspace.to_string(),
        ..Default::default()
    };
    let (_registration, yconn) = chan_tunnel_client::handshake(&cfg, duplex).await?;
    chan_tunnel_client::serve_substreams(yconn, router).await?;
    Ok(())
}

/// Send a request with a Host header. Workspace-proxy routes off Host so
/// every wildcard test must supply one; oneshot does not synthesize.
async fn send_host(
    router: &Router,
    method: Method,
    host: &str,
    uri: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, HeaderMap, Bytes) {
    let mut builder = Request::builder().method(method).uri(uri);
    builder = builder.header(header::HOST, host);
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    let req = builder.body(Body::empty()).unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let hdrs = res.headers().clone();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    (status, hdrs, bytes)
}

async fn send_admin(
    router: &Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    builder = builder.header(header::HOST, APEX_HOST);
    if let Some(b) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {b}"));
    }
    let mut req = builder.body(Body::empty()).unwrap();
    // ConnectInfo is what `forwarded_headers` reads for the peer-IP
    // tail on X-Forwarded-For. `oneshot` bypasses the axum service
    // that normally populates it, so we seed a dummy value here.
    req.extensions_mut()
        .insert(ConnectInfo::<std::net::SocketAddr>(
            "127.0.0.1:1".parse().unwrap(),
        ));
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

/// Mint a devserver-gate token of the requested shape. Tests use this to
/// build URLs and cookies the proxy gate will accept (or to forge
/// near-misses for the negative cases).
fn mint(typ: devserver_gate::TokenType, sub: Uuid, drv: &str, aud: &str) -> String {
    match typ {
        devserver_gate::TokenType::Entry => {
            devserver_gate::encode_entry(DEVSERVER_GATE_SECRET, sub, drv, aud).unwrap()
        }
        devserver_gate::TokenType::Session => {
            devserver_gate::encode_session(DEVSERVER_GATE_SECRET, sub, drv, aud).unwrap()
        }
    }
}

fn host_for(user: &str) -> String {
    format!("{user}{WILDCARD_SUFFIX}")
}

/// A `Cookie` header value carrying a valid session token for
/// `(sub, workspace)` on `host`. Every reverse-proxy request must pass
/// the gate now that there is no un-gated public path.
fn session_cookie(sub: Uuid, workspace: &str, host: &str) -> String {
    let session = mint(devserver_gate::TokenType::Session, sub, workspace, host);
    format!("devserver_gate={session}")
}

// ---------------------------------------------------------------
// Apex routing
// ---------------------------------------------------------------

#[tokio::test]
async fn apex_healthz_ok() {
    let app = TestApp::new().await;
    let (s, _, _) = send_host(&app.router, Method::GET, APEX_HOST, "/healthz", &[]).await;
    assert_eq!(s, StatusCode::OK);
    app.cleanup().await;
}

#[tokio::test]
async fn apex_unknown_path_is_404() {
    let app = TestApp::new().await;
    let (s, _, _) = send_host(&app.router, Method::GET, APEX_HOST, "/", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _, _) = send_host(&app.router, Method::GET, APEX_HOST, "/api/me", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _, _) = send_host(&app.router, Method::GET, APEX_HOST, "/alice", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn unknown_host_is_404() {
    let app = TestApp::new().await;
    let (s, _, _) = send_host(&app.router, Method::GET, "evil.example.com", "/blog/", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Wildcard root -> dashboard
// ---------------------------------------------------------------

#[tokio::test]
async fn wildcard_root_redirects_to_dashboard() {
    let app = TestApp::new().await;
    let (s, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        "alice.devserver.chan.app",
        "/",
        &[],
    )
    .await;
    assert!(s.is_redirection(), "got {s}");
    let loc = hdrs.get(header::LOCATION).unwrap().to_str().unwrap();
    assert_eq!(loc, "https://id.chan.app/workspaces");
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Proxy gate (unregistered + anonymous)
// ---------------------------------------------------------------

#[tokio::test]
async fn unregistered_workspace_is_404() {
    let app = TestApp::new().await;
    let (s, _, body) = send_host(&app.router, Method::GET, &host_for("alice"), "/blog/", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "not found");
    app.cleanup().await;
}

#[tokio::test]
async fn unregistered_workspace_html_browser_gets_dead_end_page() {
    let app = TestApp::new().await;
    let (s, hdrs, body) = send_host(
        &app.router,
        Method::GET,
        &host_for("alice"),
        "/blog/",
        &[("accept", "text/html,application/xhtml+xml")],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let ct = hdrs.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
    assert!(ct.starts_with("text/html"));
    assert!(std::str::from_utf8(&body)
        .unwrap()
        .contains("workspace unavailable"));
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Proxy gate (private + devserver_gate JWT)
// ---------------------------------------------------------------

#[tokio::test]
async fn private_workspace_anonymous_is_404() {
    // Indistinguishable from unregistered: no leak.
    let app = TestApp::new().await;
    app.register_tunnel("alice", "blog", Uuid::new_v4(), Router::new())
        .await;

    let (s, _, _) = send_host(&app.router, Method::GET, &host_for("alice"), "/blog/", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_token_mints_session_cookie() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let upstream = Router::new().route("/", axum::routing::get(|| async { "owner ok" }));
    app.register_tunnel("alice", "blog", uid, upstream).await;

    let host = host_for("alice");
    let token = mint(devserver_gate::TokenType::Entry, uid, "blog", &host);
    let (s, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/?t={token}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    let loc = hdrs.get(header::LOCATION).unwrap().to_str().unwrap();
    assert_eq!(loc, "/blog/");
    let set = hdrs.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(set.starts_with("devserver_gate="), "got {set}");
    // Whole-host cookie: the grant is the whole devserver, so the cookie
    // is no longer scoped to a per-workspace path.
    assert!(
        set.contains("Path=/;") || set.contains("Path=/ "),
        "got {set}"
    );
    assert!(set.contains("HttpOnly"));
    assert!(set.contains("Secure"));
    assert!(set.contains("SameSite=Lax"));
    app.cleanup().await;
}

#[tokio::test]
async fn entry_token_drops_t_param_but_keeps_other_query() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, Router::new())
        .await;
    let host = host_for("alice");
    let token = mint(devserver_gate::TokenType::Entry, uid, "blog", &host);
    let (_, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/page?a=1&t={token}&b=2"),
        &[],
    )
    .await;
    let loc = hdrs.get(header::LOCATION).unwrap().to_str().unwrap();
    assert_eq!(loc, "/blog/page?a=1&b=2");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_token_for_wrong_devserver_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    // alice's live devserver id is "blog"; an entry token minted for a
    // different devserver id (e.g. a rotated/old one) must not admit.
    app.register_tunnel("alice", "blog", uid, Router::new())
        .await;
    let host = host_for("alice");
    let token = mint(
        devserver_gate::TokenType::Entry,
        uid,
        "stale-devserver",
        &host,
    );
    let (s, _, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/?t={token}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_token_for_wrong_host_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, Router::new())
        .await;
    // Token minted with aud=bob.devserver.chan.app, presented on
    // alice.devserver.chan.app.
    let bad_token = mint(
        devserver_gate::TokenType::Entry,
        uid,
        "blog",
        "bob.devserver.chan.app",
    );
    let (s, _, _) = send_host(
        &app.router,
        Method::GET,
        &host_for("alice"),
        &format!("/blog/?t={bad_token}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn session_cookie_admits() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let upstream = Router::new().fallback(|| async { "owner pass" });
    app.register_tunnel("alice", "blog", uid, upstream).await;

    let host = host_for("alice");
    let session = mint(devserver_gate::TokenType::Session, uid, "blog", &host);

    let proxy_addr = serve_router_real(app.router.clone()).await;
    let res = reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/"))
        .header(header::HOST, &host)
        .header(header::COOKIE, format!("devserver_gate={session}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "owner pass");
    app.cleanup().await;
}

#[tokio::test]
async fn session_cookie_for_wrong_devserver_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    // Live devserver id is "blog"; a session cookie carrying a different
    // devserver id (drv) must not admit, even on the right host.
    app.register_tunnel("alice", "blog", uid, Router::new())
        .await;
    let host = host_for("alice");
    let session = mint(
        devserver_gate::TokenType::Session,
        uid,
        "stale-devserver",
        &host,
    );
    let (s, _, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        "/blog/",
        &[("cookie", &format!("devserver_gate={session}"))],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// Regression: identity mints entry JWTs with `sub = caller.user_id`
// (owner or accepted grantee). devserver-proxy used to compare `sub`
// against the registry-cached owner_id and 404 every grantee; the gate
// now trusts identity's mint-time `devserver_access` check and admits any
// signed entry with the right aud + drv.
#[tokio::test]
async fn entry_token_for_grantee_mints_session_carrying_grantee_sub() {
    let app = TestApp::new().await;
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    let upstream = Router::new().route("/", axum::routing::get(|| async { "grantee ok" }));
    app.register_tunnel("alice", "blog", alice, upstream).await;

    let host = host_for("alice");
    // Bob is an accepted grantee; identity mints sub = bob.
    let entry = mint(devserver_gate::TokenType::Entry, bob, "blog", &host);
    let (s, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/?t={entry}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    let set = hdrs.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(set.starts_with("devserver_gate="), "got {set}");

    // The minted session cookie must carry sub = bob (the grantee),
    // not sub = alice (the owner), so upstream attribution is correct.
    let cookie = set
        .strip_prefix("devserver_gate=")
        .and_then(|s| s.split(';').next())
        .unwrap();
    let aud = host_for("alice");
    let claims = devserver_gate::decode(
        DEVSERVER_GATE_SECRET,
        cookie,
        devserver_gate::TokenType::Session,
        &aud,
        "blog",
    )
    .expect("session cookie should decode");
    assert_eq!(
        claims.sub, bob,
        "session cookie sub must be grantee, not owner"
    );
    app.cleanup().await;
}

// Regression: a session cookie with a non-owner sub admits as long as
// the signature + aud + drv match. Belongs alongside the
// `session_cookie_for_wrong_devserver_is_404` test which still validates
// the real bound (drv must match the live devserver id).
#[tokio::test]
async fn session_cookie_with_grantee_sub_admits() {
    let app = TestApp::new().await;
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    let upstream = Router::new().fallback(|| async { "grantee pass" });
    app.register_tunnel("alice", "blog", alice, upstream).await;
    let host = host_for("alice");
    let session = mint(devserver_gate::TokenType::Session, bob, "blog", &host);

    let proxy_addr = serve_router_real(app.router.clone()).await;
    let res = reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/"))
        .header(header::HOST, &host)
        .header(header::COOKIE, format!("devserver_gate={session}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "grantee pass");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_token_with_bad_signature_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, Router::new())
        .await;
    let host = host_for("alice");
    // Token minted with a different secret; same claim envelope.
    let bad =
        devserver_gate::encode_entry(b"some-other-secret-32-bytes-foobaa", uid, "blog", &host)
            .unwrap();
    let (s, _, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/?t={bad}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Segment-preserving forward + upstream proxy
// ---------------------------------------------------------------

#[tokio::test]
async fn proxy_preserves_workspace_segment() {
    // The proxy is a segment-PRESERVING forwarder: it hands the
    // devserver the full public `/{workspace}/...` path (the devserver
    // mounts each tenant at its public slug and routes internally).
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let upstream = Router::new()
        .route("/blog/assets/foo.js", axum::routing::get(|| async { "js" }))
        .route("/blog/", axum::routing::get(|| async { "root" }));
    app.register_tunnel("alice", "blog", uid, upstream).await;

    let host = host_for("alice");
    let cookie = session_cookie(uid, "blog", &host);
    let proxy_addr = serve_router_real(app.router.clone()).await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://{proxy_addr}/blog/assets/foo.js"))
        .header(header::HOST, &host)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "js");

    let res = client
        .get(format!("http://{proxy_addr}/blog/"))
        .header(header::HOST, &host)
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(res.text().await.unwrap(), "root");

    app.cleanup().await;
}

#[tokio::test]
async fn management_api_is_404_on_public_wildcard() {
    // `/api/devserver/*` is the devserver's local-only management API;
    // the proxy must 404 it on the public host so only tenant content
    // reaches the tunnel.
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let upstream = Router::new().fallback(|| async { "should not reach upstream" });
    app.register_tunnel("alice", "blog", uid, upstream).await;

    let host = host_for("alice");
    let cookie = session_cookie(uid, "blog", &host);
    // Even with a valid session cookie, the management API is not proxied.
    let (s, _, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        "/api/devserver/workspaces",
        &[("cookie", &cookie)],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Hop-by-hop + X-Forwarded-*
// ---------------------------------------------------------------

async fn serve_router_real(router: Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}

#[derive(Clone, Default)]
struct Captured {
    requests: Arc<StdMutex<Vec<RecordedRequest>>>,
}

struct RecordedRequest {
    headers: HeaderMap,
}

fn capturing_router(captured: Captured) -> Router {
    let captured = Arc::new(captured);
    Router::new().fallback(move |req: AxRequest| {
        let captured = captured.clone();
        async move {
            captured.requests.lock().unwrap().push(RecordedRequest {
                headers: req.headers().clone(),
            });
            (
                [(header::CONTENT_TYPE, "text/plain")],
                Bytes::from_static(b"ok"),
            )
                .into_response()
        }
    })
}

#[tokio::test]
async fn x_forwarded_for_appended_when_absent() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/x"))
        .header(header::HOST, &host)
        .header(header::COOKIE, session_cookie(uid, "blog", &host))
        .send()
        .await
        .unwrap();

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    let xff = headers.get("x-forwarded-for").unwrap().to_str().unwrap();
    assert_eq!(xff, "127.0.0.1");
    // X-Forwarded-Proto is sourced from cfg.forwarded_proto (which
    // TestApp configures as the prod default "https"), NOT from any
    // inbound X-Forwarded-Proto header. The test exercises the
    // no-inbound case here.
    let proto = headers.get("x-forwarded-proto").unwrap().to_str().unwrap();
    assert_eq!(proto, "https");
    // X-Forwarded-Host is sourced from the inbound Host header workspace-
    // proxy itself routed on, not from any inbound X-Forwarded-Host.
    let host = headers.get("x-forwarded-host").unwrap().to_str().unwrap();
    assert_eq!(host, host_for("alice"));
    app.cleanup().await;
}

#[tokio::test]
async fn x_forwarded_for_extended() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/y"))
        .header(header::HOST, &host)
        .header(header::COOKIE, session_cookie(uid, "blog", &host))
        .header("x-forwarded-for", "203.0.113.5")
        // Inbound XFProto/XFHost: client-supplied and must be ignored.
        // Asserted below: outbound matches cfg / Host, not these values.
        .header("x-forwarded-proto", "http")
        .header("x-forwarded-host", "evil.example.com")
        .send()
        .await
        .unwrap();

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    // XFF chain trust: existing chain preserved, peer IP appended.
    // This is intentional; nginx is expected to normalize untrusted
    // ingress XFF.
    let xff = headers.get("x-forwarded-for").unwrap().to_str().unwrap();
    assert_eq!(xff, "203.0.113.5, 127.0.0.1");
    // XFProto and XFHost are NOT trusted from inbound; the outbound
    // values come from cfg.forwarded_proto and the inbound Host
    // header. Without this we'd be a malleable forwarder for any
    // upstream that builds absolute URLs from XFH/XFProto.
    let proto = headers.get("x-forwarded-proto").unwrap().to_str().unwrap();
    assert_eq!(proto, "https");
    let host = headers.get("x-forwarded-host").unwrap().to_str().unwrap();
    assert_eq!(host, host_for("alice"));

    let xff_count = headers.get_all("x-forwarded-for").iter().count();
    assert_eq!(xff_count, 1);
    app.cleanup().await;
}

#[tokio::test]
async fn cookie_header_stripped_from_upstream() {
    // devserver-proxy must never forward the devserver_gate cookie to the
    // tenant's chan-serve peer (the cookie is for the gate, not for
    // the tenant). The very cookie that admits the request is stripped
    // before the upstream sees it. Other inbound cookies the tenant
    // content might care about are also stripped today; if that proves
    // wrong we can selectively preserve specific cookie names later.
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/z"))
        .header(header::HOST, &host)
        .header(
            header::COOKIE,
            format!("{}; other=value", session_cookie(uid, "blog", &host)),
        )
        .send()
        .await
        .unwrap();

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    assert!(headers.get(header::COOKIE).is_none());
    app.cleanup().await;
}

#[tokio::test]
async fn authorization_header_stripped_from_upstream() {
    // A user-presented Authorization bearer (e.g. an API client that
    // happens to land on a tenant URL with its own credential) must
    // never reach the tenant's chan-serve. Auth on this leg is the
    // devserver-gate cookie / entry-token handshake; the tenant's content
    // has no business seeing the user's PAT or any other bearer.
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/a"))
        .header(header::HOST, &host)
        .header(header::COOKIE, session_cookie(uid, "blog", &host))
        .header(header::AUTHORIZATION, "Bearer chan_pat_secret")
        .send()
        .await
        .unwrap();

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    assert!(
        headers.get(header::AUTHORIZATION).is_none(),
        "Authorization header must be stripped before reaching upstream"
    );
    app.cleanup().await;
}

// ---------------------------------------------------------------
// WebSocket bridge
// ---------------------------------------------------------------

#[tokio::test]
async fn websocket_bridges_text_frames() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as TgMessage;

    let app = TestApp::new().await;
    let uid = Uuid::new_v4();

    async fn echo(ws: axum::extract::WebSocketUpgrade) -> axum::response::Response {
        ws.on_upgrade(|mut socket| async move {
            if let Some(Ok(axum::extract::ws::Message::Text(s))) = socket.recv().await {
                let _ = socket
                    .send(axum::extract::ws::Message::Text(format!("echo:{s}").into()))
                    .await;
            }
            let _ = socket.close().await;
        })
    }
    // Segment-preserving forward: the upstream sees the full /blog/ws path.
    let upstream = Router::new().route("/blog/ws", axum::routing::get(echo));
    app.register_tunnel("alice", "blog", uid, upstream).await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    let url = format!("ws://{proxy_addr}/blog/ws");
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut req = url.into_client_request().unwrap();
    req.headers_mut()
        .insert(header::HOST, HeaderValue::from_str(&host).unwrap());
    req.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&session_cookie(uid, "blog", &host)).unwrap(),
    );

    let (mut client_ws, _resp) = tokio_tungstenite::connect_async(req).await.unwrap();
    client_ws
        .send(TgMessage::Text("hello".into()))
        .await
        .unwrap();
    let echoed = client_ws.next().await.expect("frame").expect("ws ok");
    match echoed {
        TgMessage::Text(s) => assert_eq!(s, "echo:hello"),
        other => panic!("unexpected: {other:?}"),
    }
    let _ = client_ws.close(None).await;
    app.cleanup().await;
}

#[tokio::test]
async fn websocket_upgrade_runs_auth_gate() {
    let app = TestApp::new().await;
    app.register_tunnel("alice", "blog", Uuid::new_v4(), Router::new())
        .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/blog/ws")
        .header(header::HOST, host_for("alice"))
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Body::empty())
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Tunnel listener (max_workspaces)
// ---------------------------------------------------------------

async fn try_register_tunnel(
    tunnel_addr: SocketAddr,
    token: &str,
    workspace: &str,
    router: Router,
) -> anyhow::Result<()> {
    let token = token.to_string();
    let workspace = workspace.to_string();
    let task =
        tokio::spawn(
            async move { run_tunnel_client(tunnel_addr, &token, &workspace, router).await },
        );
    match tokio::time::timeout(std::time::Duration::from_millis(300), task).await {
        Err(_) => Ok(()),
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(e)) => Err(anyhow::anyhow!("tunnel client task: {e}")),
    }
}

#[tokio::test]
async fn tunnel_rejects_third_workspace_when_limit_is_two() {
    let app = TestApp::new_with_max_workspaces(2).await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "a", uid, Router::new()).await;
    app.register_tunnel("alice", "b", uid, Router::new()).await;

    let token = format!("tok-{}", Uuid::new_v4().simple());
    app.stub.add(&token, uid, "alice", "c");
    let result = try_register_tunnel(app.tunnel_addr, &token, "c", Router::new()).await;
    assert!(
        result.is_err(),
        "third workspace should be rejected: {result:?}"
    );

    let workspaces: Vec<String> = app
        .registry
        .list_for("alice")
        .into_iter()
        .map(|d| d.workspace)
        .collect();
    assert_eq!(workspaces, vec!["a".to_string(), "b".to_string()]);
    app.cleanup().await;
}

#[tokio::test]
async fn tunnel_allows_reconnect_of_existing_workspace_at_limit() {
    let app = TestApp::new_with_max_workspaces(2).await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "a", uid, Router::new()).await;
    app.register_tunnel("alice", "b", uid, Router::new()).await;
    let token = format!("tok-{}", Uuid::new_v4().simple());
    app.stub.add(&token, uid, "alice", "a");
    try_register_tunnel(app.tunnel_addr, &token, "a", Router::new())
        .await
        .expect("reconnect at limit ok");
    app.cleanup().await;
}

#[tokio::test]
async fn tunnel_unlimited_when_max_is_zero() {
    let app = TestApp::new_with_max_workspaces(0).await;
    let uid = Uuid::new_v4();
    for d in ["a", "b", "c", "d"] {
        app.register_tunnel("alice", d, uid, Router::new()).await;
    }
    assert_eq!(app.registry.list_for("alice").len(), 4);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Admin tree
// ---------------------------------------------------------------

#[tokio::test]
async fn admin_requires_bearer() {
    let app = TestApp::new().await;
    let (s, _) = send_admin(&app.router, Method::GET, "/admin/v1/tunnels", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = send_admin(&app.router, Method::GET, "/admin/v1/tunnels", Some("nope")).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn admin_tunnels_list_and_kill() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();

    let (s, body) = send_admin(
        &app.router,
        Method::GET,
        "/admin/v1/tunnels",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    app.register_tunnel("alice", "home", uid, Router::new())
        .await;
    app.register_tunnel("alice", "open", uid, Router::new())
        .await;

    let (s, body) = send_admin(
        &app.router,
        Method::GET,
        "/admin/v1/tunnels",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Kill one.
    let (s, _) = send_admin(
        &app.router,
        Method::POST,
        "/admin/v1/tunnels/alice/home/kill",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (_, body) = send_admin(
        &app.router,
        Method::GET,
        "/admin/v1/tunnels",
        Some(ADMIN_TOKEN),
    )
    .await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["devserver_id"], "open");

    // Unknown -> 404.
    let (s, _) = send_admin(
        &app.router,
        Method::POST,
        "/admin/v1/tunnels/alice/nope/kill",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn admin_list_user_tunnels() {
    let app = TestApp::new().await;
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    app.register_tunnel("alice", "home", alice, Router::new())
        .await;
    app.register_tunnel("alice", "blog", alice, Router::new())
        .await;
    app.register_tunnel("bob", "home", bob, Router::new()).await;

    // alice has two; only alice's are returned.
    let (s, body) = send_admin(
        &app.router,
        Method::GET,
        "/admin/v1/users/alice/tunnels",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let devserver_ids: Vec<&str> = arr
        .iter()
        .map(|r| r["devserver_id"].as_str().unwrap())
        .collect();
    assert_eq!(devserver_ids, vec!["blog", "home"]);

    // Unknown user -> empty, not 404.
    let (s, body) = send_admin(
        &app.router,
        Method::GET,
        "/admin/v1/users/ghost/tunnels",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
    app.cleanup().await;
}

#[tokio::test]
async fn admin_kill_user_tunnels_evicts_all_for_user() {
    let app = TestApp::new().await;
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    app.register_tunnel("alice", "home", alice, Router::new())
        .await;
    app.register_tunnel("alice", "blog", alice, Router::new())
        .await;
    app.register_tunnel("bob", "home", bob, Router::new()).await;

    let (s, body) = send_admin(
        &app.router,
        Method::POST,
        "/admin/v1/users/alice/tunnels/kill",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["killed"], 2);

    let workspaces: Vec<_> = app
        .registry
        .list_all_tunnels()
        .into_iter()
        .map(|t| {
            (
                t.user.as_ref().to_string(),
                t.workspace.as_ref().to_string(),
            )
        })
        .collect();
    assert_eq!(workspaces, vec![("bob".to_string(), "home".to_string())]);

    // Idempotent.
    let (_, body) = send_admin(
        &app.router,
        Method::POST,
        "/admin/v1/users/alice/tunnels/kill",
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(body["killed"], 0);
    app.cleanup().await;
}
