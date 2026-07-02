//! End-to-end tests against a real Postgres.
//!
//! Set `TEST_DATABASE_URL` to a database the test process can create
//! schemas in. Each test gets its own throwaway schema, so tests are
//! independent and can run in parallel.

#[path = "../../../tests-shared/pg_reaper.rs"]
mod pg_reaper;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Single-connection admin pool. `PgPool::connect` defaults to
/// max_connections=10; multiplied by ~17 parallel tests (each
/// opening admin pools twice — once on setup, once on cleanup)
/// blows past a default Postgres `max_connections=100`. Capping
/// admin to one keeps per-test peak demand well under that cap.
async fn admin_pool(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(url)
        .await
        .expect("connect admin")
}
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "test-token";
const ADMIN_TOKEN: &str = "test-admin-token";

struct TestApp {
    router: Router,
    schema: String,
    admin_url: String,
    pool: PgPool,
}

impl TestApp {
    async fn new() -> Self {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set; e.g. postgres://localhost/chan_gateway_test");
        // Hold-one-connection reaper: clears any idle connections
        // leaked by previous test-process runs, then keeps one slot
        // pinned for the rest of this process so the role never
        // goes fully idle from PG's perspective.
        pg_reaper::reap_idle(&url).await;
        let schema = format!("t_{}", Uuid::new_v4().simple());

        // Create the schema using a one-shot admin connection so we
        // don't pay for it on every pool acquire.
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

        let router = profile::http::router(profile::http::AppState {
            pool: pool.clone(),
            auth_token: TOKEN.to_string(),
            admin_token: Some(ADMIN_TOKEN.to_string()),
            workspace_admin: None,
        });

        Self {
            router,
            schema,
            admin_url: url,
            pool,
        }
    }

    async fn cleanup(self) {
        // Close the per-test pool first so the admin connection
        // doesn't have to wait behind it.
        self.pool.close().await;
        let admin = admin_pool(&self.admin_url).await;
        let _ = sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .execute(&admin)
            .await;
        admin.close().await;
    }

    async fn req(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.req_as(TOKEN, method, path, body).await
    }

    async fn admin(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.req_as(ADMIN_TOKEN, method, path, body).await
    }

    async fn req_as(
        &self,
        token: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        let body = match body {
            Some(v) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&v).unwrap())
            }
            None => Body::empty(),
        };
        let req = builder.body(body).unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, json)
    }
}

