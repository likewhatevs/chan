//! Integration tests for `/desktop/authorize`.
//!
//! Each test gets its own throwaway Postgres schema (for
//! tower-sessions + api_tokens) and wiremock mocks for profile + the
//! GitHub OAuth endpoints. Exercises the full flow:
//!
//!   * unauthenticated bounce → `/`
//!   * OAuth completion → `/desktop/authorize/consent`
//!   * consent page render (CSRF nonce, security headers)
//!   * allow / deny POST → 200 handoff page embedding
//!     `chan://auth/callback#code=...` (meta refresh + fallback link);
//!     neither the fragment nor the page ever carries the PAT secret
//!   * redeem: one-time code → PAT once, replayed / unknown → 410
//!   * audit rows record `created_via_desktop` + `desktop.redeem`

#[path = "../../../tests-shared/pg_reaper.rs"]
mod pg_reaper;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::json;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tower::ServiceExt;
use tower_sessions_sqlx_store::PostgresStore;
use url::Url;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use identity::api_tokens::{ApiTokenService, RequestMeta};
use identity::config::Config;
use identity::http;
use identity::profile_client::ProfileClient;
use identity::providers::github::{GitHubEndpoints, GitHubProvider};

const PROFILE_TOKEN: &str = "test-profile-token";

async fn admin_pool(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(url)
        .await
        .expect("connect admin")
}

struct TestApp {
    router: Router,
    api_tokens: ApiTokenService,
    schema: String,
    admin_url: String,
    profile: MockServer,
    github: MockServer,
}

impl TestApp {
    async fn new() -> Self {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set; e.g. postgres://localhost/chan_gateway_test");
        pg_reaper::reap_idle(&url).await;
        let schema = format!("t_{}", Uuid::new_v4().simple());

        let admin = admin_pool(&url).await;
        sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
            .execute(&admin)
            .await
            .expect("create schema");
        admin.close().await;

        let s = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _meta| {
                let s = s.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO \"{s}\", public"))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .expect("connect pool");

        let store = PostgresStore::new(pool.clone());
        store.migrate().await.expect("migrate sessions");

        let profile = MockServer::start().await;
        let github = MockServer::start().await;

        let profile_url: Url = profile.uri().parse().unwrap();
        let profile_client =
            ProfileClient::new(profile_url, PROFILE_TOKEN.into()).expect("profile client");

        let github_endpoints = GitHubEndpoints {
            auth: format!("{}/login/oauth/authorize", github.uri()),
            token: format!("{}/login/oauth/access_token", github.uri()),
            user: format!("{}/user", github.uri()),
            emails: format!("{}/user/emails", github.uri()),
        };
        let provider = GitHubProvider::with_endpoints(
            "client-id".into(),
            "client-secret".into(),
            github_endpoints,
        )
        .expect("github provider");

        let base_url: Url = "http://localhost:7000/".parse().unwrap();

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrate identity tables");

        let api_tokens = ApiTokenService::new(pool.clone());

        let cfg = Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url,
            database_url: url.clone(),
            cookie_secure: false,
            profile_client,
            internal_auth_token: "test-internal".to_string(),
            identity_admin_token: String::new(),
            devserver_wildcard_suffix: ".devserver.chan.app".to_string(),
            workspace_public_scheme: "https".to_string(),
            workspace_public_port: String::new(),
            workspace_admin: None,
            workspace_gate_secret: "test-workspace-gate-secret-32-bytes-aa".to_string(),
            providers: vec![Arc::new(provider)],
        });

        let router = http::router(
            cfg,
            store,
            api_tokens.clone(),
            identity::token_throttle::TokenThrottle::new(),
        );

        Self {
            router,
            api_tokens,
            schema,
            admin_url: url,
            profile,
            github,
        }
    }

    async fn cleanup(self) {
        let admin = admin_pool(&self.admin_url).await;
        let _ = sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .execute(&admin)
            .await;
        admin.close().await;
    }

    /// Insert a row in `users` with `id`. Mirrors what
    /// profile-service would have done so the api_tokens FK to users
    /// resolves on PAT create.
    async fn insert_user(&self, id: Uuid, email: &str) {
        let url = self.admin_url.clone();
        let s = self.schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .after_connect(move |conn, _meta| {
                let s = s.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO \"{s}\", public"))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .expect("connect for insert_user");
        sqlx::query(
            "INSERT INTO users (id, email, username) VALUES \
             ($1, $2, 'u' || substr(replace($1::text, '-', ''), 1, 12))",
        )
        .bind(id)
        .bind(email)
        .execute(&pool)
        .await
        .expect("insert user row");
        pool.close().await;
    }
}

