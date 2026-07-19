//! Integration tests for `POST /desktop/v1/devserver/entry`.
//!
//! Each test gets its own throwaway Postgres schema (for api_tokens +
//! tower-sessions) and a wiremock server standing in for both
//! profile-service and the devserver-proxy admin API. Exercises the
//! 404 reason body (`no_devserver` / `devserver_offline` /
//! `access_denied`), its best-effort degrade when the profile lookup
//! fails, and the happy path.

#[path = "../../../tests-shared/pg_reaper.rs"]
mod pg_reaper;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::postgres::{PgPool, PgPoolOptions};
use tower::ServiceExt;
use tower_sessions_sqlx_store::PostgresStore;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use gateway_common::devserver_gate;
use identity::api_tokens::{ApiTokenService, NewToken, RequestMeta, TokenOrigin};
use identity::config::Config;
use identity::http;
use identity::profile_client::ProfileClient;
use identity::providers::github::GitHubProvider;
use identity::token_throttle::TokenThrottle;
use identity::workspace_admin_client::WorkspaceAdminClient;

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
    pool: PgPool,
    profile: MockServer,
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

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrate identity tables");

        let store = PostgresStore::new(pool.clone());
        store.migrate().await.expect("migrate sessions");

        let profile = MockServer::start().await;

        let profile_client = ProfileClient::new(profile.uri().parse().unwrap(), "unused".into())
            .expect("profile client");
        // No OAuth in these tests; Config just requires a non-empty
        // provider list.
        let provider = GitHubProvider::new("client".into(), "secret".into()).expect("gh");

        let api_tokens = ApiTokenService::new(pool.clone());
        let cfg = Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url: "http://localhost:7000/".parse().unwrap(),
            devserver_proxy_origin: "https://proxy.example.test".parse().unwrap(),
            devserver_tunnel_origin: "https://tunnel.example.test".parse().unwrap(),
            database_url: url.clone(),
            cookie_secure: false,
            profile_client,
            internal_auth_token: "test-internal".to_string(),
            identity_admin_token: String::new(),
            devserver_wildcard_suffix: ".devserver.chan.app".to_string(),
            workspace_public_scheme: "https".to_string(),
            workspace_public_port: String::new(),
            // Same mock server backs the proxy-admin client; its
            // /admin/v1/* paths don't collide with profile's /v1/*.
            workspace_admin: Some(
                WorkspaceAdminClient::new(profile.uri().parse().unwrap(), "test-admin".into())
                    .unwrap(),
            ),
            workspace_gate_secret: "test-workspace-gate-secret-32-bytes-aa".to_string(),
            providers: vec![Arc::new(provider)],
        });
        let router = http::router(cfg, store, api_tokens.clone(), TokenThrottle::new());

        Self {
            router,
            api_tokens,
            schema,
            admin_url: url,
            pool,
            profile,
        }
    }

    async fn cleanup(self) {
        self.pool.close().await;
        let admin = admin_pool(&self.admin_url).await;
        let _ = sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .execute(&admin)
            .await;
        admin.close().await;
    }

    /// Insert a user row directly (FK target for PAT create). Mirrors
    /// the profile backfill shape for the placeholder username.
    async fn insert_user(&self) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, email, username) VALUES \
             ($1, $2, 'u' || substr(replace($1::text, '-', ''), 1, 12))",
        )
        .bind(id)
        .bind(format!("{id}@example.com"))
        .execute(&self.pool)
        .await
        .expect("insert user");
        id
    }

    /// Mint a PAT carrying `desktop.connect`, as the legacy desktop
    /// authorize flow would.
    async fn desktop_pat(&self, uid: Uuid) -> String {
        self.pat_with_scopes(uid, &["desktop.connect"]).await
    }

    /// Mint a PAT with `scopes` (`["desktop.account"]` is what the
    /// account-mode authorize flow issues).
    async fn pat_with_scopes(&self, uid: Uuid, scopes: &[&str]) -> String {
        let scopes: Vec<String> = scopes.iter().map(|s| (*s).to_string()).collect();
        self.api_tokens
            .create(
                NewToken {
                    user_id: uid,
                    label: "desktop",
                    expires_at: None,
                    scopes: &scopes,
                    origin: TokenOrigin::Desktop,
                },
                &RequestMeta::default(),
            )
            .await
            .expect("create pat")
            .secret
    }
}

