//! Integration tests for identity-service.
//!
//! Each test gets:
//! - its own throwaway Postgres schema (for tower-sessions);
//! - a wiremock-backed profile-service;
//! - a wiremock-backed GitHub (token + user + emails endpoints).
//!
//! Set `TEST_DATABASE_URL` to a database the test process can
//! create schemas in (same as `profile`'s tests).

#[path = "../../../tests-shared/pg_reaper.rs"]
mod pg_reaper;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Single-connection admin pool — see profile-tests for the
/// rationale (default pool size * parallel tests blows past
/// Postgres' max_connections).
async fn admin_pool(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(url)
        .await
        .expect("connect admin")
}
use tower::ServiceExt;
use tower_sessions_sqlx_store::PostgresStore;
use url::Url;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use identity::config::Config;
use identity::http;
use identity::profile_client::ProfileClient;
use identity::providers::github::{GitHubEndpoints, GitHubProvider};

const PROFILE_TOKEN: &str = "test-profile-token";

struct TestApp {
    router: Router,
    schema: String,
    admin_url: String,
    profile: MockServer,
    github: MockServer,
}

impl TestApp {
    async fn new() -> Self {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set; e.g. postgres://localhost/chan_gateway_test");
        // Reap leaked connections from prior test-process runs and
        // hold one slot for the rest of this process. See module
        // doc on `pg_reaper`.
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

        // Run our own migrations so api_tokens / users exist in this
        // test schema; tower-sessions only handles its own table.
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrate identity tables");

        let api_tokens = identity::api_tokens::ApiTokenService::new(pool.clone());

        let cfg = Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url,
            database_url: url.clone(),
            cookie_secure: false,
            profile_client,
            internal_auth_token: "test-internal".to_string(),
            workspace_wildcard_suffix: ".workspace.chan.app".to_string(),
            workspace_public_scheme: "https".to_string(),
            workspace_public_port: String::new(),
            workspace_admin: None,
            workspace_gate_secret: "test-workspace-gate-secret-32-bytes-aa".to_string(),
            providers: vec![Arc::new(provider)],
        });

        let router = http::router(
            cfg,
            store,
            api_tokens,
            identity::token_throttle::TokenThrottle::new(),
        );

        Self {
            router,
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
}

/// Tiny stateful client: keeps the session cookie between calls so
/// tests can step through login -> callback -> me.
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
        body: Option<Value>,
    ) -> (StatusCode, Vec<(String, String)>, Value, String) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(c) = &self.cookie {
            builder = builder.header(header::COOKIE, c.clone());
        }
        let body = match body {
            Some(v) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&v).unwrap())
            }
            None => Body::empty(),
        };
        let req = builder.body(body).unwrap();
        let res = self.app.router.clone().oneshot(req).await.unwrap();

        let status = res.status();
        let headers: Vec<(String, String)> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Update cookie jar from Set-Cookie if present.
        if let Some(set_cookie) = res
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .next()
            .and_then(|v| v.to_str().ok())
        {
            // Strip attributes; keep only `name=value`.
            let pair = set_cookie.split(';').next().unwrap_or("").to_string();
            self.cookie = Some(pair);
        }

        let location = res
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, headers, json, location)
    }
}

fn extract_state(authorize_url: &str) -> String {
    let u = Url::parse(authorize_url).unwrap();
    u.query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .expect("state param")
}

fn fake_user_id() -> Uuid {
    Uuid::new_v4()
}

#[tokio::test]
async fn me_unauthenticated() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn auth_start_redirects_to_provider() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, location) = c.send(Method::GET, "/auth/github", None).await;
    assert_eq!(s, StatusCode::SEE_OTHER, "expected 303");
    assert!(
        location.contains("/login/oauth/authorize"),
        "got {location}"
    );
    assert!(location.contains("state="), "got {location}");
    assert!(location.contains("code_challenge="), "got {location}");
    app.cleanup().await;
}