/// Stateful client that keeps the session cookie across calls.
struct Client<'a> {
    app: &'a TestApp,
    cookie: Option<String>,
}

impl<'a> Client<'a> {
    fn new(app: &'a TestApp) -> Self {
        Self { app, cookie: None }
    }

    async fn send(
        &mut self,
        method: Method,
        uri: &str,
        content_type: Option<&str>,
        body: Body,
    ) -> Sent {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(c) = &self.cookie {
            builder = builder.header(header::COOKIE, c.clone());
        }
        if let Some(ct) = content_type {
            builder = builder.header(header::CONTENT_TYPE, ct);
        }
        let req = builder.body(body).unwrap();
        let res = self.app.router.clone().oneshot(req).await.unwrap();
        let status = res.status();

        if let Some(set_cookie) = res
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .next()
            .and_then(|v| v.to_str().ok())
        {
            let pair = set_cookie.split(';').next().unwrap_or("").to_string();
            self.cookie = Some(pair);
        }
        let location = res
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let headers: Vec<(String, String)> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        Sent {
            status,
            location,
            headers,
            body: bytes.to_vec(),
        }
    }

    async fn get(&mut self, uri: &str) -> Sent {
        self.send(Method::GET, uri, None, Body::empty()).await
    }

    async fn post_form(&mut self, uri: &str, fields: &[(&str, &str)]) -> Sent {
        let encoded: String = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields.iter().copied())
            .finish();
        self.send(
            Method::POST,
            uri,
            Some("application/x-www-form-urlencoded"),
            Body::from(encoded),
        )
        .await
    }

    async fn post_json(&mut self, uri: &str, body: &serde_json::Value) -> Sent {
        self.send(
            Method::POST,
            uri,
            Some("application/json"),
            Body::from(body.to_string()),
        )
        .await
    }
}

struct Sent {
    status: StatusCode,
    location: String,
    #[allow(dead_code)]
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Sent {
    fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).expect("utf8 body")
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

fn extract_oauth_state(authorize_url: &str) -> String {
    let u = Url::parse(authorize_url).unwrap();
    u.query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .expect("state param")
}

/// Parse a `chan://auth/callback#k=v&k2=v2` fragment into a map.
fn parse_chan_fragment(url: &str) -> std::collections::HashMap<String, String> {
    let frag = url.split_once('#').map(|(_, f)| f).unwrap_or("");
    url::form_urlencoded::parse(frag.as_bytes())
        .into_owned()
        .collect()
}

/// Pull the CSRF token out of the consent page HTML.
fn extract_csrf(html: &str) -> String {
    let needle = r#"name="csrf" value=""#;
    let start = html.find(needle).expect("csrf input present");
    let after = &html[start + needle.len()..];
    let end = after.find('"').expect("csrf value closes");
    after[..end].to_string()
}

/// Pull the `chan://` target out of the handoff page HTML and assert
/// it appears exactly twice (the meta refresh + the fallback link).
/// The attribute-escaped URL can only contain `&amp;` entities
/// (percent-encoding covers every other breaker), so unescaping is a
/// single replace.
fn extract_handoff_url(html: &str) -> String {
    let needle = r#"href="chan://"#;
    let start = html.find(needle).expect("handoff link present");
    let after = &html[start + r#"href=""#.len()..];
    let end = after.find('"').expect("href value closes");
    let escaped = &after[..end];
    assert_eq!(
        html.matches(escaped).count(),
        2,
        "the target rides the meta refresh AND the link: {html}"
    );
    escaped.replace("&amp;", "&")
}

/// Workspace an end-to-end OAuth callback so the test client ends up
/// holding an authenticated session cookie tied to `user_id`.
async fn happy_login(app: &TestApp, c: &mut Client<'_>, user_id: Uuid, email: &str) {
    let resp = c.get("/auth/github").await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let state = extract_oauth_state(&resp.location);

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gh-access",
            "token_type": "Bearer",
            "scope": "read:user,user:email",
        })))
        .mount(&app.github)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 999,
            "login": "octocat",
            "name": "Octo Cat",
            "email": email,
        })))
        .mount(&app.github)
        .await;

    let now = chrono::Utc::now().to_rfc3339();
    let user_body = json!({
        "id": user_id,
        "email": email,
        "display_name": "Octo Cat",
        "username": format!("u{}", &user_id.simple().to_string()[..12]),
        "username_edits": 0,
        "created_at": now,
        "updated_at": now,
    });
    Mock::given(method("POST"))
        .and(path("/v1/users/upsert-by-identity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": user_body.clone(),
            "user_created": true,
            "identity_created": true,
        })))
        .mount(&app.profile)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/users/{user_id}/grants/claim")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"claimed": 0})))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}/flags")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"oauth_login": true})))
        .mount(&app.profile)
        .await;

    let resp = c
        .get(&format!("/auth/github/callback?code=fake&state={state}"))
        .await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER, "callback should 303");
}

