//! Integration tests for personal access tokens (PATs).
//!
//! Each test gets its own throwaway Postgres schema. Exercises the
//! `ApiTokenService` directly (create / validate / revoke / audit)
//! and the `/internal/v1/tokens/validate` endpoint over the live
//! router.

#[path = "../../../tests-shared/pg_reaper.rs"]
mod pg_reaper;

use std::sync::Arc;

/// Default scope set for tests that don't care about scope content.
/// Matches the production default in
/// `identity::api_tokens::DEFAULT_TOKEN_SCOPES`.
fn default_scopes() -> Vec<String> {
    vec!["tunnel".to_string()]
}

/// Audit context with both fields populated, for tests asserting the
/// recorded ip / user_agent. Use `RequestMeta::default()` when the
/// test doesn't care.
fn meta(ip: &str, ua: &str) -> RequestMeta {
    RequestMeta {
        ip: Some(ip.to_string()),
        user_agent: Some(ua.to_string()),
    }
}

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::postgres::{PgPool, PgPoolOptions};

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
use uuid::Uuid;

use identity::api_tokens::{NewToken, RequestMeta, TokenOrigin};
use identity::config::Config;
use identity::http;
use identity::profile_client::ProfileClient;
use identity::providers::github::GitHubProvider;
use identity::token_throttle::TokenThrottle;

struct TestEnv {
    router: Router,
    api_tokens: identity::api_tokens::ApiTokenService,
    schema: String,
    admin_url: String,
    pool: PgPool,
}

impl TestEnv {
    fn api_tokens_service(&self) -> &identity::api_tokens::ApiTokenService {
        &self.api_tokens
    }

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
            .expect("migrate");

        let store = PostgresStore::new(pool.clone());
        store.migrate().await.expect("migrate sessions");

        // Minimal Config; nothing in the PAT endpoints reads OAuth
        // provider state. We still need a provider configured because
        // Config requires non-empty `providers`.
        let provider = GitHubProvider::new("client".into(), "secret".into()).expect("gh");
        let profile_client =
            ProfileClient::new("http://127.0.0.1:65535/".parse().unwrap(), "unused".into())
                .expect("profile client");

        let api_tokens = identity::api_tokens::ApiTokenService::new(pool.clone());
        let api_tokens_for_state = api_tokens.clone();
        let cfg = Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url: "http://localhost:7000/".parse().unwrap(),
            database_url: url.clone(),
            cookie_secure: false,
            profile_client,
            internal_auth_token: "test-internal".to_string(),
            devserver_wildcard_suffix: ".devserver.chan.app".to_string(),
            workspace_public_scheme: "https".to_string(),
            workspace_public_port: String::new(),
            workspace_admin: None,
            workspace_gate_secret: "test-workspace-gate-secret-32-bytes-aa".to_string(),
            providers: vec![Arc::new(provider)],
        });
        let router = http::router(cfg, store, api_tokens_for_state, TokenThrottle::new());

        Self {
            router,
            api_tokens,
            schema,
            admin_url: url,
            pool,
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

    /// Insert a user row directly so PAT create has an FK target.
    async fn insert_user(&self) -> Uuid {
        let id = Uuid::new_v4();
        // username is NOT NULL since migration 0003. Mirror the
        // backfill shape so the row passes the unique index across
        // tests that insert multiple users.
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
}

async fn json_post(router: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, v)
}

async fn json_post_with_auth(
    router: &Router,
    uri: &str,
    bearer: &str,
    body: Value,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, v)
}

#[tokio::test]
async fn pat_create_validate_revoke_audit() {
    let env = TestEnv::new().await;
    let uid = env.insert_user().await;

    let created = env
        .api_tokens_service()
        .create(
            NewToken {
                user_id: uid,
                label: "ci-runner",
                expires_at: None,
                scopes: &default_scopes(),
                origin: TokenOrigin::Spa,
            },
            &meta("10.0.0.1", "test-ua"),
        )
        .await
        .expect("create");
    assert!(created.secret.starts_with("chan_pat_"));
    assert_eq!(created.token.label, "ci-runner");

    // Validate succeeds, returns user_id + username, bumps last_used.
    let v = env
        .api_tokens_service()
        .validate(&created.secret, &meta("10.0.0.2", "tunneld"))
        .await
        .expect("validate");
    assert_eq!(v.user_id, uid);
    assert_eq!(v.token_id, created.token.id);
    assert!(v.username.starts_with('u'));

    // Revoke kills the token; subsequent validate is unauthorized.
    let revoked = env
        .api_tokens_service()
        .revoke(uid, created.token.id, &meta("10.0.0.1", "test-ua"))
        .await
        .expect("revoke");
    assert!(revoked);
    assert!(env
        .api_tokens_service()
        .validate(&created.secret, &RequestMeta::default())
        .await
        .is_err());

    // Audit log records all three actions in reverse chronological
    // order: created -> used -> revoked.
    let entries = env
        .api_tokens_service()
        .audit(uid, created.token.id, 50)
        .await
        .expect("audit");
    let actions: Vec<_> = entries.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(actions, vec!["revoked", "used", "created"]);

    env.cleanup().await;
}

