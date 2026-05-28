//! Integration tests for `/desktop/authorize`.
//!
//! Each test gets its own throwaway Postgres schema (for
//! tower-sessions + api_tokens) and wiremock mocks for profile + the
//! GitHub OAuth endpoints. Exercises the full flow:
//!
//!   * unauthenticated bounce → `/`
//!   * OAuth completion → `/desktop/authorize/consent`
//!   * consent page render (CSRF nonce, security headers)
//!   * allow / deny POST → `chan://auth/callback#...`
//!   * audit row records `created_via_desktop`

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

use identity::api_tokens::ApiTokenService;
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
            drive_wildcard_suffix: ".drive.chan.app".to_string(),
            drive_public_scheme: "https".to_string(),
            drive_public_port: String::new(),
            drive_admin: None,
            drive_gate_secret: "test-drive-gate-secret-32-bytes-aa".to_string(),
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

/// Drive an end-to-end OAuth callback so the test client ends up
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
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&app.profile)
        .await;
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
    // and 302s to /. The session cookie now carries the pending
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
    // Authed GET /desktop/authorize -> 302 /desktop/authorize/consent
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
    assert!(resp
        .header("content-security-policy")
        .unwrap()
        .contains("frame-ancestors 'none'"));

    // Authorize.
    let resp = c
        .post_form(
            "/desktop/authorize/confirm",
            &[("csrf", &csrf), ("action", "allow")],
        )
        .await;
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let frag = parse_chan_fragment(&resp.location);
    assert!(resp.location.starts_with("chan://auth/callback#"));
    assert!(frag.get("secret").unwrap().starts_with("chan_pat_"));
    assert_eq!(
        frag.get("state").map(String::as_str),
        Some("desktop-nonce-1")
    );
    assert_eq!(
        frag.get("label").map(String::as_str),
        Some("chan-desktop @ host")
    );

    // Audit row carries the desktop-specific action.
    let token_id: Uuid = frag.get("id").unwrap().parse().expect("uuid id");
    let entries = app
        .api_tokens
        .audit(uid, token_id, 10)
        .await
        .expect("audit");
    let actions: Vec<_> = entries.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(actions, vec!["created_via_desktop"]);

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
    assert_eq!(resp.status, StatusCode::SEE_OTHER);
    let frag = parse_chan_fragment(&resp.location);
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