#[tokio::test]
async fn unknown_provider_is_404() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, _) = c.send(Method::GET, "/auth/myspace", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

async fn happy_login(app: &TestApp, c: &mut Client<'_>, user_id: Uuid, email: &str) {
    // 1. /auth/github -> redirect with state + Set-Cookie session.
    let (_, _, _, location) = c.send(Method::GET, "/auth/github", None).await;
    let state = extract_state(&location);

    // 2. wiremock GitHub: token exchange + user info.
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

    // 3. wiremock profile-service: single atomic upsert call.
    //    username + username_edits are NOT NULL on users since 0003;
    //    identity::profile_client::User deserializes them, so the
    //    mock body must include them or the callback returns 502.
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

    // 3b. callback runs a best-effort claim sweep. Tests don't care
    //     about the count; respond 0 so the warn-on-error path is
    //     not taken (which would otherwise pollute test output via
    //     `wiremock` returning 404 for an unmocked path).
    Mock::given(method("POST"))
        .and(path(format!("/v1/users/{user_id}/grants/claim")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"claimed": 0})))
        .mount(&app.profile)
        .await;

    // 3c. Feature flags. happy_login grants oauth_login + share_workspaces
    //     so the callback gate passes and the SPA gets the flags.
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{user_id}/flags")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "oauth_login": true,
            "share_workspaces": true,
        })))
        .mount(&app.profile)
        .await;

    // 4. callback -> 303 to /
    let (s, _, _, location) = c
        .send(
            Method::GET,
            &format!("/auth/github/callback?code=fake&state={state}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::SEE_OTHER, "callback should redirect");
    assert_eq!(location, "/");
}

#[tokio::test]
async fn login_then_me() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    // /api/me returns just the user; workspaces moved to workspace-proxy.
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": uid,
            "email": "octo@example.com",
            "display_name": "Octo Cat",
            "username": format!("u{}", &uid.simple().to_string()[..12]),
            "username_edits": 0,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })))
        .mount(&app.profile)
        .await;

    let (s, _, body, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["user"]["id"].as_str().unwrap(), uid.to_string());
    // Workspaces now come from workspace-proxy admin. TestApp wires
    // workspace_admin: None, so the list resolves to an empty array.
    assert_eq!(
        body["workspaces"].as_array().expect("workspaces present"),
        &Vec::<serde_json::Value>::new()
    );
    app.cleanup().await;
}

#[tokio::test]
async fn callback_state_mismatch_rejects() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);

    // Start the flow to populate the session, then send a tampered
    // state. No GitHub mocks; we should never get that far.
    let (_, _, _, _) = c.send(Method::GET, "/auth/github", None).await;
    let (s, _, _, _) = c
        .send(
            Method::GET,
            "/auth/github/callback?code=fake&state=tampered",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn logout_clears_session() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    let (s, _, _, _) = c.send(Method::POST, "/api/logout", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, _, _, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn providers_endpoint_lists_configured() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, body, _) = c.send(Method::GET, "/api/providers", None).await;
    assert_eq!(s, StatusCode::OK);
    let providers = body["providers"].as_array().unwrap();
    let names: Vec<_> = providers.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["github"]);
    app.cleanup().await;
}