#[tokio::test]
async fn auth_required() {
    let app = TestApp::new().await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"email":"a@b"}"#))
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn user_crud() {
    let app = TestApp::new().await;

    let (s, v) = app
        .req(
            Method::POST,
            "/v1/users",
            Some(json!({"email": "a@b.com", "display_name": "A"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let id = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["email"], "a@b.com");

    let (s, v) = app.req(Method::GET, &format!("/v1/users/{id}"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["display_name"], "A");

    let (s, v) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{id}"),
            Some(json!({"display_name": "B"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["display_name"], "B");

    let (s, _) = app
        .req(Method::DELETE, &format!("/v1/users/{id}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, _) = app.req(Method::GET, &format!("/v1/users/{id}"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn identity_link_and_lookup() {
    let app = TestApp::new().await;

    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "x@y.com"})))
        .await;
    let uid = u["id"].as_str().unwrap();

    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{uid}/identities"),
            Some(json!({
                "provider": "github",
                "provider_subject": "12345",
                "email": "x@y.com",
            })),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, v) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=github&subject=12345",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["id"].as_str().unwrap(), uid);

    // Re-linking the same provider+subject must conflict.
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{uid}/identities"),
            Some(json!({"provider": "github", "provider_subject": "12345"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // by-identity miss is 404, not 500.
    let (s, _) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=github&subject=nope",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn username_backfill_rename_and_cap() {
    let app = TestApp::new().await;

    // New users get the deterministic 'u<hex>' placeholder; edit
    // counter starts at 0 so callers have the full budget.
    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "n@m.com"})))
        .await;
    let uid = u["id"].as_str().unwrap().to_string();
    let initial = u["username"].as_str().unwrap().to_string();
    assert!(initial.starts_with('u'));
    assert_eq!(u["username_edits"].as_i64().unwrap(), 0);

    // Reject malformed handles before SQL ever sees them.
    let (s, _) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid}/username"),
            Some(json!({"username": "ab"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid}/username"),
            Some(json!({"username": "-leading"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Successful rename increments the counter.
    let (s, v) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid}/username"),
            Some(json!({"username": "alice"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["username"], "alice");
    assert_eq!(v["username_edits"].as_i64().unwrap(), 1);

    // Renaming to the same handle (case-insensitive) is a no-op,
    // not an edit -- caller can re-submit a form without burning a
    // slot.
    let (s, v) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid}/username"),
            Some(json!({"username": "ALICE"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["username_edits"].as_i64().unwrap(), 1);

    // Three more renames to exhaust the cap (4 total).
    for name in ["bob", "carol", "dave"] {
        let (s, _) = app
            .req(
                Method::PATCH,
                &format!("/v1/users/{uid}/username"),
                Some(json!({"username": name})),
            )
            .await;
        assert_eq!(s, StatusCode::OK);
    }

    // Cap reached: the next rename is rejected.
    let (s, _) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid}/username"),
            Some(json!({"username": "eve"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // Uniqueness enforced across users (case-insensitive).
    let (_, u2) = app
        .req(
            Method::POST,
            "/v1/users",
            Some(json!({"email": "n2@m.com"})),
        )
        .await;
    let uid2 = u2["id"].as_str().unwrap().to_string();
    let (s, _) = app
        .req(
            Method::PATCH,
            &format!("/v1/users/{uid2}/username"),
            Some(json!({"username": "Dave"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    app.cleanup().await;
}

/// Helper: insert an api_token row (skips the hashing layer; we
/// only care about state transitions in profile-side admin tests).
async fn insert_api_token(pool: &PgPool, user_id: Uuid, label: &str) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO api_tokens (user_id, label, token_hash) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user_id)
    .bind(label)
    .bind(format!("hash-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn admin_token_gating() {
    let app = TestApp::new().await;
    // Wrong bearer (the regular service token) is not enough for
    // /v1/admin: rotation independence requires this stays separate.
    let (s, _) = app.req(Method::GET, "/v1/admin/users", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    // Missing bearer entirely: same.
    let req = Request::builder()
        .method(Method::GET)
        .uri("/v1/admin/users")
        .body(Body::empty())
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Admin bearer works.
    let (s, _) = app.admin(Method::GET, "/v1/admin/users", None).await;
    assert_eq!(s, StatusCode::OK);
    app.cleanup().await;
}

#[tokio::test]
async fn admin_list_users_filters() {
    let app = TestApp::new().await;
    app.req(
        Method::POST,
        "/v1/users",
        Some(json!({"email": "alice@example.com"})),
    )
    .await;
    let (_, b) = app
        .req(
            Method::POST,
            "/v1/users",
            Some(json!({"email": "bob@other.com"})),
        )
        .await;
    let bob_id = b["id"].as_str().unwrap().to_string();

    // Substring filter (case-insensitive).
    let (s, v) = app
        .admin(Method::GET, "/v1/admin/users?email=EXAMPLE", None)
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["email"], "alice@example.com");

    // Block bob, then filter.
    app.admin(
        Method::POST,
        &format!("/v1/admin/users/{bob_id}/block"),
        Some(json!({"reason": "spam"})),
    )
    .await;
    let (_, only_blocked) = app
        .admin(Method::GET, "/v1/admin/users?blocked=true", None)
        .await;
    assert_eq!(only_blocked.as_array().unwrap().len(), 1);
    assert_eq!(only_blocked[0]["id"], bob_id);

    let (_, only_active) = app
        .admin(Method::GET, "/v1/admin/users?blocked=false", None)
        .await;
    assert_eq!(only_active.as_array().unwrap().len(), 1);
    assert_eq!(only_active[0]["email"], "alice@example.com");

    app.cleanup().await;
}

#[tokio::test]
async fn admin_block_revokes_tokens_and_audits() {
    let app = TestApp::new().await;
    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "z@z.com"})))
        .await;
    let uid: Uuid = u["id"].as_str().unwrap().parse().unwrap();
    let t1 = insert_api_token(&app.pool, uid, "t1").await;
    let t2 = insert_api_token(&app.pool, uid, "t2").await;

    // Pre-block: the regular GET still resolves the user, with
    // blocked_at null on the wire.
    let (_, before) = app
        .req(Method::GET, &format!("/v1/users/{uid}"), None)
        .await;
    assert!(before["blocked_at"].is_null());

    let (s, blocked) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/users/{uid}/block"),
            Some(json!({"reason": "abuse"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!blocked["blocked_at"].is_null());
    assert_eq!(blocked["block_reason"], "abuse");

    // Both tokens auto-revoked.
    let revoked: Vec<bool> = sqlx::query_scalar(
        "SELECT revoked_at IS NOT NULL FROM api_tokens \
         WHERE id IN ($1, $2) ORDER BY id",
    )
    .bind(t1)
    .bind(t2)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(revoked, vec![true, true]);

    // auth_audit row written with the reason.
    let (_, audit) = app
        .admin(
            Method::GET,
            &format!("/v1/admin/users/{uid}/auth-audit"),
            None,
        )
        .await;
    let arr = audit.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["action"], "blocked");
    assert_eq!(arr[0]["note"], "abuse");

    // Re-block keeps the original blocked_at (idempotent timestamp,
    // updated reason). Issue another block with a fresh reason.
    let original_blocked_at = blocked["blocked_at"].clone();
    let (s, reblocked) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/users/{uid}/block"),
            Some(json!({"reason": "still bad"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(reblocked["blocked_at"], original_blocked_at);
    assert_eq!(reblocked["block_reason"], "still bad");

    // Unblock clears flags but does NOT un-revoke tokens.
    let (s, unblocked) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/users/{uid}/unblock"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert!(unblocked["blocked_at"].is_null());
    assert!(unblocked["block_reason"].is_null());
    let still_revoked: bool = sqlx::query_scalar(
        "SELECT bool_and(revoked_at IS NOT NULL) FROM api_tokens WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(still_revoked);

    app.cleanup().await;
}

#[tokio::test]
async fn admin_token_revoke_and_audit() {
    let app = TestApp::new().await;
    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "k@k.com"})))
        .await;
    let uid: Uuid = u["id"].as_str().unwrap().parse().unwrap();
    let tid = insert_api_token(&app.pool, uid, "cli").await;

    // List by user.
    let (s, list) = app
        .admin(Method::GET, &format!("/v1/admin/users/{uid}/tokens"), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"], tid.to_string());
    assert!(list[0]["revoked_at"].is_null());

    // Revoke writes an audit row.
    let (s, _) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/tokens/{tid}/revoke"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, audit) = app
        .admin(Method::GET, &format!("/v1/admin/tokens/{tid}/audit"), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = audit.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["action"], "revoked");

    // Second revoke is a clean no-op (no audit duplicate).
    let (s, _) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/tokens/{tid}/revoke"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, audit2) = app
        .admin(Method::GET, &format!("/v1/admin/tokens/{tid}/audit"), None)
        .await;
    assert_eq!(audit2.as_array().unwrap().len(), 1);

    // Unknown token: 404.
    let (s, _) = app
        .admin(
            Method::POST,
            &format!("/v1/admin/tokens/{}/revoke", Uuid::new_v4()),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn write_auth_audit_round_trip() {
    let app = TestApp::new().await;
    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "a@a.com"})))
        .await;
    let uid = u["id"].as_str().unwrap().to_string();

    let (s, _) = app
        .req(
            Method::POST,
            "/v1/auth-audit",
            Some(json!({
                "user_id": uid,
                "action": "login",
                "ip": "10.0.0.1",
                "user_agent": "test/1.0",
                "note": "github",
            })),
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (_, audit) = app
        .admin(
            Method::GET,
            &format!("/v1/admin/users/{uid}/auth-audit"),
            None,
        )
        .await;
    let arr = audit.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["action"], "login");
    assert_eq!(arr[0]["ip"], "10.0.0.1");
    assert_eq!(arr[0]["note"], "github");

    // Empty action rejected.
    let (s, _) = app
        .req(
            Method::POST,
            "/v1/auth-audit",
            Some(json!({"user_id": uid, "action": ""})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Audit for unknown user is 404 (FK is enforced via the WHERE
    // EXISTS guard so we get a clean error, not a 500).
    let (s, _) = app
        .req(
            Method::POST,
            "/v1/auth-audit",
            Some(json!({"user_id": Uuid::new_v4(), "action": "login"})),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ---------------------------------------------------------------
// upsert_by_identity (single-tx find-or-create)
// ---------------------------------------------------------------

#[tokio::test]
async fn upsert_first_time_creates_user_and_identity() {
    let app = TestApp::new().await;
    let (s, v) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "1001",
                "email": "alice@example.com",
                "display_name": "Alice",
                "avatar_url": "https://gh/a.png",
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["user_created"], true);
    assert_eq!(v["identity_created"], true);
    assert_eq!(v["user"]["email"], "alice@example.com");
    assert_eq!(v["user"]["avatar_url"], "https://gh/a.png");

    // by-identity lookup now resolves to the same user.
    let (s, by_id) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=github&subject=1001",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(by_id["id"], v["user"]["id"]);
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_existing_identity_returns_same_user() {
    let app = TestApp::new().await;
    let (_, first) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "2002",
                "email": "bob@example.com",
            })),
        )
        .await;
    let uid = first["user"]["id"].clone();

    // Second call with the same (provider, subject) returns the
    // same user; no new identity row written.
    let (s, second) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "2002",
                "email": "bob@example.com",
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(second["user_created"], false);
    assert_eq!(second["identity_created"], false);
    assert_eq!(second["user"]["id"], uid);
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_existing_email_links_identity_to_existing_user() {
    // The migration 0001 contract: a second provider with a verified
    // email matching an existing user attaches to that user, not a
    // new one.
    let app = TestApp::new().await;
    let (_, gh) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "3003",
                "email": "carol@example.com",
            })),
        )
        .await;
    let uid = gh["user"]["id"].clone();

    let (s, google) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "google",
                "provider_subject": "g-3003",
                "email": "Carol@Example.COM",  // case-insensitive match
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(google["user_created"], false);
    assert_eq!(google["identity_created"], true);
    assert_eq!(google["user"]["id"], uid, "should link to existing user");

    // Both identities resolve to the same user.
    let (_, by_gh) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=github&subject=3003",
            None,
        )
        .await;
    let (_, by_google) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=google&subject=g-3003",
            None,
        )
        .await;
    assert_eq!(by_gh["id"], by_google["id"]);
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_missing_email_for_unknown_user_is_400() {
    let app = TestApp::new().await;
    let (s, _) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "4004",
                // no email
            })),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_existing_identity_without_email_still_succeeds() {
    // Re-auth for an already-linked identity must not require email
    // (provider may stop returning it after first consent).
    let app = TestApp::new().await;
    app.req(
        Method::POST,
        "/v1/users/upsert-by-identity",
        Some(json!({
            "provider": "github",
            "provider_subject": "5005",
            "email": "dan@example.com",
        })),
    )
    .await;
    let (s, v) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "5005",
                // no email
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["user_created"], false);
    assert_eq!(v["identity_created"], false);
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_refreshes_avatar_when_changed() {
    let app = TestApp::new().await;
    app.req(
        Method::POST,
        "/v1/users/upsert-by-identity",
        Some(json!({
            "provider": "github",
            "provider_subject": "6006",
            "email": "eve@example.com",
            "avatar_url": "https://gh/old.png",
        })),
    )
    .await;
    let (_, v) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "6006",
                "avatar_url": "https://gh/new.png",
            })),
        )
        .await;
    assert_eq!(v["user"]["avatar_url"], "https://gh/new.png");

    // Same avatar -> no spurious updated_at churn (we don't observe
    // updated_at directly; just confirm the field stayed).
    let (_, v2) = app
        .req(
            Method::POST,
            "/v1/users/upsert-by-identity",
            Some(json!({
                "provider": "github",
                "provider_subject": "6006",
                "avatar_url": "https://gh/new.png",
            })),
        )
        .await;
    assert_eq!(v2["user"]["avatar_url"], "https://gh/new.png");
    app.cleanup().await;
}

#[tokio::test]
async fn upsert_concurrent_first_time_no_orphans() {
    // Race regression: multiple concurrent first-time signups for
    // the same (provider, subject) must converge on a single user
    // row, not leave orphans behind. Workspaces N parallel upserts and
    // asserts they all return the same user id and no extra users
    // were created with that email.
    let app = TestApp::new().await;
    let router = std::sync::Arc::new(app.router.clone());
    let mut handles = Vec::new();
    for _ in 0..5 {
        let r = router.clone();
        handles.push(tokio::spawn(async move {
            let req = Request::builder()
                .method(Method::POST)
                .uri("/v1/users/upsert-by-identity")
                .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "provider": "github",
                        "provider_subject": "race-7007",
                        "email": "race@example.com",
                    }))
                    .unwrap(),
                ))
                .unwrap();
            let res = (*r).clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK, "concurrent upsert failed");
            let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            v["user"]["id"].as_str().unwrap().to_string()
        }));
    }

    let mut ids = Vec::new();
    for h in handles {
        ids.push(h.await.unwrap());
    }
    let first = &ids[0];
    assert!(
        ids.iter().all(|x| x == first),
        "all concurrent upserts must converge on one user id, got {ids:?}"
    );

    // Exactly one user row carries this email.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind("race@example.com")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "no orphans");
    app.cleanup().await;
}