/// Stand up the `GET /v1/users/{uid}` mock so /desktop/authorize and
/// the consent / confirm handlers can look the user up. `blocked`
/// controls the response shape: blocked sets `blocked_at` to now.
async fn mock_get_user(app: &TestApp, user_id: Uuid, email: &str, blocked: bool) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json(user_id, email, blocked)))
        .mount(&app.profile)
        .await;
}

/// [`mock_get_user`] capped at `n` responses; once exhausted the next
/// matching mock serves, so a test can flip the user's blocked state
/// mid-flow without depending on wiremock mount precedence.
async fn mock_get_user_up_to(app: &TestApp, user_id: Uuid, email: &str, blocked: bool, n: u64) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json(user_id, email, blocked)))
        .up_to_n_times(n)
        .mount(&app.profile)
        .await;
}

/// The profile-service user body both `mock_get_user*` helpers serve.
fn user_json(user_id: Uuid, email: &str, blocked: bool) -> serde_json::Value {
    let now = chrono::Utc::now().to_rfc3339();
    let mut body = json!({
        "id": user_id,
        "email": email,
        "display_name": "Octo Cat",
        "username": format!("u{}", &user_id.simple().to_string()[..12]),
        "username_edits": 0,
        "created_at": now,
        "updated_at": now,
    });
    if blocked {
        body["blocked_at"] = json!(chrono::Utc::now().to_rfc3339());
        body["block_reason"] = json!("admin action");
    }
    body
}

const AUTH_URI: &str = "/desktop/authorize?\
                        redirect_uri=chan%3A%2F%2Fauth%2Fcallback&\
                        state=desktop-nonce-1&\
                        label=chan-desktop+%40+host&\
                        scopes=tunnel&\
                        expires_in=2592000";

// ---------------------------------------------------------------
// Tests
// ---------------------------------------------------------------

#[tokio::test]
async fn unauthenticated_bounces_to_root() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let resp = c.get(AUTH_URI).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    assert_eq!(resp.location, "/");
    app.cleanup().await;
}

#[tokio::test]
async fn bad_redirect_uri_returns_400() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let resp = c
        .get(
            "/desktop/authorize?redirect_uri=https%3A%2F%2Fevil.example%2Fcb&\
             state=x&label=x&expires_in=10",
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn oauth_bounce_lands_on_consent() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;

    // First: hit /desktop/authorize unauthenticated. Stashes params
    // and 303s to /. The session cookie now carries the pending
    // authorize, which auth_callback will pick up.
    let resp = c.get(AUTH_URI).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    assert_eq!(resp.location, "/");

    happy_login_resume(&app, &mut c, uid, "octo@example.com").await;
    app.cleanup().await;
}