#[tokio::test]
async fn delete_profile_succeeds_and_clears_session() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    // Mock profile-service DELETE /v1/users/{uid}.
    Mock::given(method("DELETE"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c.send(Method::DELETE, "/api/profile", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    // Session was flushed. /api/me with the same cookie -> 401.
    let (s, _, _, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn delete_profile_unauthenticated_is_401() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, _) = c.send(Method::DELETE, "/api/profile", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Blocked-account gates
// ---------------------------------------------------------------

/// Helper: serialize a User body with `blocked_at` set so /api/me
/// returns the row that triggers the gate.
fn blocked_user_body(uid: Uuid, email: &str) -> Value {
    let now = chrono::Utc::now().to_rfc3339();
    json!({
        "id": uid,
        "email": email,
        "display_name": "Octo Cat",
        "username": format!("u{}", &uid.simple().to_string()[..12]),
        "username_edits": 0,
        "created_at": now,
        "updated_at": now,
        "blocked_at": now,
        "block_reason": "abuse",
    })
}

#[tokio::test]
async fn me_returns_user_with_blocked_state() {
    // /api/me must NOT 403 a blocked user; the SPA needs the row
    // to render the blocked view. Other endpoints gate, but `me`
    // surfaces the state.
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, body, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::OK, "me must succeed for blocked users");
    assert!(body["user"]["blocked_at"].is_string());
    assert_eq!(body["user"]["block_reason"], "abuse");
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_user_rename_is_403() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c
        .send(
            Method::PATCH,
            "/api/me/username",
            Some(json!({"username": "newhandle"})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_user_token_create_is_403() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c
        .send(
            Method::POST,
            "/api/tokens",
            Some(json!({"label": "cli", "expires_in": null})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_user_token_list_is_403() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c.send(Method::GET, "/api/tokens", None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_user_can_still_logout() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c.send(Method::POST, "/api/logout", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT, "logout must always work");
    app.cleanup().await;
}

#[tokio::test]
async fn blocked_user_can_still_delete_account() {
    // Right to deletion: a blocked account must be able to delete
    // itself even though every other write is refused.
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("DELETE"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(blocked_user_body(uid, "octo@example.com")),
        )
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c.send(Method::DELETE, "/api/profile", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    app.cleanup().await;
}

// ---------------------------------------------------------------------------
// Workspace sharing (grants) + share landing
// ---------------------------------------------------------------------------

fn live_user_body(uid: Uuid, email: &str, username: &str) -> Value {
    let now = chrono::Utc::now().to_rfc3339();
    json!({
        "id": uid,
        "email": email,
        "display_name": "Owner",
        "username": username,
        "username_edits": 0,
        "created_at": now,
        "updated_at": now,
    })
}

fn grant_body(grant_id: Uuid, owner_id: Uuid, workspace: &str, email: &str, role: &str) -> Value {
    let now = chrono::Utc::now().to_rfc3339();
    json!({
        "id": grant_id,
        "owner_user_id": owner_id,
        "workspace_name": workspace,
        "grantee_email": email,
        "grantee_user_id": null,
        "role": role,
        "created_at": now,
        "accepted_at": null,
    })
}

#[tokio::test]
async fn grant_create_requires_session() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, _) = c
        .send(
            Method::POST,
            "/api/workspaces/photos/grants",
            Some(json!({"grantee_email": "a@b.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_validates_role() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "owner@x.com").await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            uid,
            "owner@x.com",
            "owner-handle",
        )))
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c
        .send(
            Method::POST,
            "/api/workspaces/photos/grants",
            Some(json!({"grantee_email": "a@b.com", "role": "admin"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_forwards_to_profile() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "owner@x.com").await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            uid,
            "owner@x.com",
            "owner-handle",
        )))
        .mount(&app.profile)
        .await;
    let grant_id = Uuid::new_v4();
    Mock::given(method("POST"))
        .and(path(format!("/v1/users/{uid}/workspaces/photos/grants")))
        .respond_with(ResponseTemplate::new(201).set_body_json(grant_body(
            grant_id,
            uid,
            "photos",
            "alice@x.com",
            "editor",
        )))
        .mount(&app.profile)
        .await;

    let (s, _, body, _) = c
        .send(
            Method::POST,
            "/api/workspaces/photos/grants",
            Some(json!({"grantee_email": "alice@x.com", "role": "editor"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(body["id"].as_str().unwrap(), grant_id.to_string());
    assert_eq!(body["role"], "editor");
    app.cleanup().await;
}

#[tokio::test]
async fn share_landing_unauthed_stashes_redirect() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let (s, _, _, location) = c.send(Method::GET, "/s/owner-handle/photos", None).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert_eq!(location, "/");
    // Now log in. After the callback, the redirect target should be
    // the stashed share URL, not "/".
    let uid = fake_user_id();
    // happy_login asserts location == "/" — bypass it and run the
    // OAuth steps manually so we can inspect the real location.
    let (_, _, _, _) = c.send(Method::GET, "/auth/github", None).await;
    // Pull state by re-reading the redirect from a fresh /auth call:
    // the session already has KEY_POST_LOGIN_REDIRECT set, and
    // /auth/github overwrites KEY_PENDING (a different key) without
    // clearing the redirect stash.
    let (_, _, _, loc2) = c.send(Method::GET, "/auth/github", None).await;
    let state = extract_state(&loc2);

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
            "email": "octo@example.com",
        })))
        .mount(&app.github)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/users/upsert-by-identity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": live_user_body(uid, "octo@example.com", "octocat"),
            "user_created": true,
            "identity_created": true,
        })))
        .mount(&app.profile)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/users/{uid}/grants/claim")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"claimed": 0})))
        .mount(&app.profile)
        .await;
    // Grant oauth_login so the callback gate admits the user.
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/flags")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"oauth_login": true})))
        .mount(&app.profile)
        .await;

    let (s, _, _, location) = c
        .send(
            Method::GET,
            &format!("/auth/github/callback?code=fake&state={state}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert_eq!(
        location, "/s/owner-handle/photos",
        "callback should resume the stashed share URL"
    );
    app.cleanup().await;
}

#[tokio::test]
async fn share_landing_grantee_minted_jwt_redirect() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let caller_uid = fake_user_id();
    happy_login(&app, &mut c, caller_uid, "alice@x.com").await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{caller_uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            caller_uid,
            "alice@x.com",
            "alice",
        )))
        .mount(&app.profile)
        .await;

    let owner_uid = Uuid::new_v4();
    Mock::given(method("GET"))
        .and(path("/v1/users/by-username"))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            owner_uid,
            "owner@x.com",
            "owner-handle",
        )))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/users/{owner_uid}/workspaces/photos/access"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "editor"})))
        .mount(&app.profile)
        .await;

    let (s, _, _, location) = c.send(Method::GET, "/s/owner-handle/photos", None).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert!(
        location.starts_with("https://owner-handle.workspace.chan.app/photos/?t="),
        "got {location}"
    );
    app.cleanup().await;
}