#[tokio::test]
async fn cascade_on_user_delete() {
    let app = TestApp::new().await;

    let (_, u) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": "c@d.com"})))
        .await;
    let uid = u["id"].as_str().unwrap();

    app.req(
        Method::POST,
        &format!("/v1/users/{uid}/identities"),
        Some(json!({"provider": "google", "provider_subject": "g1"})),
    )
    .await;

    let (s, _) = app
        .req(Method::DELETE, &format!("/v1/users/{uid}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, _) = app
        .req(
            Method::GET,
            "/v1/users/by-identity?provider=google&subject=g1",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ---------------------------------------------------------------------------
// devserver_grants
// ---------------------------------------------------------------------------

async fn mk_user(app: &TestApp, email: &str) -> String {
    let (_, v) = app
        .req(Method::POST, "/v1/users", Some(json!({"email": email})))
        .await;
    v["id"].as_str().unwrap().to_string()
}

/// A syntactically-valid devserver id: 64 lowercase hex chars. The real
/// id is SHA-256(PAT); tests only need the right shape and distinct,
/// lexicographically-sortable values, so one repeated hex digit suffices.
fn ds(hex_digit: &str) -> String {
    hex_digit.repeat(64)
}

#[tokio::test]
async fn grant_create_resolves_existing_user() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let alice = mk_user(&app, "alice@x.com").await;
    let dsid = ds("a");

    let (s, v) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "alice@x.com", "role": "editor"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(v["grantee_user_id"], alice);
    assert_eq!(v["role"], "editor");
    assert_eq!(v["devserver_id"], dsid);
    assert!(v["accepted_at"].is_string());

    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_pending_for_unknown_email() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let dsid = ds("a");

    let (s, v) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "future@x.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert!(v["grantee_user_id"].is_null());
    assert!(v["accepted_at"].is_null());

    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_validates_inputs() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let dsid = ds("a");

    // Bad role.
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "a@b.com", "role": "admin"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Bad email.
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "nope", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Bad devserver id (non-hex char at the right length).
    let bad = "g".repeat(64);
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{bad}/grants"),
            Some(json!({"grantee_email": "a@b.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Bad devserver id (too short).
    let short = "a".repeat(63);
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{short}/grants"),
            Some(json!({"grantee_email": "a@b.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Unknown owner.
    let ghost = Uuid::new_v4();
    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{ghost}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "a@b.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_is_idempotent_and_promotes_role() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let _alice = mk_user(&app, "alice@x.com").await;
    let dsid = ds("a");

    let (s, v1) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "alice@x.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, v2) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "ALICE@x.com", "role": "editor"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(v1["id"], v2["id"], "same row");
    assert_eq!(v2["role"], "editor", "role promoted");
    assert_eq!(v1["created_at"], v2["created_at"], "created_at preserved");

    app.cleanup().await;
}

#[tokio::test]
async fn grant_list_and_delete() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    mk_user(&app, "alice@x.com").await;
    mk_user(&app, "bob@x.com").await;
    let dsid = ds("a");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "viewer"})),
    )
    .await;
    let (_, b) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "bob@x.com", "role": "editor"})),
        )
        .await;
    let bid = b["id"].as_str().unwrap().to_string();

    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 2);

    let (s, _) = app
        .req(
            Method::DELETE,
            &format!("/v1/users/{owner}/grants/{bid}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (_, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            None,
        )
        .await;
    assert_eq!(v.as_array().unwrap().len(), 1);

    // Wrong-owner delete is 404 (defense-in-depth).
    let other = mk_user(&app, "other@x.com").await;
    let (_, again) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "bob@x.com", "role": "viewer"})),
        )
        .await;
    let gid = again["id"].as_str().unwrap();
    let (s, _) = app
        .req(
            Method::DELETE,
            &format!("/v1/users/{other}/grants/{gid}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn devserver_access_owner_grantee_and_stranger() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let alice = mk_user(&app, "alice@x.com").await;
    let stranger = mk_user(&app, "s@x.com").await;
    let dsid = ds("a");
    let other_ds = ds("b");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "editor"})),
    )
    .await;

    // Owner.
    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/access?as={owner}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["role"], "owner");

    // Grantee.
    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/access?as={alice}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["role"], "editor");

    // Stranger: 404, not 403 (no enumeration).
    let (s, _) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/access?as={stranger}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Different devserver of the same owner: grantee has no grant on it,
    // so 404 (same shape). Proves the grant is scoped to its devserver.
    let (s, _) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{other_ds}/access?as={alice}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn claim_sweep_fills_pending_grants() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let dsid = ds("a");

    // Pre-seed a grant for an email that doesn't exist yet.
    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
        Some(json!({"grantee_email": "late@x.com", "role": "viewer"})),
    )
    .await;

    // Now the user signs up.
    let late = mk_user(&app, "late@x.com").await;

    // Sweep with the user's verified emails.
    let (s, v) = app
        .req(
            Method::POST,
            &format!("/v1/users/{late}/grants/claim"),
            Some(json!({"emails": ["LATE@x.com", "alt@x.com"]})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["claimed"], 1);

    // Access now resolves.
    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{dsid}/access?as={late}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["role"], "viewer");

    // Second sweep is a no-op.
    let (_, v) = app
        .req(
            Method::POST,
            &format!("/v1/users/{late}/grants/claim"),
            Some(json!({"emails": ["late@x.com"]})),
        )
        .await;
    assert_eq!(v["claimed"], 0);

    app.cleanup().await;
}