#[tokio::test]
async fn full_flow_mints_pat_with_desktop_audit_action() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;

    happy_login(&app, &mut c, uid, "octo@example.com").await;
    // Authed GET /desktop/authorize -> 303 /desktop/authorize/consent
    mock_get_user(&app, uid, "octo@example.com", false).await;
    let resp = c.get(AUTH_URI).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    assert_eq!(resp.location, "/desktop/authorize/consent");

    // Consent page renders with CSRF + security headers.
    let resp = c.get("/desktop/authorize/consent").await;
    assert_eq!(resp.status, StatusCode::OK);
    let html = resp.body_str();
    let csrf = extract_csrf(html);
    assert!(!csrf.is_empty(), "csrf token present");
    assert_eq!(resp.header("x-frame-options"), Some("DENY"));
    assert_eq!(resp.header("cache-control"), Some("no-store"));
    let csp = resp.header("content-security-policy").unwrap();
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert!(csp.contains("img-src 'self'"), "{csp}");

    // Authorize: a 200 handoff page, NOT a redirect (a 3xx off this
    // form POST would put the chan:// hop under form-action).
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "allow")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(
        resp.location.is_empty(),
        "no Location header on the handoff"
    );
    assert_eq!(resp.header("x-frame-options"), Some("DENY"));
    assert_eq!(resp.header("cache-control"), Some("no-store"));
    assert_eq!(resp.header("referrer-policy"), Some("no-referrer"));
    let csp = resp.header("content-security-policy").unwrap();
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert!(csp.contains("img-src 'self'"), "{csp}");
    let url = extract_handoff_url(resp.body_str());
    let frag = parse_chan_fragment(&url);
    assert!(url.starts_with("chan://auth/callback#"));
    assert_eq!(
        frag.get("state").map(String::as_str),
        Some("desktop-nonce-1")
    );
    assert_eq!(
        frag.get("label").map(String::as_str),
        Some("chan-desktop @ host")
    );

    // The fragment carries a one-time code and NEVER the credential;
    // the handoff page as a whole is grep-clean of the secret.
    assert!(!frag.contains_key("secret"), "no secret key: {url}");
    assert!(!frag.contains_key("id"), "no id key: {url}");
    let code = frag.get("code").expect("code in fragment").clone();
    assert!(!code.is_empty());
    let html = resp.body_str();
    assert!(!html.contains("chan_pat_"), "PAT leaked into the handoff");
    assert!(!html.contains("secret="), "secret key in the handoff");

    // Redeem the code: 200 exactly once, with the working PAT.
    let resp = c
        .post_json("/desktop/authorize/redeem", &json!({ "code": code }))
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let redeemed: serde_json::Value = serde_json::from_slice(&resp.body).expect("redeem json");
    let secret = redeemed["secret"].as_str().expect("secret string");
    assert!(secret.starts_with("chan_pat_"), "{redeemed}");
    assert_eq!(redeemed["label"], "chan-desktop @ host");
    assert!(
        redeemed["expires_at"].is_string(),
        "expires_in was requested, so the key carries a timestamp: {redeemed}"
    );
    let token_id: Uuid = redeemed["id"].as_str().unwrap().parse().expect("uuid id");

    // The redeemed PAT works through the normal validation path.
    let validated = app
        .api_tokens
        .validate(secret, &RequestMeta::default())
        .await
        .expect("redeemed PAT validates");
    assert_eq!(validated.user_id, uid);
    assert_eq!(validated.token_id, token_id);

    // A replay of the same code is 410 with an error body.
    let resp = c
        .post_json("/desktop/authorize/redeem", &json!({ "code": code }))
        .await;
    assert_eq!(resp.status, StatusCode::GONE);
    let body: serde_json::Value = serde_json::from_slice(&resp.body).expect("error json");
    assert!(body["error"].is_string(), "{body}");

    // Audit trail: the desktop mint, the redemption, the validate.
    let entries = app
        .api_tokens
        .audit(uid, token_id, 10)
        .await
        .expect("audit");
    let mut actions: Vec<_> = entries.iter().map(|e| e.action.as_str()).collect();
    actions.sort_unstable();
    assert_eq!(
        actions,
        vec!["created_via_desktop", "desktop.redeem", "used"]
    );

    app.cleanup().await;
}

#[tokio::test]
async fn redeem_unknown_code_is_410() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let resp = c
        .post_json(
            "/desktop/authorize/redeem",
            &json!({ "code": "no-such-code" }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::GONE);
    let body: serde_json::Value = serde_json::from_slice(&resp.body).expect("error json");
    assert!(body["error"].is_string(), "{body}");
    app.cleanup().await;
}