/// The placeholder username `insert_user` seeds, computed the same way.
fn placeholder_username(id: Uuid) -> String {
    format!("u{}", &id.simple().to_string()[..12])
}

async fn mock_tunnels(app: &TestApp, username: &str, devserver_ids: &[&str]) {
    let now = chrono::Utc::now().to_rfc3339();
    let rows: Vec<Value> = devserver_ids
        .iter()
        .map(|id| {
            json!({
                "user": username,
                "devserver_id": id,
                "peer_addr": null,
                "connected_at": now,
            })
        })
        .collect();
    Mock::given(method("GET"))
        .and(path(format!("/admin/v1/users/{username}/tunnels")))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows))
        .mount(&app.profile)
        .await;
}

async fn post_entry(app: &TestApp, pat: &str) -> (StatusCode, Value) {
    post_entry_body(app, pat, json!({})).await
}

async fn post_entry_body(app: &TestApp, pat: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/desktop/v1/devserver/entry")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {pat}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, v)
}

/// The disc wildcard host identity mints for `(username, dsid)`.
fn disc_host(username: &str, dsid: &str) -> String {
    format!("{username}--{}.devserver.chan.app", &dsid[..12])
}

#[tokio::test]
async fn entry_404_reason_no_devserver() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    mock_tunnels(&app, &username, &[]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/grants/owned")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
    assert_eq!(body["reason"], "no_devserver");
    assert_eq!(body["username"], username);
    assert!(body.get("label").is_none(), "got {body}");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_404_reason_devserver_offline_with_label() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    mock_tunnels(&app, &username, &[]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/grants/owned")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"devserver_id": "a".repeat(64), "label": "office-box", "grant_count": 0},
            {"devserver_id": "b".repeat(64), "label": "second-box", "grant_count": 1},
        ])))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
    assert_eq!(body["reason"], "devserver_offline");
    assert_eq!(body["username"], username);
    assert_eq!(body["label"], "office-box", "first row's label");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_404_reason_access_denied() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    let dsid = "c".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    // profile answers 404 on the access check: the ProfileClient maps
    // it to None, the handler to the access_denied reason.
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "not found"})))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
    assert_eq!(body["reason"], "access_denied");
    assert_eq!(body["username"], username);
    assert!(body.get("label").is_none(), "got {body}");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_404_degrades_to_plain_body_on_profile_error() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    mock_tunnels(&app, &username, &[]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/grants/owned")))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "narration is best-effort");
    assert_eq!(body["error"], "not found");
    assert!(body.get("reason").is_none(), "got {body}");
    app.cleanup().await;
}

/// Mock `GET /v1/users/{uid}` with a live profile row.
async fn mock_get_user(app: &TestApp, uid: Uuid, display_name: Option<&str>, email: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": uid,
            "email": email,
            "display_name": display_name,
            "username": placeholder_username(uid),
            "username_edits": 0,
            "created_at": now,
            "updated_at": now,
        })))
        .mount(&app.profile)
        .await;
}

/// Decode the `?t=` entry token off a minted entry_url. The aud is
/// the disc host the mint targeted.
fn decode_entry_url(entry_url: &str, username: &str, dsid: &str) -> devserver_gate::Claims {
    let token = entry_url.split("?t=").nth(1).expect("t= in entry_url");
    devserver_gate::decode(
        b"test-workspace-gate-secret-32-bytes-aa",
        token,
        devserver_gate::TokenType::Entry,
        &disc_host(username, dsid),
        dsid,
    )
    .expect("entry token decodes")
}