#[tokio::test]
async fn pat_validate_skips_blocked_user() {
    // Block-flag enforcement is the safety net on top of the
    // admin block path's auto-revoke: even an unrevoked token
    // stops working when the owner's row carries blocked_at.
    let env = TestEnv::new().await;
    let uid = env.insert_user().await;
    let created = env
        .api_tokens_service()
        .create(
            NewToken {
                user_id: uid,
                label: "ci",
                expires_at: None,
                scopes: &default_scopes(),
                origin: TokenOrigin::Spa,
            },
            &RequestMeta::default(),
        )
        .await
        .expect("create");

    // Active path works.
    env.api_tokens_service()
        .validate(&created.secret, &RequestMeta::default())
        .await
        .expect("validate ok");

    // Set blocked_at directly (the admin endpoint lives in
    // profile-service; here we exercise the SQL guard).
    sqlx::query("UPDATE users SET blocked_at = now() WHERE id = $1")
        .bind(uid)
        .execute(&env.pool)
        .await
        .unwrap();

    let res = env
        .api_tokens_service()
        .validate(&created.secret, &RequestMeta::default())
        .await;
    assert!(res.is_err(), "blocked-user validate should fail");
    env.cleanup().await;
}

#[tokio::test]
async fn pat_expired_is_unauthorized() {
    let env = TestEnv::new().await;
    let uid = env.insert_user().await;

    let past = chrono::Utc::now() - chrono::Duration::seconds(60);
    let created = env
        .api_tokens_service()
        .create(
            NewToken {
                user_id: uid,
                label: "stale",
                expires_at: Some(past),
                scopes: &default_scopes(),
                origin: TokenOrigin::Spa,
            },
            &RequestMeta::default(),
        )
        .await
        .expect("create");
    assert!(env
        .api_tokens_service()
        .validate(&created.secret, &RequestMeta::default())
        .await
        .is_err());
    env.cleanup().await;
}

#[tokio::test]
async fn pat_audit_scoped_to_owner() {
    let env = TestEnv::new().await;
    let alice = env.insert_user().await;
    let bob = env.insert_user().await;

    let alice_token = env
        .api_tokens_service()
        .create(
            NewToken {
                user_id: alice,
                label: "a",
                expires_at: None,
                scopes: &default_scopes(),
                origin: TokenOrigin::Spa,
            },
            &RequestMeta::default(),
        )
        .await
        .expect("create");

    // Bob asking for Alice's token audit must 404, not leak rows.
    let res = env
        .api_tokens_service()
        .audit(bob, alice_token.token.id, 50)
        .await;
    assert!(matches!(res, Err(identity::error::Error::NotFound)));

    // Bob revoking Alice's token returns false (no row matched);
    // Alice's token continues to validate.
    let revoked = env
        .api_tokens_service()
        .revoke(bob, alice_token.token.id, &RequestMeta::default())
        .await
        .expect("revoke call");
    assert!(!revoked);
    assert!(env
        .api_tokens_service()
        .validate(&alice_token.secret, &RequestMeta::default())
        .await
        .is_ok());

    env.cleanup().await;
}

#[tokio::test]
async fn pat_validate_endpoint_requires_internal_bearer() {
    let env = TestEnv::new().await;
    let uid = env.insert_user().await;

    let created = env
        .api_tokens_service()
        .create(
            NewToken {
                user_id: uid,
                label: "tunnel",
                expires_at: None,
                scopes: &default_scopes(),
                origin: TokenOrigin::Spa,
            },
            &RequestMeta::default(),
        )
        .await
        .expect("create");

    // Missing bearer is rejected.
    let (s, _) = json_post(
        &env.router,
        "/internal/v1/tokens/validate",
        json!({"token": created.secret}),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // Wrong bearer is rejected.
    let (s, _) = json_post_with_auth(
        &env.router,
        "/internal/v1/tokens/validate",
        "wrong",
        json!({"token": created.secret}),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // Correct bearer succeeds and returns user_id + username.
    let (s, v) = json_post_with_auth(
        &env.router,
        "/internal/v1/tokens/validate",
        "test-internal",
        json!({"token": created.secret}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["user_id"].as_str().unwrap(), uid.to_string());
    // W1: the response carries the devserver identity (lowercase hex
    // SHA-256 of the PAT). devserver-proxy keys the registry + drv on it.
    let ds = v["devserver_id"].as_str().expect("devserver_id present");
    assert_eq!(ds.len(), 64);
    assert!(ds.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')));

    // Garbage token gets unauthorized, not bad-request, so callers
    // can't probe shape.
    let (s, _) = json_post_with_auth(
        &env.router,
        "/internal/v1/tokens/validate",
        "test-internal",
        json!({"token": "not-a-pat"}),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    env.cleanup().await;
}
