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
use axum::extract::Request as AxRequest;
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use bytes::Bytes;
use chan_tunnel_proto::{H2Duplex, TUNNEL_PATH};
use chan_tunnel_server::{
    serve_tunnel_listener_with_admission, AllowAllAdmission, ServerError, Validated, Validator,
};
use devserver_control_proto::{CanonicalOrigin, ProxyId};
use futures_util::{SinkExt, StreamExt};
use gateway_common::devserver_gate;
use http::Method as HttpMethod;
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tower::ServiceExt;
use uuid::Uuid;

use devserver_proxy::config::{Config, DEFAULT_WS_IDLE_TIMEOUT};
use devserver_proxy::http as dp_http;
use devserver_proxy::identity_validator::CapturingValidator;
use devserver_proxy::registry::Registry;

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
                gateway_assertion_key: Some(
                    chan_tunnel_proto::gateway_assertion::derive_assertion_key(token),
                ),
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
    _readiness: watch::Sender<bool>,
}

impl TestApp {
    async fn new() -> Self {
        Self::new_inner(DEFAULT_WS_IDLE_TIMEOUT).await
    }

    /// The WS-bridge tests inject a sub-second idle window so the cut
    /// is observable without waiting out the production default.
    async fn new_with_ws_idle_timeout(ws_idle_timeout: std::time::Duration) -> Self {
        Self::new_inner(ws_idle_timeout).await
    }