#[tokio::test]
async fn list_owned_and_incoming() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let alice = mk_user(&app, "alice@x.com").await;
    let ds_a = ds("a");
    let ds_b = ds("b");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{ds_a}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "viewer"})),
    )
    .await;
    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{ds_a}/grants"),
        Some(json!({"grantee_email": "ghost@x.com", "role": "viewer"})),
    )
    .await;
    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{ds_b}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "editor"})),
    )
    .await;

    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/grants/owned"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // Ordered by devserver_id, so ds_a ("aaaa...") precedes ds_b.
    assert_eq!(arr[0]["devserver_id"], ds_a);
    assert_eq!(arr[0]["grant_count"], 2);
    assert_eq!(arr[1]["devserver_id"], ds_b);
    assert_eq!(arr[1]["grant_count"], 1);

    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{alice}/grants/incoming"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // Both rows show this owner and carry a devserver_id + label.
    for row in arr {
        assert_eq!(row["owner_user_id"], owner);
        assert!(row["devserver_id"].is_string());
        assert!(row["label"].is_string());
        assert!(row["accepted_at"].is_string());
    }

    app.cleanup().await;
}

#[tokio::test]
async fn cascade_grants_on_user_delete() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let alice = mk_user(&app, "alice@x.com").await;
    let dsid = ds("a");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "editor"})),
    )
    .await;

    // Delete the owner; grant should vanish.
    app.req(Method::DELETE, &format!("/v1/users/{owner}"), None)
        .await;
    let (_, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{alice}/grants/incoming"),
            None,
        )
        .await;
    assert_eq!(v.as_array().unwrap().len(), 0);

    app.cleanup().await;
}