#[tokio::test]
async fn share_landing_no_access_is_404() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let caller_uid = fake_user_id();
    happy_login(&app, &mut c, caller_uid, "alice@x.com").await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{caller_uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            caller_uid,
            "alice@x.com",
            "alice",
        )))
        .mount(&app.profile)
        .await;
    let owner_uid = Uuid::new_v4();
    Mock::given(method("GET"))
        .and(path("/v1/users/by-username"))
        .respond_with(ResponseTemplate::new(200).set_body_json(live_user_body(
            owner_uid,
            "owner@x.com",
            "owner-handle",
        )))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/users/{owner_uid}/workspaces/photos/access"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "not found"})))
        .mount(&app.profile)
        .await;

    let (s, _, _, _) = c.send(Method::GET, "/s/owner-handle/photos", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

// ---------------------------------------------------------------------------
// Feature-flag gate at OAuth callback
// ---------------------------------------------------------------------------

#[tokio::test]
async fn callback_denied_when_oauth_login_flag_off() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();

    // Walk the OAuth flow manually so we can install a flags mock
    // that returns oauth_login=false (default-off shape).
    let (_, _, _, location) = c.send(Method::GET, "/auth/github", None).await;
    let state = extract_state(&location);

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
            "email": "octo@example.com",
        })))
        .mount(&app.github)
        .await;
    let now = chrono::Utc::now().to_rfc3339();
    let user_body = json!({
        "id": uid,
        "email": "octo@example.com",
        "display_name": "Octo Cat",
        "username": format!("u{}", &uid.simple().to_string()[..12]),
        "username_edits": 0,
        "created_at": now,
        "updated_at": now,
    });
    Mock::given(method("POST"))
        .and(path("/v1/users/upsert-by-identity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user": user_body,
            "user_created": true,
            "identity_created": true,
        })))
        .mount(&app.profile)
        .await;
    // Flags mock: oauth_login disabled.
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/flags")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"oauth_login": false})))
        .mount(&app.profile)
        .await;

    let (s, _, _, location) = c
        .send(
            Method::GET,
            &format!("/auth/github/callback?code=fake&state={state}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert_eq!(location, "/?denied=oauth_login");

    // No session was granted: /api/me returns 401.
    let (s, _, _, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn me_includes_flags_map() {
    let app = TestApp::new().await;
    let mut c = Client::new(&app);
    let uid = fake_user_id();
    happy_login(&app, &mut c, uid, "octo@example.com").await;

    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": uid,
            "email": "octo@example.com",
            "display_name": "Octo Cat",
            "username": format!("u{}", &uid.simple().to_string()[..12]),
            "username_edits": 0,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })))
        .mount(&app.profile)
        .await;

    let (s, _, body, _) = c.send(Method::GET, "/api/me", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["flags"]["oauth_login"], true);
    assert_eq!(body["flags"]["share_workspaces"], true);
    app.cleanup().await;
}