#[tokio::test]
async fn deny_returns_user_cancelled_and_does_not_mint() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;
    mock_get_user(&app, uid, "octo@example.com", false).await;

    let resp = c.get(AUTH_URI).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    let csrf = extract_csrf(resp.body_str());

    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "deny")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body_str().contains("Request cancelled"), "deny copy");
    let url = extract_handoff_url(resp.body_str());
    let frag = parse_chan_fragment(&url);
    assert_eq!(
        frag.get("error").map(String::as_str),
        Some("user_cancelled")
    );
    assert_eq!(
        frag.get("state").map(String::as_str),
        Some("desktop-nonce-1")
    );
    // No PAT rows for this user.
    let pool = app.api_tokens.list(uid).await.expect("list");
    assert!(pool.is_empty(), "deny must not mint a token");
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_on_confirm_renders_error_handoff() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    // The user is unblocked through authorize + consent (two lookups),
    // then blocked by the time they click Authorize. Exhaustion (not
    // mount order) hands the third lookup to the blocked mock.
    mock_get_user_up_to(&app, uid, "octo@example.com", false, 2).await;
    mock_get_user(&app, uid, "octo@example.com", true).await;

    let resp = c.get(AUTH_URI).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    assert_eq!(resp.status, StatusCode::OK);
    let csrf = extract_csrf(resp.body_str());

    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "allow")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body_str().contains("Sign-in failed"), "error copy");
    let url = extract_handoff_url(resp.body_str());
    let frag = parse_chan_fragment(&url);
    assert_eq!(
        frag.get("error").map(String::as_str),
        Some("account_blocked")
    );
    // No PAT was minted for the blocked user.
    let tokens = app.api_tokens.list(uid).await.expect("list");
    assert!(tokens.is_empty(), "blocked allow must not mint a token");
    app.cleanup().await;
}

#[tokio::test]
async fn csrf_mismatch_returns_400_and_clears_pending() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;
    mock_get_user(&app, uid, "octo@example.com", false).await;

    let _ = c.get(AUTH_URI).await;
    let _ = c.get("/desktop/authorize/consent").await;

    // POST with a bogus CSRF.
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", "not-the-real-csrf"), ("action", "allow")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);

    // Pending is cleared: a retry without re-running /desktop/authorize
    // 400s instead of minting against a stale stash.
    let resp = c.get("/desktop/authorize/consent").await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

/// Smaller variant of `happy_login` used when the test client already
/// has a pending `/desktop/authorize` stashed in its session and we
/// just want to confirm `auth_callback` redirects to the consent page
/// instead of `/`.
async fn happy_login_resume(app: &TestApp, c: &mut Client<'_>, user_id: Uuid, email: &str) {
    let resp = c.get("/auth/github").await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let state = extract_oauth_state(&resp.location);

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gh-access",
            "token_type": "Bearer",
            "scope": "read:user,user:email",
        })))
        .mount(&app.github)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 999,
            "login": "octocat",
            "name": "Octo Cat",
            "email": email,
        })))
        .mount(&app.github)
        .await;
    let now = chrono::Utc::now().to_rfc3339();
    Mock::given(method("POST"))
        .and(path("/v1/users/upsert-by-identity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": {
                "id": user_id,
                "email": email,
                "display_name": "Octo Cat",
                "username": format!("u{}", &user_id.simple().to_string()[..12]),
                "username_edits": 0,
                "created_at": now,
                "updated_at": now,
            },
            "user_created": true,
            "identity_created": true,
        })))
        .mount(&app.profile)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/users/{user_id}/grants/claim")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"claimed": 0})))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}/flags")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"oauth_login": true})))
        .mount(&app.profile)
        .await;

    let resp = c
        .get(&format!("/auth/github/callback?code=fake&state={state}"))
        .await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    // The hook in auth_callback redirects to the consent page, not /.
    assert_eq!(resp.location, "/desktop/authorize/consent");
}

// ---------------------------------------------------------------
// Consent devserver picker
// ---------------------------------------------------------------

/// AUTH query whose scopes include desktop.connect, which is what
/// makes the consent page render the devserver picker.
const AUTH_URI_CONNECT: &str = "/desktop/authorize?\
                                redirect_uri=chan%3A%2F%2Fauth%2Fcallback&\
                                state=desktop-nonce-2&\
                                label=chan-desktop+%40+host&\
                                scopes=tunnel%2Cdesktop.connect&\
                                expires_in=2592000";

/// Mock the owned + incoming devserver lists the picker fetches.
async fn mock_devserver_lists(
    app: &TestApp,
    user_id: Uuid,
    owned: serde_json::Value,
    incoming: serde_json::Value,
) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}/grants/owned")))
        .respond_with(ResponseTemplate::new(200).set_body_json(owned))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}/grants/incoming")))
        .respond_with(ResponseTemplate::new(200).set_body_json(incoming))
        .mount(&app.profile)
        .await;
}

fn incoming_share_json(owner: &str, devserver_id: &str, label: &str) -> serde_json::Value {
    let now = chrono::Utc::now().to_rfc3339();
    json!({
        "grant_id": Uuid::new_v4(),
        "owner_user_id": Uuid::new_v4(),
        "owner_username": owner,
        "owner_display_name": null,
        "owner_avatar_url": null,
        "devserver_id": devserver_id,
        "label": label,
        "role": "editor",
        "accepted_at": now,
    })
}