// ---------------------------------------------------------------------------
// devservers (first-class entity)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn devserver_create_idempotent() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let dsid = ds("a");
    let upper = "A".repeat(64); // same id, uppercase: normalizes to dsid

    let (s, v1) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers"),
            Some(json!({"devserver_id": dsid, "label": "laptop"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(v1["devserver_id"], dsid);
    assert_eq!(v1["label"], "laptop");

    let (s, v2) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers"),
            Some(json!({"devserver_id": upper})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "re-create returns 200, not 201");
    assert_eq!(v1["id"], v2["id"], "same row");
    assert_eq!(
        v2["label"], "laptop",
        "blank label leaves the name untouched"
    );

    app.cleanup().await;
}

#[tokio::test]
async fn devserver_create_validates_id() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;

    let too_short = "a".repeat(63);
    let too_long = "a".repeat(65);
    let non_hex = "g".repeat(64);
    for bad in ["", "Has Space", &too_short, &too_long, &non_hex] {
        let (s, _) = app
            .req(
                Method::POST,
                &format!("/v1/users/{owner}/devservers"),
                Some(json!({"devserver_id": bad})),
            )
            .await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "should reject {bad:?}");
    }
    app.cleanup().await;
}

#[tokio::test]
async fn devserver_list_and_delete_cascades_grants() {
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    mk_user(&app, "alice@x.com").await;
    let ds_a = ds("a");
    let ds_b = ds("b");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers"),
        Some(json!({"devserver_id": ds_a})),
    )
    .await;
    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers/{ds_a}/grants"),
        Some(json!({"grantee_email": "alice@x.com", "role": "viewer"})),
    )
    .await;
    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers"),
        Some(json!({"devserver_id": ds_b})),
    )
    .await;

    let (s, v) = app
        .req(Method::GET, &format!("/v1/users/{owner}/devservers"), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["devserver_id"], ds_a);
    assert_eq!(arr[1]["devserver_id"], ds_b);

    let (s, _) = app
        .req(
            Method::DELETE,
            &format!("/v1/users/{owner}/devservers/{ds_a}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    // List shrinks.
    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{owner}/devservers"), None)
        .await;
    assert_eq!(v.as_array().unwrap().len(), 1);

    // Cascade dropped the grant. Listing grants on the deleted devserver
    // returns an empty array (the route is valid; just no rows).
    let (_, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/devservers/{ds_a}/grants"),
            None,
        )
        .await;
    assert_eq!(v.as_array().unwrap().len(), 0);

    app.cleanup().await;
}

#[tokio::test]
async fn owned_includes_grantless() {
    // A brand-new devserver with zero grants must still appear in
    // /grants/owned with grant_count = 0 so the dashboard can render it.
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    let dsid = ds("a");

    app.req(
        Method::POST,
        &format!("/v1/users/{owner}/devservers"),
        Some(json!({"devserver_id": dsid, "label": "desktop"})),
    )
    .await;

    let (s, v) = app
        .req(
            Method::GET,
            &format!("/v1/users/{owner}/grants/owned"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["devserver_id"], dsid);
    assert_eq!(arr[0]["label"], "desktop");
    assert_eq!(arr[0]["grant_count"], 0);

    app.cleanup().await;
}

#[tokio::test]
async fn grant_create_autocreates_devserver() {
    // POST /v1/users/{o}/devservers/{d}/grants must work even if the
    // owner has not POSTed the devserver first: the grant handler
    // upserts the devservers row in the same transaction, so a caller
    // that pre-seeds a grant before the devserver registers still
    // produces a valid graph and the FK never fires.
    let app = TestApp::new().await;
    let owner = mk_user(&app, "owner@x.com").await;
    mk_user(&app, "alice@x.com").await;
    let dsid = ds("c");

    let (s, _) = app
        .req(
            Method::POST,
            &format!("/v1/users/{owner}/devservers/{dsid}/grants"),
            Some(json!({"grantee_email": "alice@x.com", "role": "viewer"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);

    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{owner}/devservers"), None)
        .await;
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|r| r["devserver_id"] == dsid));

    app.cleanup().await;
}

// ---------------------------------------------------------------------------
// feature_flags
// ---------------------------------------------------------------------------

#[tokio::test]
async fn flags_seeded_oauth_login_and_share_workspaces() {
    let app = TestApp::new().await;
    let (s, v) = app.admin(Method::GET, "/v1/admin/flags", None).await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    let keys: Vec<&str> = arr.iter().map(|r| r["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"oauth_login"));
    assert!(keys.contains(&"share_workspaces"));
    for r in arr {
        if r["key"] == "oauth_login" || r["key"] == "share_workspaces" {
            assert_eq!(r["default_enabled"], false);
        }
    }
    app.cleanup().await;
}

#[tokio::test]
async fn flag_upsert_then_delete() {
    let app = TestApp::new().await;
    let (s, v) = app
        .admin(
            Method::POST,
            "/v1/admin/flags",
            Some(json!({
                "key": "experimental",
                "description": "internal canary",
                "default_enabled": true,
            })),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["default_enabled"], true);
    // Re-upsert flips default.
    let (_, v) = app
        .admin(
            Method::POST,
            "/v1/admin/flags",
            Some(json!({
                "key": "experimental",
                "default_enabled": false,
            })),
        )
        .await;
    assert_eq!(v["default_enabled"], false);
    assert_eq!(
        v["description"], "internal canary",
        "preserved on partial upsert"
    );

    let (s, _) = app
        .admin(Method::DELETE, "/v1/admin/flags/experimental", None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = app
        .admin(Method::DELETE, "/v1/admin/flags/experimental", None)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn flag_override_resolution() {
    let app = TestApp::new().await;
    let alice = mk_user(&app, "alice@x.com").await;
    let bob = mk_user(&app, "bob@x.com").await;

    // oauth_login is seeded default=false. Resolve for alice -> false.
    let (s, v) = app
        .req(Method::GET, &format!("/v1/users/{alice}/flags"), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["oauth_login"], false);

    // Grant oauth_login to alice via override.
    let (s, _) = app
        .admin(
            Method::POST,
            "/v1/admin/flags/oauth_login/overrides",
            Some(json!({"user_id": alice, "enabled": true})),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{alice}/flags"), None)
        .await;
    assert_eq!(v["oauth_login"], true);

    // Bob still defaults to false.
    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{bob}/flags"), None)
        .await;
    assert_eq!(v["oauth_login"], false);

    // Revoke alice's override; resolution falls back to default.
    let (s, _) = app
        .admin(
            Method::DELETE,
            &format!("/v1/admin/flags/oauth_login/overrides/{alice}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{alice}/flags"), None)
        .await;
    assert_eq!(v["oauth_login"], false);
    app.cleanup().await;
}

#[tokio::test]
async fn flag_override_idempotent_and_lists() {
    let app = TestApp::new().await;
    let alice = mk_user(&app, "alice@x.com").await;

    app.admin(
        Method::POST,
        "/v1/admin/flags/share_workspaces/overrides",
        Some(json!({"user_id": alice, "enabled": true})),
    )
    .await;
    app.admin(
        Method::POST,
        "/v1/admin/flags/share_workspaces/overrides",
        Some(json!({"user_id": alice, "enabled": false})),
    )
    .await;
    let (s, v) = app
        .admin(
            Method::GET,
            "/v1/admin/flags/share_workspaces/overrides",
            None,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1, "upsert collapses to one row");
    assert_eq!(arr[0]["enabled"], false);
    app.cleanup().await;
}

#[tokio::test]
async fn flag_override_on_unknown_flag_or_user_is_404() {
    let app = TestApp::new().await;
    let alice = mk_user(&app, "alice@x.com").await;
    let ghost = Uuid::new_v4();

    let (s, _) = app
        .admin(
            Method::POST,
            "/v1/admin/flags/nope/overrides",
            Some(json!({"user_id": alice, "enabled": true})),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, _) = app
        .admin(
            Method::POST,
            "/v1/admin/flags/oauth_login/overrides",
            Some(json!({"user_id": ghost, "enabled": true})),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn flag_delete_cascades_overrides() {
    let app = TestApp::new().await;
    let alice = mk_user(&app, "alice@x.com").await;
    // Make a throwaway flag we can delete (seeded ones stick around).
    app.admin(
        Method::POST,
        "/v1/admin/flags",
        Some(json!({"key": "tmp", "default_enabled": false})),
    )
    .await;
    app.admin(
        Method::POST,
        "/v1/admin/flags/tmp/overrides",
        Some(json!({"user_id": alice, "enabled": true})),
    )
    .await;
    app.admin(Method::DELETE, "/v1/admin/flags/tmp", None).await;
    let (_, v) = app
        .req(Method::GET, &format!("/v1/users/{alice}/flags"), None)
        .await;
    assert!(v.get("tmp").is_none(), "tmp flag is gone");
    app.cleanup().await;
}