#[tokio::test]
async fn entry_token_carries_caller_identity() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    let dsid = "e".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;
    mock_get_user(&app, uid, Some("Alice Doe"), "alice@example.com").await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    let claims = decode_entry_url(body["entry_url"].as_str().unwrap(), &username, &dsid);
    assert_eq!(claims.name.as_deref(), Some("Alice Doe"));
    assert_eq!(claims.email.as_deref(), Some("alice@example.com"));
    app.cleanup().await;
}

#[tokio::test]
async fn entry_mints_without_identity_on_profile_error() {
    // Identity is cosmetic: a profile failure on the get_user lookup
    // must not fail the entry mint.
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    let dsid = "f".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}")))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    let claims = decode_entry_url(body["entry_url"].as_str().unwrap(), &username, &dsid);
    assert_eq!(claims.name, None);
    assert_eq!(claims.email, None);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_mints_url_with_live_tunnel_and_access() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    let dsid = "d".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["username"], username);
    assert_eq!(body["devserver_id"], dsid);
    let origin = format!("https://{}", disc_host(&username, &dsid));
    assert_eq!(body["proxy_origin"], origin);
    let entry_url = body["entry_url"].as_str().expect("entry_url");
    assert!(
        entry_url.starts_with(&format!("{origin}/?t=")),
        "got {entry_url}"
    );
    app.cleanup().await;
}

/// Mock `GET /v1/users/by-username` resolving to a fixed user row.
async fn mock_user_by_username(app: &TestApp, uid: Uuid, username: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    Mock::given(method("GET"))
        .and(path("/v1/users/by-username"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": uid,
            "email": format!("{username}@example.com"),
            "display_name": null,
            "username": username,
            "username_edits": 0,
            "created_at": now,
            "updated_at": now,
        })))
        .mount(&app.profile)
        .await;
}

#[tokio::test]
async fn entry_explicit_target_shared_devserver_mints_owner_disc_host() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let pat = app.desktop_pat(uid).await;

    // The devserver belongs to another user who shared it with the
    // caller; the entry body carries the recorded selection.
    let owner_uid = Uuid::new_v4();
    let owner = "owner-handle";
    let dsid = "1".repeat(64);
    mock_user_by_username(&app, owner_uid, owner).await;
    mock_tunnels(&app, owner, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/users/{owner_uid}/devservers/{dsid}/access"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "editor"})))
        .mount(&app.profile)
        .await;

    let (s, body) =
        post_entry_body(&app, &pat, json!({"owner": owner, "devserver_id": dsid})).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["username"], owner, "response names the OWNER");
    assert_eq!(body["devserver_id"], dsid);
    let origin = format!("https://{}", disc_host(owner, &dsid));
    assert_eq!(body["proxy_origin"], origin);
    let claims = decode_entry_url(body["entry_url"].as_str().unwrap(), owner, &dsid);
    assert_eq!(claims.sub, uid, "sub is the caller, not the owner");
    assert_eq!(claims.role, "editor");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_explicit_target_not_live_is_devserver_offline() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    // One devserver live, but the recorded selection names another.
    mock_tunnels(&app, &username, &[&"a".repeat(64)]).await;
    let (s, body) = post_entry_body(&app, &pat, json!({"devserver_id": "b".repeat(64)})).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["reason"], "devserver_offline");
    assert_eq!(body["username"], username);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_explicit_target_no_access_is_access_denied() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let pat = app.desktop_pat(uid).await;

    let owner_uid = Uuid::new_v4();
    let owner = "owner-handle";
    let dsid = "2".repeat(64);
    mock_user_by_username(&app, owner_uid, owner).await;
    mock_tunnels(&app, owner, &[&dsid]).await;
    // Grant revoked since the desktop recorded the selection.
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/users/{owner_uid}/devservers/{dsid}/access"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "not found"})))
        .mount(&app.profile)
        .await;

    let (s, body) =
        post_entry_body(&app, &pat, json!({"owner": owner, "devserver_id": dsid})).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["reason"], "access_denied");
    assert_eq!(body["username"], owner);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_two_live_no_selector_falls_back_to_first_accessible() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.desktop_pat(uid).await;

    // Two live devservers, no recorded selection (pre-disc desktop).
    // The first (sorted) accessible one wins deterministically; here
    // access on the first fails, so the second is picked.
    let ds1 = "3".repeat(64);
    let ds2 = "4".repeat(64);
    mock_tunnels(&app, &username, &[&ds1, &ds2]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{ds1}/access")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "not found"})))
        .mount(&app.profile)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{ds2}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["devserver_id"], ds2);
    assert_eq!(
        body["proxy_origin"],
        format!("https://{}", disc_host(&username, &ds2))
    );
    app.cleanup().await;
}