#[tokio::test]
async fn consent_lists_devservers_and_pick_rides_the_fragment() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;
    mock_get_user(&app, uid, "octo@example.com", false).await;

    let own_id = "a".repeat(64);
    let shared_id = "b".repeat(64);
    mock_devserver_lists(
        &app,
        uid,
        json!([{"devserver_id": own_id, "label": "laptop", "grant_count": 0}]),
        json!([incoming_share_json("bob-handle", &shared_id, "bob-box")]),
    )
    .await;

    let resp = c.get(AUTH_URI_CONNECT).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    assert_eq!(resp.status, StatusCode::OK);
    let html = resp.body_str().to_string();
    let csrf = extract_csrf(&html);
    let username = format!("u{}", &uid.simple().to_string()[..12]);
    // Own row + shared row render as radios keyed <owner>:<id>.
    assert!(html.contains("laptop"), "{html}");
    assert!(
        html.contains(&format!(r#"value="{username}:{own_id}""#)),
        "{html}"
    );
    assert!(html.contains("bob-box"), "{html}");
    assert!(html.contains("from @bob-handle"), "{html}");
    assert!(
        html.contains(&format!(r#"value="bob-handle:{shared_id}""#)),
        "{html}"
    );

    // Pick the SHARED devserver; the pick rides the callback fragment
    // as the devserver_* keys next to the one-time code.
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[
                ("csrf", &csrf),
                ("action", "allow"),
                ("devserver", &format!("bob-handle:{shared_id}")),
            ],
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let url = extract_handoff_url(resp.body_str());
    let frag = parse_chan_fragment(&url);
    assert!(frag.contains_key("code"), "{url}");
    assert_eq!(
        frag.get("devserver_owner").map(String::as_str),
        Some("bob-handle"),
        "{url}"
    );
    assert_eq!(
        frag.get("devserver_id").map(String::as_str),
        Some(shared_id.as_str()),
        "{url}"
    );
    assert_eq!(
        frag.get("devserver_label").map(String::as_str),
        Some("bob-box"),
        "{url}"
    );
    app.cleanup().await;
}

#[tokio::test]
async fn consent_zero_state_still_mints_without_devserver_keys() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;
    mock_get_user(&app, uid, "octo@example.com", false).await;
    mock_devserver_lists(&app, uid, json!([]), json!([])).await;

    let resp = c.get(AUTH_URI_CONNECT).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    assert_eq!(resp.status, StatusCode::OK);
    let html = resp.body_str().to_string();
    let csrf = extract_csrf(&html);
    assert!(html.contains("No devservers yet"), "{html}");
    assert!(!html.contains(r#"type="radio""#), "{html}");

    // Authorize still mints; the fragment simply has no devserver_*.
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "allow")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let url = extract_handoff_url(resp.body_str());
    let frag = parse_chan_fragment(&url);
    assert!(frag.contains_key("code"), "{url}");
    assert!(!frag.contains_key("devserver_owner"), "{url}");
    assert!(!frag.contains_key("devserver_id"), "{url}");
    app.cleanup().await;
}

#[tokio::test]
async fn confirm_rejects_pick_not_in_lists() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = Uuid::new_v4();
    app.insert_user(uid, "octo@example.com").await;
    happy_login(&app, &mut c, uid, "octo@example.com").await;
    mock_get_user(&app, uid, "octo@example.com", false).await;
    mock_devserver_lists(
        &app,
        uid,
        json!([{"devserver_id": "a".repeat(64), "label": "laptop", "grant_count": 0}]),
        json!([]),
    )
    .await;

    // A pick that is in neither fresh list (revoked / fabricated).
    let resp = c.get(AUTH_URI_CONNECT).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    let csrf = extract_csrf(resp.body_str());
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[
                ("csrf", &csrf),
                ("action", "allow"),
                ("devserver", &format!("mallory:{}", "f".repeat(64))),
            ],
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);

    // Malformed pick (no owner:id separator): fresh flow, same 400.
    let resp = c.get(AUTH_URI_CONNECT).await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let resp = c.get("/desktop/authorize/consent").await;
    let csrf = extract_csrf(resp.body_str());
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "allow"), ("devserver", "junk")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}