    async fn new_inner(ws_idle_timeout: std::time::Duration) -> Self {
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
            control_url: "http://127.0.0.1:7101/".parse().unwrap(),
            proxy_token: "unused-control-token".into(),
            proxy_id: ProxyId::parse("p1").unwrap(),
            proxy_base_url: CanonicalOrigin::parse("https://p1.devserver.chan.app").unwrap(),
            max_response_bytes: None,
            max_request_bytes: None,
            request_timeout: None,
            ws_idle_timeout,
            forwarded_proto: "https".into(),
        });

        let (readiness, readiness_rx) = watch::channel(true);
        let router = dp_http::router(cfg, registry.clone(), readiness_rx);

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
                let _ = serve_tunnel_listener_with_admission(
                    tunnel_listener,
                    validator,
                    Arc::new(AllowAllAdmission),
                    tunnels,
                    0,
                )
                .await;
            });
        }

        Self {
            router,
            registry,
            tunnel_addr,
            stub,
            _readiness: readiness,
        }
    }

    async fn cleanup(self) {
        // Nothing DB-backed; just drop self.
    }

    async fn register_tunnel(&self, username: &str, devserver_id: &str, uid: Uuid, router: Router) {
        let token = format!("tok-{}", Uuid::new_v4().simple());
        self.register_tunnel_with_token(&token, username, devserver_id, uid, router)
            .await;
    }

    /// Register a tunnel whose token-resolved devserver id differs
    /// from the client's Hello workspace name. Production-shaped ids
    /// are 64 hex chars, which `is_valid_workspace_name` (max 32)
    /// rejects on the client dial; the registry keys on the
    /// token-resolved id regardless, so the Hello name is a short
    /// advisory slug here.
    async fn register_tunnel_hello(
        &self,
        username: &str,
        devserver_id: &str,
        hello: &str,
        uid: Uuid,
        router: Router,
    ) {
        let token = format!("tok-{}", Uuid::new_v4().simple());
        self.stub.add(&token, uid, username, devserver_id);
        spawn_tunnel_client(self.tunnel_addr, &token, hello, router).await;
        for _ in 0..50 {
            if self.registry.get(username, devserver_id).is_some() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("tunnel for {username}/{devserver_id} did not register");
    }

    async fn register_tunnel_with_token(
        &self,
        token: &str,
        username: &str,
        devserver_id: &str,
        uid: Uuid,
        router: Router,
    ) {
        // The tunnel-server keys the registration on the token-resolved
        // devserver_id, so the stub returns it; the registry's second key
        // is this value (Hello.workspace is not the identity source).
        self.stub.add(token, uid, username, devserver_id);
        spawn_tunnel_client(self.tunnel_addr, token, devserver_id, router).await;
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

/// Mint a devserver-gate token of the requested shape. Tests use this to
/// build URLs and cookies the proxy gate will accept (or to forge
/// near-misses for the negative cases).
fn mint(typ: devserver_gate::TokenType, sub: Uuid, drv: &str, aud: &str) -> String {
    mint_role(typ, sub, "owner", drv, aud)
}

fn mint_role(
    typ: devserver_gate::TokenType,
    sub: Uuid,
    role: &str,
    drv: &str,
    aud: &str,
) -> String {
    mint_identity(typ, sub, role, drv, aud, Default::default())
}

fn mint_identity(
    typ: devserver_gate::TokenType,
    sub: Uuid,
    role: &str,
    drv: &str,
    aud: &str,
    identity: devserver_gate::CallerIdentity,
) -> String {
    match typ {
        devserver_gate::TokenType::Entry => {
            devserver_gate::encode_entry(DEVSERVER_GATE_SECRET, sub, role, drv, aud, identity)
                .unwrap()
        }
        devserver_gate::TokenType::Session => {
            devserver_gate::encode_session(DEVSERVER_GATE_SECRET, sub, role, drv, aud, identity)
                .unwrap()
        }
    }
}

fn host_for(user: &str) -> String {
    format!("{user}{WILDCARD_SUFFIX}")
}

/// Disc host for a devserver: `{user}--{first 12 hex of id}.<suffix>`.
fn disc_host_for(user: &str, devserver_id: &str) -> String {
    format!("{user}--{}{WILDCARD_SUFFIX}", &devserver_id[..12])
}

// 64-hex devserver ids with distinct 12-char prefixes, plus a pair
// sharing one prefix for the ambiguity case.
const DS_A: &str = "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa7777bbbb8888";
const DS_B: &str = "bbbb1111cccc2222dddd3333eeee4444ffff5555aaaa6666bbbb7777cccc8888";
const DS_AMB1: &str = "9999aaaa88881111111111111111111111111111111111111111111111111111";
const DS_AMB2: &str = "9999aaaa88882222222222222222222222222222222222222222222222222222";

/// A `Cookie` header value carrying a valid session token for
/// `(sub, workspace)` on `host`. Every reverse-proxy request must pass
/// the gate now that there is no un-gated public path.
fn session_cookie(sub: Uuid, workspace: &str, host: &str) -> String {
    let session = mint(devserver_gate::TokenType::Session, sub, workspace, host);
    format!("devserver_gate={session}")
}

fn session_cookie_role(sub: Uuid, role: &str, workspace: &str, host: &str) -> String {
    let session = mint_role(
        devserver_gate::TokenType::Session,
        sub,
        role,
        workspace,
        host,
    );
    format!("devserver_gate={session}")
}

fn session_and_csrf_cookie(sub: Uuid, workspace: &str, host: &str, csrf: &str) -> String {
    format!(
        "{}; devserver_csrf={csrf}",
        session_cookie(sub, workspace, host)
    )
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
async fn health_and_readiness_are_not_exposed_on_tenant_or_unknown_hosts() {
    let app = TestApp::new().await;
    for host in [host_for("alice"), "evil.example.com".to_string()] {
        for path in ["/healthz", "/readyz"] {
            let (status, _, _) = send_host(&app.router, Method::GET, &host, path, &[]).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{host}{path}");
        }
    }
    app.cleanup().await;
}

#[tokio::test]
async fn apex_readyz_reflects_control_readiness() {
    let registry = Registry::new();
    let app = TestApp::new().await;
    let cfg = app.router.clone();
    let (status, _, _) = send_host(&cfg, Method::GET, APEX_HOST, "/readyz", &[]).await;
    assert_eq!(status, StatusCode::OK);

    let test_cfg = Arc::new(Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        tunnel_bind_addr: "127.0.0.1:0".parse().unwrap(),
        apex_host: APEX_HOST.into(),
        wildcard_suffix: WILDCARD_SUFFIX.into(),
        identity_url: "http://127.0.0.1:7000/".parse().unwrap(),
        identity_auth_token: "unused-in-tests".into(),
        dashboard_url: "https://id.chan.app/workspaces".into(),
        workspace_gate_secret: std::str::from_utf8(DEVSERVER_GATE_SECRET).unwrap().into(),
        control_url: "http://127.0.0.1:7101/".parse().unwrap(),
        proxy_token: "unused-control-token".into(),
        proxy_id: ProxyId::parse("p1").unwrap(),
        proxy_base_url: CanonicalOrigin::parse("https://p1.devserver.chan.app").unwrap(),
        max_response_bytes: None,
        max_request_bytes: None,
        request_timeout: None,
        ws_idle_timeout: DEFAULT_WS_IDLE_TIMEOUT,
        forwarded_proto: "https".into(),
    });
    let (_readiness, readiness_rx) = watch::channel(false);
    let unready = dp_http::router(test_cfg, registry, readiness_rx);
    let (status, _, _) = send_host(&unready, Method::GET, APEX_HOST, "/readyz", &[]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
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
    let set_cookies: Vec<&str> = hdrs
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect();
    assert!(
        set_cookies.iter().any(|v| v.starts_with("devserver_csrf=")
            && v.contains("Path=/")
            && !v.contains("HttpOnly")),
        "csrf cookie missing from {set_cookies:?}",
    );
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
    let bad = devserver_gate::encode_entry(
        b"some-other-secret-32-bytes-foobaa",
        uid,
        "owner",
        "blog",
        &host,
        Default::default(),
    )
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

#[tokio::test]
async fn unsafe_methods_require_matching_csrf_header() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    let client = reqwest::Client::new();
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let res = client
            .request(method.clone(), format!("http://{proxy_addr}/blog/mutate"))
            .header(header::HOST, &host)
            .header(header::COOKIE, session_cookie(uid, "blog", &host))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{method}");

        let csrf = "csrf-test-token";
        let res = client
            .request(method.clone(), format!("http://{proxy_addr}/blog/mutate"))
            .header(header::HOST, &host)
            .header(
                header::COOKIE,
                session_and_csrf_cookie(uid, "blog", &host, csrf),
            )
            .header("x-chan-csrf", csrf)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{method}");
    }

    assert_eq!(captured.requests.lock().unwrap().len(), 3);
    app.cleanup().await;
}

#[tokio::test]
async fn csrf_header_is_stripped_from_upstream() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let captured = Captured::default();
    app.register_tunnel("alice", "blog", uid, capturing_router(captured.clone()))
        .await;

    let host = host_for("alice");
    let csrf = "csrf-test-token";
    let proxy_addr = serve_router_real(app.router.clone()).await;
    reqwest::Client::new()
        .post(format!("http://{proxy_addr}/blog/mutate"))
        .header(header::HOST, &host)
        .header(
            header::COOKIE,
            session_and_csrf_cookie(uid, "blog", &host, csrf),
        )
        .header("x-chan-csrf", csrf)
        .send()
        .await
        .unwrap();

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    assert!(headers.get("x-chan-csrf").is_none());
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

#[tokio::test]
async fn gateway_assertion_matches_authenticated_session() {
    let app = TestApp::new().await;
    let owner = Uuid::new_v4();
    let caller = Uuid::new_v4();
    let devserver_id = "blog";
    let tunnel_token = "tok-gateway-assertion";
    let captured = Captured::default();
    app.register_tunnel_with_token(
        tunnel_token,
        "alice",
        devserver_id,
        owner,
        capturing_router(captured.clone()),
    )
    .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    let res = reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/assertion"))
        .header(header::HOST, &host)
        .header(
            header::COOKIE,
            session_cookie_role(caller, "editor", devserver_id, &host),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    let assertion = headers
        .get(chan_tunnel_proto::gateway_assertion::HEADER_NAME)
        .expect("gateway assertion header")
        .to_str()
        .unwrap();
    let key = chan_tunnel_proto::gateway_assertion::derive_assertion_key(tunnel_token);
    let claims = chan_tunnel_proto::gateway_assertion::verify(&key, assertion, &host, devserver_id)
        .expect("assertion verifies with tunnel token derived key");
    assert_eq!(claims.sub, caller.to_string());
    assert_eq!(claims.role, "editor");
    assert_eq!(claims.aud, host);
    assert_eq!(claims.drv, devserver_id);
    app.cleanup().await;
}

#[tokio::test]
async fn gateway_assertion_carries_entry_identity() {
    // Full identity chain: entry token (as identity-service mints it,
    // with name/email) -> session cookie minted by the gate -> the
    // per-request assertion the upstream sees.
    let app = TestApp::new().await;
    let owner = Uuid::new_v4();
    let caller = Uuid::new_v4();
    let devserver_id = "blog";
    let tunnel_token = "tok-assertion-identity";
    let captured = Captured::default();
    app.register_tunnel_with_token(
        tunnel_token,
        "alice",
        devserver_id,
        owner,
        capturing_router(captured.clone()),
    )
    .await;

    let host = host_for("alice");
    let identity = devserver_gate::CallerIdentity {
        name: Some("Alice Doe".to_string()),
        email: Some("alice@example.com".to_string()),
    };
    let entry = mint_identity(
        devserver_gate::TokenType::Entry,
        caller,
        "editor",
        devserver_id,
        &host,
        identity,
    );
    let (s, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        &host,
        &format!("/blog/?t={entry}"),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    let session = hdrs
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|v| v.starts_with("devserver_gate="))
        .expect("session cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let proxy_addr = serve_router_real(app.router.clone()).await;
    let res = reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/assertion"))
        .header(header::HOST, &host)
        .header(header::COOKIE, &session)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    let assertion = headers
        .get(chan_tunnel_proto::gateway_assertion::HEADER_NAME)
        .expect("gateway assertion header")
        .to_str()
        .unwrap();
    let key = chan_tunnel_proto::gateway_assertion::derive_assertion_key(tunnel_token);
    let claims = chan_tunnel_proto::gateway_assertion::verify(&key, assertion, &host, devserver_id)
        .expect("assertion verifies");
    assert_eq!(claims.sub, caller.to_string());
    assert_eq!(claims.name.as_deref(), Some("Alice Doe"));
    assert_eq!(claims.email.as_deref(), Some("alice@example.com"));
    app.cleanup().await;
}

#[tokio::test]
async fn gateway_assertion_empty_identity_for_legacy_session() {
    // A session cookie minted before identity claims existed carries
    // no name/email; the assertion must verify and carry None for
    // both rather than failing or inventing values.
    let app = TestApp::new().await;
    let owner = Uuid::new_v4();
    let caller = Uuid::new_v4();
    let devserver_id = "blog";
    let tunnel_token = "tok-assertion-legacy";
    let captured = Captured::default();
    app.register_tunnel_with_token(
        tunnel_token,
        "alice",
        devserver_id,
        owner,
        capturing_router(captured.clone()),
    )
    .await;

    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    let res = reqwest::Client::new()
        .get(format!("http://{proxy_addr}/blog/assertion"))
        .header(header::HOST, &host)
        .header(
            header::COOKIE,
            session_cookie_role(caller, "viewer", devserver_id, &host),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = captured.requests.lock().unwrap()[0].headers.clone();
    let assertion = headers
        .get(chan_tunnel_proto::gateway_assertion::HEADER_NAME)
        .expect("gateway assertion header")
        .to_str()
        .unwrap();
    let key = chan_tunnel_proto::gateway_assertion::derive_assertion_key(tunnel_token);
    let claims = chan_tunnel_proto::gateway_assertion::verify(&key, assertion, &host, devserver_id)
        .expect("assertion verifies");
    assert_eq!(claims.name, None);
    assert_eq!(claims.email, None);
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
// Multi-devserver routing (disc hosts + bare-host compat)
// ---------------------------------------------------------------

#[tokio::test]
async fn disc_hosts_route_to_their_devservers() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let up_a = Router::new().fallback(|| async { "ds-a" });
    let up_b = Router::new().fallback(|| async { "ds-b" });
    app.register_tunnel_hello("alice", DS_A, "ws-a", uid, up_a)
        .await;
    app.register_tunnel_hello("alice", DS_B, "ws-b", uid, up_b)
        .await;

    let proxy_addr = serve_router_real(app.router.clone()).await;
    for (id, body) in [(DS_A, "ds-a"), (DS_B, "ds-b")] {
        let host = disc_host_for("alice", id);
        let session = mint(devserver_gate::TokenType::Session, uid, id, &host);
        let res = reqwest::Client::new()
            .get(format!("http://{proxy_addr}/blog/"))
            .header(header::HOST, &host)
            .header(header::COOKIE, format!("devserver_gate={session}"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), body);
    }
    app.cleanup().await;
}

#[tokio::test]
async fn bare_host_with_two_live_routes_by_credential() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    let up_a = Router::new().fallback(|| async { "ds-a" });
    let up_b = Router::new().fallback(|| async { "ds-b" });
    app.register_tunnel_hello("alice", DS_A, "ws-a", uid, up_a)
        .await;
    app.register_tunnel_hello("alice", DS_B, "ws-b", uid, up_b)
        .await;

    // Same bare host both times; the session's drv claim picks the
    // devserver.
    let host = host_for("alice");
    let proxy_addr = serve_router_real(app.router.clone()).await;
    for (id, body) in [(DS_A, "ds-a"), (DS_B, "ds-b")] {
        let session = mint(devserver_gate::TokenType::Session, uid, id, &host);
        let res = reqwest::Client::new()
            .get(format!("http://{proxy_addr}/blog/"))
            .header(header::HOST, &host)
            .header(header::COOKIE, format!("devserver_gate={session}"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), body);
    }
    app.cleanup().await;
}

#[tokio::test]
async fn bare_host_with_two_live_and_no_credential_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel_hello("alice", DS_A, "ws-a", uid, Router::new())
        .await;
    app.register_tunnel_hello("alice", DS_B, "ws-b", uid, Router::new())
        .await;

    let (s, _, _) = send_host(&app.router, Method::GET, &host_for("alice"), "/blog/", &[]).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn bare_host_entry_token_mints_session_for_its_devserver() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel_hello("alice", DS_A, "ws-a", uid, Router::new())
        .await;
    app.register_tunnel_hello("alice", DS_B, "ws-b", uid, Router::new())
        .await;

    let host = host_for("alice");
    let entry = mint(devserver_gate::TokenType::Entry, uid, DS_B, &host);
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
    let cookie = set
        .strip_prefix("devserver_gate=")
        .and_then(|s| s.split(';').next())
        .unwrap();
    // The minted session is bound to the entry's devserver, not to
    // whichever live registration happens to sort first.
    devserver_gate::decode(
        DEVSERVER_GATE_SECRET,
        cookie,
        devserver_gate::TokenType::Session,
        &host,
        DS_B,
    )
    .expect("session should be bound to the entry's devserver");
    app.cleanup().await;
}

#[tokio::test]
async fn ambiguous_disc_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    // Two live devservers sharing the same 12-hex prefix: the disc
    // host cannot pick one, even with a valid credential.
    app.register_tunnel_hello("alice", DS_AMB1, "ws-1", uid, Router::new())
        .await;
    app.register_tunnel_hello("alice", DS_AMB2, "ws-2", uid, Router::new())
        .await;

    let host = disc_host_for("alice", DS_AMB1);
    let session = mint(devserver_gate::TokenType::Session, uid, DS_AMB1, &host);
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

#[tokio::test]
async fn unknown_disc_is_404() {
    let app = TestApp::new().await;
    let uid = Uuid::new_v4();
    app.register_tunnel_hello("alice", DS_A, "ws-a", uid, Router::new())
        .await;

    // Well-formed disc host naming a devserver that is not live.
    let host = disc_host_for("alice", DS_B);
    let session = mint(devserver_gate::TokenType::Session, uid, DS_A, &host);
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

#[tokio::test]
async fn credential_for_other_users_devserver_never_routes() {
    let app = TestApp::new().await;
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    app.register_tunnel_hello("alice", DS_A, "ws-a", alice, Router::new())
        .await;
    app.register_tunnel_hello("bob", DS_B, "ws-b", bob, Router::new())
        .await;

    // A session minted for bob's devserver on bob's host: replaying
    // it on alice's hosts (bare and disc) must 404. The aud claim
    // binds the credential to bob's host, so the bare-host drv loop
    // over alice's live set can never verify it.
    let session = mint(
        devserver_gate::TokenType::Session,
        bob,
        DS_B,
        &host_for("bob"),
    );
    for host in [host_for("alice"), disc_host_for("alice", DS_A)] {
        let (s, _, _) = send_host(
            &app.router,
            Method::GET,
            &host,
            "/blog/",
            &[("cookie", &format!("devserver_gate={session}"))],
        )
        .await;
        assert_eq!(s, StatusCode::NOT_FOUND, "host {host}");
    }
    app.cleanup().await;
}

#[tokio::test]
async fn disc_wildcard_root_redirects_to_dashboard() {
    let app = TestApp::new().await;
    let (s, hdrs, _) = send_host(
        &app.router,
        Method::GET,
        &disc_host_for("alice", DS_A),
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
// WS bridge idle semantics
// ---------------------------------------------------------------

/// Sub-second idle window for the bridge tests: long enough that
/// handshakes and scheduling jitter never trip it, short enough that
/// the cut is observable in a unit-test budget.
const WS_TEST_IDLE: std::time::Duration = std::time::Duration::from_millis(600);

/// Serve the proxy router on a real listener: a WS upgrade needs a
/// live connection, which `oneshot` cannot provide. The server task
/// is aborted by the caller when the test ends.
async fn serve_proxy(router: Router) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    (addr, task)
}

/// Dial a gated WS through the proxy: TCP to the test listener, Host
/// riding the request URI, session cookie passing the gate.
async fn ws_connect(
    addr: SocketAddr,
    host: &str,
    path: &str,
    cookie: &str,
) -> tokio_tungstenite::WebSocketStream<TcpStream> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let tcp = TcpStream::connect(addr).await.unwrap();
    tcp.set_nodelay(true).unwrap();
    let mut request = format!("ws://{host}{path}").into_client_request().unwrap();
    request
        .headers_mut()
        .insert(header::COOKIE, cookie.parse().unwrap());
    let (ws, _resp) = tokio_tungstenite::client_async(request, tcp)
        .await
        .expect("ws handshake through the proxy");
    ws
}

/// Upstream devserver router with three WS personalities: `stream`
/// pushes a text frame every 100ms unprompted, `echo` answers each
/// text frame and sends nothing on its own, `sink` reads and
/// discards everything.
fn ws_upstream_router() -> Router {
    use axum::extract::ws::{Message as AxMessage, WebSocketUpgrade as AxUpgrade};
    Router::new()
        .route(
            "/blog/ws-stream",
            axum::routing::get(|ws: AxUpgrade| async move {
                ws.on_upgrade(|mut socket| async move {
                    let mut n = 0u64;
                    loop {
                        n += 1;
                        if socket
                            .send(AxMessage::text(format!("tick-{n}")))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                })
            }),
        )
        .route(
            "/blog/ws-echo",
            axum::routing::get(|ws: AxUpgrade| async move {
                ws.on_upgrade(|mut socket| async move {
                    while let Some(Ok(msg)) = socket.recv().await {
                        if let AxMessage::Text(t) = msg {
                            if socket.send(AxMessage::Text(t)).await.is_err() {
                                break;
                            }
                        }
                    }
                })
            }),
        )
        .route(
            "/blog/ws-sink",
            axum::routing::get(|ws: AxUpgrade| async move {
                ws.on_upgrade(
                    |mut socket| async move { while let Some(Ok(_)) = socket.recv().await {} },
                )
            }),
        )
}

#[tokio::test]
async fn ws_bridge_survives_idle_window_while_upstream_streams() {
    use tokio_tungstenite::tungstenite::Message as WsMsg;
    let app = TestApp::new_with_ws_idle_timeout(WS_TEST_IDLE).await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, ws_upstream_router())
        .await;
    let (addr, server) = serve_proxy(app.router.clone()).await;

    let host = host_for("alice");
    let cookie = session_cookie(uid, "blog", &host);
    let mut ws = ws_connect(addr, &host, "/blog/ws-stream", &cookie).await;

    // Zero client->upstream frames for 3x the idle window: the shared
    // window must keep resetting on upstream->client traffic alone.
    let hold_until = tokio::time::Instant::now() + 3 * WS_TEST_IDLE;
    let mut ticks = 0u32;
    while tokio::time::Instant::now() < hold_until {
        match tokio::time::timeout(std::time::Duration::from_millis(500), ws.next()).await {
            Ok(Some(Ok(WsMsg::Text(_)))) => ticks += 1,
            Ok(Some(Ok(WsMsg::Close(frame)))) => {
                panic!("bridge cut a streaming socket: {frame:?}")
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("ws error on a streaming socket: {e}"),
            Ok(None) => panic!("bridge dropped a streaming socket"),
            Err(_) => panic!("stream stalled past the tick interval"),
        }
    }
    assert!(ticks >= 12, "expected steady ticks over 1.8s, got {ticks}");
    server.abort();
    app.cleanup().await;
}

#[tokio::test]
async fn ws_bridge_survives_idle_window_on_client_frames_alone() {
    use tokio_tungstenite::tungstenite::Message as WsMsg;
    let app = TestApp::new_with_ws_idle_timeout(WS_TEST_IDLE).await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, ws_upstream_router())
        .await;
    let (addr, server) = serve_proxy(app.router.clone()).await;

    let host = host_for("alice");
    let cookie = session_cookie(uid, "blog", &host);
    let mut ws = ws_connect(addr, &host, "/blog/ws-sink", &cookie).await;

    // The upstream never sends; client frames every 150ms must keep
    // the bridge open well past the idle window. If the client
    // direction failed to reset the shared window, the cut would land
    // mid-send phase and surface as a Close below.
    let send_until = tokio::time::Instant::now() + 3 * WS_TEST_IDLE;
    while tokio::time::Instant::now() < send_until {
        ws.send(WsMsg::text("ping")).await.expect("send while live");
        match tokio::time::timeout(std::time::Duration::from_millis(150), ws.next()).await {
            Err(_) => {} // nothing inbound: the sink stays silent
            Ok(Some(Ok(WsMsg::Close(frame)))) => {
                panic!("bridge cut a client-active socket: {frame:?}")
            }
            Ok(Some(Ok(_))) => {}
            Ok(other) => panic!("client-active socket ended early: {other:?}"),
        }
    }

    // Now go quiet: the cut must arrive roughly one idle window later,
    // proving the bridge was alive until the LAST client frame.
    let quiet_started = tokio::time::Instant::now();
    let closed = tokio::time::timeout(4 * WS_TEST_IDLE, async {
        loop {
            match ws.next().await {
                Some(Ok(WsMsg::Close(frame))) => break frame,
                Some(Ok(_)) => continue,
                other => panic!("expected a Close frame, got {other:?}"),
            }
        }
    })
    .await
    .expect("idle cut must arrive after the client goes quiet");
    let elapsed = quiet_started.elapsed();
    assert!(
        elapsed >= WS_TEST_IDLE.mul_f32(0.75),
        "cut arrived before the idle window elapsed: {elapsed:?}"
    );
    let frame = closed.expect("close carries code and reason");
    assert_eq!(u16::from(frame.code), 1001, "going away");
    server.abort();
    app.cleanup().await;
}

#[tokio::test]
async fn ws_bridge_cuts_both_idle_socket_with_a_close_frame() {
    use tokio_tungstenite::tungstenite::Message as WsMsg;
    let app = TestApp::new_with_ws_idle_timeout(WS_TEST_IDLE).await;
    let uid = Uuid::new_v4();
    app.register_tunnel("alice", "blog", uid, ws_upstream_router())
        .await;
    let (addr, server) = serve_proxy(app.router.clone()).await;

    let host = host_for("alice");
    let cookie = session_cookie(uid, "blog", &host);
    let mut ws = ws_connect(addr, &host, "/blog/ws-echo", &cookie).await;

    // Prove the socket is live end to end, then go silent in both
    // directions.
    ws.send(WsMsg::text("hello")).await.unwrap();
    let echoed = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .expect("echo within budget")
        .expect("socket open")
        .expect("clean frame");
    assert_eq!(echoed, WsMsg::text("hello"));

    // The client half must observe a real Close frame (code + reason),
    // not an abrupt FIN, and not before the idle window has elapsed.
    let quiet_started = tokio::time::Instant::now();
    let closed = tokio::time::timeout(4 * WS_TEST_IDLE, async {
        loop {
            match ws.next().await {
                Some(Ok(WsMsg::Close(frame))) => break frame,
                Some(Ok(_)) => continue,
                other => panic!("expected a Close frame, got {other:?}"),
            }
        }
    })
    .await
    .expect("both-idle cut must arrive");
    let elapsed = quiet_started.elapsed();
    assert!(
        elapsed >= WS_TEST_IDLE.mul_f32(0.75),
        "cut arrived before the idle window elapsed: {elapsed:?}"
    );
    let frame = closed.expect("close carries code and reason");
    assert_eq!(u16::from(frame.code), 1001, "going away");
    assert_eq!(frame.reason.as_str(), "idle timeout");

    // After the Close the stream ends cleanly.
    match tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await {
        Ok(None) | Ok(Some(Err(_))) => {}
        other => panic!("socket should end after the Close, got {other:?}"),
    }
    server.abort();
    app.cleanup().await;
}