// ---------------------------------------------------------------
// Scope gate: desktop.connect OR desktop.account (Contract B)
// ---------------------------------------------------------------

#[tokio::test]
async fn entry_accepts_account_scope_for_owned_devserver() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.pat_with_scopes(uid, &["desktop.account"]).await;

    let dsid = "5".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;

    let (s, body) =
        post_entry_body(&app, &pat, json!({"owner": username, "devserver_id": dsid})).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["username"], username);
    assert_eq!(body["devserver_id"], dsid);
    let claims = decode_entry_url(body["entry_url"].as_str().unwrap(), &username, &dsid);
    assert_eq!(claims.role, "owner");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_accepts_account_scope_for_granted_devserver() {
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let pat = app.pat_with_scopes(uid, &["desktop.account"]).await;

    let owner_uid = Uuid::new_v4();
    let owner = "owner-handle";
    let dsid = "6".repeat(64);
    mock_user_by_username(&app, owner_uid, owner).await;
    mock_tunnels(&app, owner, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/users/{owner_uid}/devservers/{dsid}/access"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "editor"})))
        .mount(&app.profile)
        .await;

    let (s, body) =
        post_entry_body(&app, &pat, json!({"owner": owner, "devserver_id": dsid})).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["username"], owner);
    let claims = decode_entry_url(body["entry_url"].as_str().unwrap(), owner, &dsid);
    assert_eq!(claims.sub, uid, "sub is the caller, not the owner");
    assert_eq!(claims.role, "editor");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_still_accepts_legacy_connect_scope() {
    // Shipped desktops hold desktop.connect PATs; the widened gate
    // must keep accepting them unchanged.
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.pat_with_scopes(uid, &["desktop.connect"]).await;

    let dsid = "7".repeat(64);
    mock_tunnels(&app, &username, &[&dsid]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/devservers/{dsid}/access")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"role": "owner"})))
        .mount(&app.profile)
        .await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::OK, "got {body}");
    assert_eq!(body["devserver_id"], dsid);
    app.cleanup().await;
}

#[tokio::test]
async fn entry_rejects_tunnel_only_pat() {
    // The widened gate is connect-or-account, never tunnel: a
    // devserver's own dial-in credential cannot mint entry URLs.
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let pat = app.pat_with_scopes(uid, &["tunnel"]).await;

    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "got {body}");
    app.cleanup().await;
}

#[tokio::test]
async fn entry_404_reasons_unchanged_for_account_pat() {
    // The reason vocabulary is a desktop wire contract; the account
    // scope must not change it.
    let app = TestApp::new().await;
    let uid = app.insert_user().await;
    let username = placeholder_username(uid);
    let pat = app.pat_with_scopes(uid, &["desktop.account"]).await;

    // No devserver at all -> no_devserver.
    mock_tunnels(&app, &username, &[]).await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/users/{uid}/grants/owned")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&app.profile)
        .await;
    let (s, body) = post_entry(&app, &pat).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["reason"], "no_devserver");
    assert_eq!(body["username"], username);

    // Explicit target that is not live -> devserver_offline.
    let (s, body) = post_entry_body(&app, &pat, json!({"devserver_id": "8".repeat(64)})).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(body["reason"], "devserver_offline");
    app.cleanup().await;
}
