use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use gateway_common::validators::{valid_username, MAX_USERNAME_EDITS};
use oauth2::PkceCodeVerifier;
use rustrict::CensorStr;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tower_http::trace::TraceLayer;
use tower_sessions::{cookie::time::Duration, Expiry, Session, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use uuid::Uuid;

use crate::api_tokens::{
    ApiToken, ApiTokenService, AuditEntry, CreatedToken, NewToken, RequestMeta, TokenOrigin,
    ValidatedToken, DEFAULT_TOKEN_SCOPES,
};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::profile_client::{
    IncomingShare, OwnedWorkspaceSummary, User, Workspace, WorkspaceGrant,
};
use crate::static_files;
use crate::token_throttle::TokenThrottle;

const SESSION_COOKIE: &str = "id_session";
const KEY_USER: &str = "user_id";
const KEY_PENDING: &str = "pending_oauth";
/// Optional post-login redirect target. Set by the share landing
/// when an unauthenticated user lands on `/s/:owner/:workspace` so the
/// OAuth callback can resume the flow instead of dropping the user
/// at the dashboard. Stored as a relative path; the callback
/// validates the prefix before using it.
const KEY_POST_LOGIN_REDIRECT: &str = "post_login_redirect";

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub api_tokens: ApiTokenService,
    /// Per-token-fingerprint rate limiter applied to
    /// /internal/v1/tokens/validate. Defense in depth: workspace-proxy
    /// throttles by the same fingerprint one hop earlier, so this
    /// kicks in only if the internal bearer leaks and someone calls
    /// identity directly. Throttled requests come back as 401 so
    /// they are indistinguishable from "unknown token" on the wire.
    pub token_throttle: TokenThrottle,
}

/// Reserved usernames. Anything that could collide with an existing
/// or future top-level path under chan.app/ goes here. Kept short on
/// purpose; profanity / leet-speak is handled separately by the
/// rustrict pass.
const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "administrator",
    "api",
    "app",
    "auth",
    "billing",
    "blog",
    "chan",
    "dashboard",
    "developer",
    "developers",
    "docs",
    "workspace",
    "workspaces",
    "help",
    "id",
    "identity",
    "internal",
    "login",
    "logout",
    "me",
    "oauth",
    "owner",
    "profile",
    "public",
    "root",
    "settings",
    "signin",
    "signup",
    "staff",
    "static",
    "status",
    "support",
    "system",
    "team",
    "user",
    "users",
    "www",
];

#[derive(Debug, Deserialize, Serialize)]
struct PendingOauth {
    provider: String,
    state: String,
    verifier: String,
}

pub fn router(
    cfg: Arc<Config>,
    store: PostgresStore,
    api_tokens: ApiTokenService,
    token_throttle: TokenThrottle,
) -> Router {
    // Host-only on id.chan.app: no Domain attribute, so the cookie
    // does not propagate to workspace.chan.app or its subdomains. The
    // workspace-gate handoff covers the cross-service auth need; see
    // crates/identity/design.md.
    let session_layer = SessionManagerLayer::new(store)
        .with_name(SESSION_COOKIE)
        .with_secure(cfg.cookie_secure)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(30)));

    let state = AppState {
        cfg,
        api_tokens,
        token_throttle,
    };

    // /internal/* is gated by IDENTITY_INTERNAL_TOKEN (distinct from
    // PROFILE_AUTH_TOKEN; see internal_auth). Kept on its own
    // sub-router so the session layer doesn't try to load a cookie
    // session for callers that don't have one.
    //
    // No per-IP rate limit here. The only caller is workspace-proxy,
    // so a governor at this hop sees one peer IP regardless of how
    // many distinct clients are probing tokens upstream: a single
    // global bucket that can lock out legitimate `chan serve`
    // handshakes while leaving real attacker shape invisible. The
    // primary PAT brute-force gate sits in workspace-proxy, keyed on
    // a hash of the candidate token; `token_throttle` inside the
    // validate handler is its defense-in-depth twin.
    let internal = Router::new()
        .route("/internal/v1/tokens/validate", post(validate_token))
        .route_layer(middleware::from_fn_with_state(state.clone(), internal_auth));

    Router::new()
        .route("/healthz", get(healthz))
        .route("/auth/:provider", get(auth_start))
        .route("/auth/:provider/callback", get(auth_callback))
        .route("/api/providers", get(providers_list))
        .route("/api/me", get(me))
        .route("/api/me/username", patch(update_username))
        .route("/api/logout", post(logout))
        .route("/api/profile", axum::routing::delete(delete_profile))
        .route("/api/tokens", get(tokens_list).post(tokens_create))
        .route("/api/tokens/:id", axum::routing::delete(tokens_revoke))
        .route("/api/tokens/:id/audit", get(tokens_audit))
        .route("/api/workspaces/open", get(workspaces_open))
        .route("/api/workspaces/owned", get(workspaces_owned))
        .route("/api/workspaces/incoming", get(workspaces_incoming))
        .route("/api/workspaces", post(workspaces_create))
        .route(
            "/api/workspaces/:workspace",
            axum::routing::delete(workspaces_delete),
        )
        .route(
            "/api/workspaces/:workspace/grants",
            get(workspace_grants_list).post(workspace_grants_create),
        )
        .route(
            "/api/grants/:id",
            axum::routing::delete(workspace_grants_delete),
        )
        .route("/s/:owner/:workspace", get(share_landing))
        .route(
            "/desktop/authorize",
            get(crate::desktop_authorize::authorize),
        )
        .route(
            "/desktop/authorize/consent",
            get(crate::desktop_authorize::consent),
        )
        .route(
            "/desktop/authorize/confirm",
            post(crate::desktop_authorize::confirm),
        )
        .merge(internal)
        .fallback(static_files::handler)
        .with_state(state)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn auth_start(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    session: Session,
) -> Result<Redirect> {
    let p = state.cfg.provider(&provider).ok_or(Error::NotFound)?;
    let redirect = state.cfg.redirect_uri(p.name());
    let (url, csrf, verifier) = p.authorize_url(&redirect)?;
    session
        .insert(
            KEY_PENDING,
            &PendingOauth {
                provider: p.name().to_string(),
                state: csrf.secret().clone(),
                verifier: verifier.secret().clone(),
            },
        )
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session insert: {e}")))?;
    Ok(Redirect::to(url.as_str()))
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn auth_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(q): Query<CallbackParams>,
    headers: HeaderMap,
    session: Session,
) -> Result<Redirect> {
    // Bound the entire callback at 15s. The provider's `state` lifetime
    // and the user's patience both run out well before the worst-case
    // sum of sequential profile-service awaits (exchange + upsert +
    // flags + audit + cycle_id + insert + audit + claim_grants), so a
    // slow profile cannot strand the OAuth window.
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        auth_callback_inner(state, provider, q, headers, session),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => {
            tracing::warn!("auth_callback exceeded 15s deadline");
            Err(Error::Anyhow(anyhow::anyhow!("auth_callback timed out")))
        }
    }
}

async fn auth_callback_inner(
    state: AppState,
    provider: String,
    q: CallbackParams,
    headers: HeaderMap,
    session: Session,
) -> Result<Redirect> {
    if let Some(err) = q.error {
        // Provider error codes are OAuth-spec values (`access_denied`,
        // `server_error`, etc.); echoing them is safe. The SPA renders
        // the response body via Svelte interpolation which HTML-escapes,
        // so a hostile provider can't smuggle HTML/JS here either.
        return Err(Error::BadRequest(format!("provider error: {err}")));
    }
    let code = q
        .code
        .ok_or_else(|| Error::BadRequest("missing code".into()))?;
    let state_param = q
        .state
        .ok_or_else(|| Error::BadRequest("missing state".into()))?;

    let pending: PendingOauth = session
        .remove(KEY_PENDING)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session remove: {e}")))?
        .ok_or(Error::BadRequest("no pending oauth".into()))?;
    // Validate state first (constant-time) so a non-constant-time
    // provider compare can't be used to oracle which provider the
    // session expects via response-time differences. Provider check
    // is a plain compare because the value was already trusted on
    // /auth/:provider entry; pairing it with state validation just
    // catches a session that crossed providers mid-flow.
    if !ct_eq(&pending.state, &state_param) {
        return Err(Error::BadRequest("state mismatch".into()));
    }
    if pending.provider != provider {
        return Err(Error::BadRequest("provider mismatch".into()));
    }

    let p = state.cfg.provider(&provider).ok_or(Error::NotFound)?;
    let redirect_uri = state.cfg.redirect_uri(p.name());
    let info = p
        .exchange(
            &code,
            PkceCodeVerifier::new(pending.verifier),
            &redirect_uri,
        )
        .await?;

    // One atomic round trip: find existing identity, else attach
    // identity to the existing user with this email, else create
    // user + identity. Avatar refresh on the steady-state branch is
    // folded into the same tx server-side. A single transaction is
    // what prevents orphan user rows on concurrent first-time logins
    // and lets a second provider attach to an existing user by email
    // instead of failing on a duplicate.
    let upsert = state
        .cfg
        .profile_client
        .upsert_by_identity(
            p.name(),
            &info.provider_subject,
            info.email.as_deref(),
            info.display_name.as_deref(),
            info.picture_url.as_deref(),
        )
        .await?;
    let user = upsert.user;

    let ip = client_ip(&headers);
    let ua = user_agent(&headers);

    if user.is_blocked() {
        // Record the denied attempt before bouncing the user. The
        // session is never granted, so the SPA never reaches /api/me.
        // Forensic only: a profile outage here loses the row but does
        // not change the user-facing behavior. Surface the failure via
        // warn so an audit gap is visible in logs.
        if let Err(e) = state
            .cfg
            .profile_client
            .write_auth_audit(
                user.id,
                "login_denied",
                ip.as_deref(),
                ua.as_deref(),
                user.block_reason.as_deref(),
            )
            .await
        {
            tracing::warn!(error = ?e, user = %user.username, "write_auth_audit (blocked) failed");
        }
        // If the user was bounced here by /desktop/authorize, finish
        // the flow with a chan:// error redirect so the desktop client
        // can render its own "blocked" panel.
        if let Some(params) = crate::desktop_authorize::take_pending(&session).await? {
            return Ok(Redirect::to(&crate::desktop_authorize::error_url(
                &params,
                "account_blocked",
            )));
        }
        return Err(Error::Forbidden("account blocked"));
    }

    // Feature-flag gate. `oauth_login` is the allowlist for sign-in.
    // Profile resolves the per-user override on top of the registry
    // default; a fresh deploy ships `default_enabled=false`, so only
    // explicitly granted users can sign in. The deny path leaves the
    // user row in place (matches the blocked-account posture) and
    // 303s to the SPA's denied panel. We do this *before* cycle_id
    // so the session never holds an authenticated state for a
    // denied account.
    let flags = state
        .cfg
        .profile_client
        .get_user_flags(user.id)
        .await
        .unwrap_or_default();
    if !flags.get("oauth_login").copied().unwrap_or(false) {
        if let Err(e) = state
            .cfg
            .profile_client
            .write_auth_audit(
                user.id,
                "login_denied",
                ip.as_deref(),
                ua.as_deref(),
                Some("oauth_login flag not granted"),
            )
            .await
        {
            tracing::warn!(error = ?e, user = %user.username, "write_auth_audit (oauth_login deny) failed");
        }
        // Desktop bounce: route the deny back to chan-desktop via the
        // chan:// fragment so the desktop client can surface it.
        if let Some(params) = crate::desktop_authorize::take_pending(&session).await? {
            return Ok(Redirect::to(&crate::desktop_authorize::error_url(
                &params,
                "oauth_denied",
            )));
        }
        return Ok(Redirect::to("/?denied=oauth_login"));
    }

    // Rotate the session id at the privilege boundary: anything that
    // was in this session before sign-in (pending OAuth state, anon
    // CSRF nonces, a cookie an attacker planted on the victim's
    // browser pre-login) keeps the old id, the freshly authenticated
    // state lives under a new one. Prevents session fixation: a
    // pre-set `id_session` cannot survive the authentication step.
    session
        .cycle_id()
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session cycle_id: {e}")))?;

    session
        .insert(KEY_USER, &user.id)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session insert: {e}")))?;

    if let Err(e) = state
        .cfg
        .profile_client
        .write_auth_audit(
            user.id,
            "login",
            ip.as_deref(),
            ua.as_deref(),
            Some(p.name()),
        )
        .await
    {
        tracing::warn!(error = ?e, user = %user.username, "write_auth_audit (login) failed");
    }

    // Best-effort claim sweep. Profile fills `grantee_user_id` on
    // any pending grant whose email matches one of the user's
    // verified addresses. Pass the user's primary email plus the
    // freshly-observed provider email; previous providers' emails
    // would already have been swept on their own callbacks. A failure
    // here logs and continues so an unhealthy profile call doesn't
    // block sign-in.
    //
    // Caveat: `users.email` is verified-at-link-time, not re-verified
    // here. Provider reassignment (Google Workspace / Microsoft
    // tenant) of the address after signup could theoretically let a
    // stale `users.email` claim a grant intended for the new owner.
    // The freshly-observed provider email is always re-verified
    // through the provider's own check, so the new-owner side will
    // also sweep it on their next sign-in; we accept the race.
    let mut emails: Vec<String> = vec![user.email.clone()];
    if let Some(e) = info.email.as_deref() {
        if !e.eq_ignore_ascii_case(&user.email) {
            emails.push(e.to_string());
        }
    }
    match state
        .cfg
        .profile_client
        .claim_grants(user.id, &emails)
        .await
    {
        Ok(claimed) if claimed > 0 => {
            tracing::info!(user = %user.username, claimed, "claimed pending workspace grants");
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = ?e, user = %user.username, "claim_grants failed");
        }
    }

    // Desktop bounce: if /desktop/authorize stashed params before
    // sending the user through OAuth, hand the user to the consent
    // page. We do NOT mint here — that needs the user's explicit
    // click on the consent form. peek (not take) so the stash
    // survives across reloads of the consent page.
    if crate::desktop_authorize::peek_pending(&session)
        .await?
        .is_some()
    {
        return Ok(Redirect::to(crate::desktop_authorize::CONSENT_PATH));
    }

    // Resume share landing (or any other stashed return path) if the
    // pre-login redirect was set on this session. We validate it
    // starts with `/` and is not a protocol-relative URL (`//host`)
    // so an attacker cannot use the stash to point us at another
    // origin after login.
    let dest = match session.remove::<String>(KEY_POST_LOGIN_REDIRECT).await {
        Ok(Some(p)) if is_safe_local_redirect(&p) => p,
        Ok(_) => "/".to_string(),
        Err(e) => {
            tracing::warn!(error = ?e, "session remove post_login_redirect failed");
            "/".to_string()
        }
    };
    Ok(Redirect::to(&dest))
}

/// Allow only paths that stay on this origin: must start with a
/// single `/`, must not be protocol-relative (`//evil.com`), and
/// must not contain a scheme separator. Empty / malformed strings
/// fall through to `/` at the caller.
///
/// Intentionally coarse: this is a denylist over "could a browser
/// follow this off-origin?" The `!contains(':')` clause forbids
/// `javascript:` and any path containing a colon (e.g. matrix-style
/// `;jsessionid=`). It also rejects benign paths like `/foo:bar`,
/// which we don't mint anywhere, so the false-positive cost is zero
/// against the same-origin safety win.
fn is_safe_local_redirect(p: &str) -> bool {
    p.starts_with('/') && !p.starts_with("//") && !p.contains(':')
}

pub(crate) async fn current_user_id(session: &Session) -> Result<Uuid> {
    session
        .get::<Uuid>(KEY_USER)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session get: {e}")))?
        .ok_or(Error::Unauthorized)
}

/// Same as [`current_user_id`] but absence of a session returns `Ok(None)`
/// instead of `Unauthorized`. Used by handlers that have an
/// unauthenticated fall-through (`/desktop/authorize` bounces through
/// sign-in before completing).
pub(crate) async fn current_user_id_optional(session: &Session) -> Result<Option<Uuid>> {
    session
        .get::<Uuid>(KEY_USER)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session get: {e}")))
}

/// Resolve the session to a non-blocked user. Used by every
/// management endpoint (rename, mint/revoke/list/audit tokens) so a
/// blocked user can't keep mutating their account through a stale
/// cookie. `me`, `logout` and `delete_profile` deliberately don't
/// gate on blocked: `me` returns the row so the SPA can render the
/// blocked view, the other two are always permitted (right to log
/// out, right to delete).
async fn current_active_user(state: &AppState, session: &Session) -> Result<User> {
    let uid = current_user_id(session).await?;
    let pc = &state.cfg.profile_client;
    let Some(user) = pc.get_user(uid).await? else {
        let _ = session.flush().await;
        return Err(Error::Unauthorized);
    };
    if user.is_blocked() {
        return Err(Error::Forbidden("account blocked"));
    }
    Ok(user)
}

#[derive(Serialize)]
struct WorkspaceView {
    /// Workspace slug (`{user}.workspace.chan.app/{workspace}/`).
    workspace: String,
    /// Display label. Defaults to the workspace slug until the wire
    /// carries a separate label.
    label: String,
    public: bool,
    /// "online" while the tunnel registration is live.
    status: &'static str,
}

#[derive(Serialize)]
struct MeResponse {
    user: User,
    /// Live tunnel snapshot for this user, sourced from workspace-proxy
    /// admin. Empty when the user has no `chan serve` connected (or
    /// is blocked, or workspace-proxy is unreachable; in the unreachable
    /// case we log and serve an empty list so the dashboard renders).
    workspaces: Vec<WorkspaceView>,
    /// Resolved feature flags for this user. Map of flag_key -> bool.
    /// Sourced from profile each call (no caching) so a gradual
    /// rollout takes effect on the next dashboard reload.
    flags: gateway_common::profile_client::FlagMap,
}

async fn me(State(state): State<AppState>, session: Session) -> Result<Response> {
    let uid = current_user_id(&session).await?;
    let pc = &state.cfg.profile_client;
    // User vanished underneath the cookie: invalidate and 401.
    let Some(user) = pc.get_user(uid).await? else {
        let _ = session.flush().await;
        return Err(Error::Unauthorized);
    };

    // Workspace list comes from workspace-proxy. Blocked users get an empty
    // list; the SPA renders the blocked view from `user.blocked_at`.
    // workspace-proxy outages also surface as empty (with a log line)
    // rather than failing the whole `/api/me`: the dashboard is the
    // user's only way to discover other state (rename, PATs, account
    // delete), and that state still loads from profile-service.
    let workspaces = if user.is_blocked() {
        Vec::new()
    } else if let Some(client) = &state.cfg.workspace_admin {
        match client.list_user_tunnels(&user.username).await {
            Ok(rows) => rows
                .into_iter()
                .map(|t| WorkspaceView {
                    label: t.workspace.clone(),
                    workspace: t.workspace,
                    public: t.public,
                    status: "online",
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = ?e, user = %user.username, "workspace list fetch failed");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Resolve feature flags for this user. Profile unhealthy =>
    // empty map (SPA falls back to "feature off" for everything,
    // which is the safe default).
    let flags = state
        .cfg
        .profile_client
        .get_user_flags(user.id)
        .await
        .unwrap_or_default();

    Ok(Json(MeResponse {
        user,
        workspaces,
        flags,
    })
    .into_response())
}

async fn logout(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> Result<StatusCode> {
    // Read the user_id before flushing so we can attribute the audit
    // row; absent (already-logged-out) sessions just skip the write.
    let uid = session.get::<Uuid>(KEY_USER).await.ok().flatten();
    session
        .flush()
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session flush: {e}")))?;
    if let Some(uid) = uid {
        let ip = client_ip(&headers);
        let ua = user_agent(&headers);
        let _ = state
            .cfg
            .profile_client
            .write_auth_audit(uid, "logout", ip.as_deref(), ua.as_deref(), None)
            .await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Constant-time string equality for OAuth state and bearer
/// comparison. Length inequality short-circuits to false; this leaks
/// the length but no byte of the secret.
fn ct_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[derive(Serialize)]
struct ProvidersResponse {
    providers: Vec<&'static str>,
}

async fn providers_list(State(state): State<AppState>) -> Json<ProvidersResponse> {
    Json(ProvidersResponse {
        providers: state.cfg.providers.iter().map(|p| p.name()).collect(),
    })
}

async fn delete_profile(State(state): State<AppState>, session: Session) -> Result<StatusCode> {
    let uid = current_user_id(&session).await?;
    // Look the user up before delete so we can hand workspace-proxy the
    // username for the bulk tunnel evict; the row is gone after the
    // DELETE returns, including via FK cascade. Tolerate "already
    // gone" (cookie outlived the row) by treating None as a no-op.
    let username = state
        .cfg
        .profile_client
        .get_user(uid)
        .await?
        .map(|u| u.username);

    // FK cascades clean up identities and api_tokens.
    state.cfg.profile_client.delete_user(uid).await?;

    // Best-effort: drop every live tunnel the user had open.
    // workspace-proxy holds those substreams in-process, so the cascade
    // above doesn't reach them. A failure here logs and continues;
    // the remote chan serve will get rejected on its next handshake
    // anyway because the PAT is now gone.
    if let (Some(client), Some(name)) = (&state.cfg.workspace_admin, username) {
        match client.kill_user_tunnels(&name).await {
            Ok(killed) if killed > 0 => {
                tracing::info!(user = %name, killed, "evicted tunnels on account delete");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = ?e, user = %name, "tunnel evict on delete failed");
            }
        }
    }

    let _ = session.flush().await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct UsernameBody {
    username: String,
}

#[derive(Serialize)]
struct UsernameResponse {
    username: String,
    edits_remaining: i32,
}

/// Validate the candidate before sending it upstream. Cheap rejects
/// happen here so the SPA gets a fast, specific error; profile-
/// service still re-checks format and uniqueness as a safety net.
async fn update_username(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<UsernameBody>,
) -> Result<Json<UsernameResponse>> {
    let user = current_active_user(&state, &session).await?;
    let uid = user.id;
    let candidate = body.username.trim().to_ascii_lowercase();

    if !valid_username(&candidate) {
        return Err(Error::BadRequest(
            "username must be 3-32 chars, lowercase alphanumeric or hyphen, no leading/trailing hyphen".into(),
        ));
    }
    if RESERVED_USERNAMES
        .binary_search(&candidate.as_str())
        .is_ok()
    {
        return Err(Error::BadRequest("username is reserved".into()));
    }
    // No explicit reject for the `u<hex>` placeholder shape: the
    // UNIQUE index on username plus profile-service's CAS update
    // (lower(username) <> $2) already make it impossible to rename
    // onto another user's placeholder, and renaming to your own
    // current handle is a no-op upstream.
    // rustrict: leet-normalises and matches an internal profanity
    // list. Adequate for usernames; known to false-positive on some
    // place names and short letter combinations. `RUSTRICT_ALLOWLIST`
    // is a comma-separated escape hatch: any handle that appears in
    // it bypasses the filter (case-insensitive). The check itself
    // still runs after every other validation (length, charset,
    // reserved list) so the allowlist cannot reintroduce shapes the
    // earlier rules already refused.
    if candidate.is_inappropriate() && !is_rustrict_allowed(&candidate) {
        return Err(Error::BadRequest("username not allowed".into()));
    }

    let user = state
        .cfg
        .profile_client
        .update_username(uid, &candidate)
        .await?;

    Ok(Json(UsernameResponse {
        username: user.username,
        edits_remaining: (MAX_USERNAME_EDITS - user.username_edits).max(0),
    }))
}

/// Returns true when the candidate (already lowercased, ASCII)
/// matches an entry in `RUSTRICT_ALLOWLIST`. Env is parsed on every
/// call; the value is short and the rename path is cold.
fn is_rustrict_allowed(candidate: &str) -> bool {
    let Ok(raw) = std::env::var("RUSTRICT_ALLOWLIST") else {
        return false;
    };
    raw.split(',')
        .map(|s| s.trim())
        .any(|s| s.eq_ignore_ascii_case(candidate))
}

#[derive(Debug, Deserialize)]
struct CreateTokenBody {
    label: String,
    /// Lifetime in seconds. None = never expires; the SPA presets
    /// 30d / 90d / 1y / never as the issue requested, but the
    /// concrete expiry is computed client-side and sent here.
    expires_in: Option<i64>,
    /// Capabilities to grant the token. When absent (or empty), the
    /// service falls back to `DEFAULT_TOKEN_SCOPES` (`["tunnel"]`):
    /// the holder can dial chan-tunnel but cannot host a publicly-
    /// readable workspace. Grant `"tunnel.public"` explicitly when
    /// minting tokens for users authorised to share anonymously.
    #[serde(default)]
    scopes: Option<Vec<String>>,
}

#[derive(Serialize)]
struct TokenView {
    id: Uuid,
    label: String,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    scopes: Vec<String>,
}

impl From<ApiToken> for TokenView {
    fn from(t: ApiToken) -> Self {
        Self {
            id: t.id,
            label: t.label,
            expires_at: t.expires_at,
            created_at: t.created_at,
            revoked_at: t.revoked_at,
            last_used_at: t.last_used_at,
            scopes: t.scopes,
        }
    }
}

#[derive(Serialize)]
struct CreatedTokenView {
    #[serde(flatten)]
    token: TokenView,
    /// Plaintext PAT. Shown in the UI exactly once on creation;
    /// never returned again from any endpoint.
    secret: String,
}

async fn tokens_create(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(body): Json<CreateTokenBody>,
) -> Result<(StatusCode, Json<CreatedTokenView>)> {
    let uid = current_active_user(&state, &session).await?.id;
    let expires_at = body
        .expires_in
        .filter(|s| *s > 0)
        .map(|s| Utc::now() + chrono::Duration::seconds(s));

    let scopes: Vec<String> = match body.scopes {
        Some(ref s) if !s.is_empty() => s.clone(),
        _ => DEFAULT_TOKEN_SCOPES
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    let CreatedToken { token, secret } = state
        .api_tokens
        .create(
            NewToken {
                user_id: uid,
                label: &body.label,
                expires_at,
                scopes: &scopes,
                origin: TokenOrigin::Spa,
            },
            &request_meta(&headers),
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedTokenView {
            token: token.into(),
            secret,
        }),
    ))
}

async fn tokens_list(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<TokenView>>> {
    let uid = current_active_user(&state, &session).await?.id;
    let tokens = state.api_tokens.list(uid).await?;
    Ok(Json(tokens.into_iter().map(Into::into).collect()))
}

async fn tokens_revoke(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let user = current_active_user(&state, &session).await?;
    let uid = user.id;
    if !state
        .api_tokens
        .revoke(uid, id, &request_meta(&headers))
        .await?
    {
        return Err(Error::NotFound);
    }
    // Best-effort: drop every live tunnel the user has. We can't
    // selectively kill the tunnel(s) backed by this specific PAT
    // (chan-tunnel-server doesn't track which token registered which
    // substream), so a revoke pulls down everything the user has
    // open. chan-serve instances using a non-revoked token will
    // reconnect on the next handshake; instances using the revoked
    // token fail the next validate and stay disconnected. A failure
    // to reach workspace-proxy logs and continues; the next handshake
    // will refuse the token anyway via the DB check.
    if let Some(client) = &state.cfg.workspace_admin {
        match client.kill_user_tunnels(&user.username).await {
            Ok(killed) if killed > 0 => {
                tracing::info!(user = %user.username, killed, "evicted tunnels on PAT revoke");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = ?e, user = %user.username, "tunnel evict on revoke failed");
            }
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    /// Cap rows returned; defaults to 50, hard-clamped to 200.
    limit: Option<i64>,
}

async fn tokens_audit(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>> {
    let uid = current_active_user(&state, &session).await?.id;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = state.api_tokens.audit(uid, id, limit).await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct WorkspacesOpenQuery {
    u: String,
    d: String,
}

/// Mint a workspace-gate entry token and 303 the browser to
/// `https://{u}.workspace.chan.app/{d}/?t=<jwt>`. workspace-proxy verifies
/// the token, sets a host-only `workspace_gate` cookie, and 303s to the
/// clean URL. The whole handshake is invisible to the user past the
/// initial click.
///
/// Authorization (single path, owner or grantee):
///   * caller must hold an authed (non-blocked) session;
///   * `u` is resolved to a user record (case-insensitive username
///     lookup); 404 if unknown so the endpoint cannot be used to
///     probe handles;
///   * profile `workspace_access?as=<self>` is consulted: owner returns
///     `owner`, accepted grantee returns `viewer`/`editor`, anyone
///     else 404 (same shape as "unknown workspace");
///   * `d` is verified live in `u`'s tunnel registry on workspace-proxy
///     (cheap defense-in-depth and a friendly 404 instead of a
///     valid-token-into-a-cold-workspace race).
///
/// The entry token is short-lived (30s). workspace-proxy validates
/// signature + exp + aud (`{u}.workspace.chan.app`) + drv (`{d}`) + sub
/// (the caller's user_id, *not* the owner's, so the workspace_gate
/// cookie minted on the next leg carries the right identity for
/// upstream collab attribution) and then issues its own 24h session
/// JWT cookie.
async fn workspaces_open(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<WorkspacesOpenQuery>,
) -> Result<Redirect> {
    let caller = current_active_user(&state, &session).await?;
    let owner_handle = q.u.trim().to_ascii_lowercase();
    let workspace = q.d.trim().to_ascii_lowercase();
    if !valid_username(&owner_handle) || !is_workspace_name_shape(&workspace) {
        return Err(Error::NotFound);
    }

    // Resolve owner. Owner == caller is fast-pathable but the
    // username lookup is one cheap query, so we always go through it
    // for one code path.
    let owner = state
        .cfg
        .profile_client
        .find_user_by_username(&owner_handle)
        .await?
        .ok_or(Error::NotFound)?;

    // Authorization. 404 on no-access matches the unknown-workspace shape
    // so this endpoint cannot be used to enumerate which workspaces an
    // owner is currently sharing or what handles exist.
    state
        .cfg
        .profile_client
        .workspace_access(owner.id, &workspace, caller.id)
        .await?
        .ok_or(Error::NotFound)?;

    // Verify the workspace is actually live before minting. workspace-proxy is
    // the authority on live registrations.
    let client =
        state.cfg.workspace_admin.as_ref().ok_or_else(|| {
            Error::Anyhow(anyhow::anyhow!("workspace admin client not configured"))
        })?;
    let live = client.list_user_tunnels(&owner.username).await?;
    if !live.iter().any(|t| t.workspace == workspace) {
        return Err(Error::NotFound);
    }

    let host = state.cfg.workspace_host_for(&owner.username);
    let token = gateway_common::workspace_gate::encode_entry(
        state.cfg.workspace_gate_secret.as_bytes(),
        caller.id,
        &workspace,
        &host,
    )
    .map_err(|e| Error::Anyhow(anyhow::anyhow!("mint entry token: {e}")))?;

    // 303 (See Other) is the right shape for a GET that produced a
    // resource we want the browser to navigate to. Token rides in
    // the URL because that's the only way to hand it off across
    // origins without JS; the 30s exp keeps the leak window short.
    //
    // Scheme + port come from config so a local-dev deploy where
    // `*.workspace.localtest.me` resolves to 127.0.0.1 over plain HTTP
    // builds the right URL.
    let url = format!(
        "{scheme}://{host}{port}/{workspace}/?t={token}",
        scheme = state.cfg.workspace_public_scheme,
        port = state.cfg.workspace_public_port,
    );
    Ok(Redirect::to(&url))
}

// ---------------------------------------------------------------------------
// Workspace sharing (grants)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateGrantBody {
    grantee_email: String,
    role: String,
}

/// Owner creates / promotes a grant on one of their workspaces. The
/// session user is the owner; the URL carries only the workspace name
/// (not the owner's id), so a stale tab cannot mint grants against
/// somebody else's workspace.
async fn workspace_grants_create(
    State(state): State<AppState>,
    session: Session,
    Path(workspace): Path<String>,
    Json(body): Json<CreateGrantBody>,
) -> Result<(StatusCode, Json<WorkspaceGrant>)> {
    let user = current_active_user(&state, &session).await?;
    // Surface format errors before the round trip; profile re-checks.
    let workspace = workspace.trim().to_ascii_lowercase();
    if !is_workspace_name_shape(&workspace) {
        return Err(Error::BadRequest("invalid workspace name".into()));
    }
    let role = body.role.trim();
    if role != "viewer" && role != "editor" {
        return Err(Error::BadRequest("role must be viewer or editor".into()));
    }
    let grant = state
        .cfg
        .profile_client
        .create_workspace_grant(user.id, &workspace, body.grantee_email.trim(), role)
        .await?;
    Ok((StatusCode::CREATED, Json(grant)))
}

async fn workspace_grants_list(
    State(state): State<AppState>,
    session: Session,
    Path(workspace): Path<String>,
) -> Result<Json<Vec<WorkspaceGrant>>> {
    let user = current_active_user(&state, &session).await?;
    let workspace = workspace.trim().to_ascii_lowercase();
    if !is_workspace_name_shape(&workspace) {
        return Err(Error::BadRequest("invalid workspace name".into()));
    }
    let rows = state
        .cfg
        .profile_client
        .list_workspace_grants(user.id, &workspace)
        .await?;
    Ok(Json(rows))
}

async fn workspace_grants_delete(
    State(state): State<AppState>,
    session: Session,
    Path(grant_id): Path<Uuid>,
) -> Result<StatusCode> {
    let user = current_active_user(&state, &session).await?;
    // Pass the session user as owner_id; profile's DELETE filters on
    // `id = $1 AND owner_user_id = $2`, so a bug here cannot let
    // user A revoke user B's grant — 404 from profile instead.
    state
        .cfg
        .profile_client
        .delete_workspace_grant(user.id, grant_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn workspaces_owned(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<OwnedWorkspaceSummary>>> {
    let user = current_active_user(&state, &session).await?;
    let rows = state
        .cfg
        .profile_client
        .list_owned_workspaces(user.id)
        .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct CreateWorkspaceBody {
    workspace_name: String,
}

/// Create one workspace in the owner's namespace. Idempotent at
/// profile-service: re-issuing for the same name returns the
/// existing row.
async fn workspaces_create(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<CreateWorkspaceBody>,
) -> Result<Json<Workspace>> {
    let user = current_active_user(&state, &session).await?;
    let name = body.workspace_name.trim().to_ascii_lowercase();
    if !is_workspace_name_shape(&name) {
        return Err(Error::BadRequest("invalid workspace name".into()));
    }
    let workspace = state
        .cfg
        .profile_client
        .create_workspace(user.id, &name)
        .await?;
    Ok(Json(workspace))
}

async fn workspaces_delete(
    State(state): State<AppState>,
    session: Session,
    Path(workspace): Path<String>,
) -> Result<StatusCode> {
    let user = current_active_user(&state, &session).await?;
    let name = workspace.trim().to_ascii_lowercase();
    if !is_workspace_name_shape(&name) {
        return Err(Error::BadRequest("invalid workspace name".into()));
    }
    state
        .cfg
        .profile_client
        .delete_workspace(user.id, &name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn workspaces_incoming(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<IncomingShare>>> {
    let user = current_active_user(&state, &session).await?;
    let rows = state
        .cfg
        .profile_client
        .list_incoming_shares(user.id)
        .await?;
    Ok(Json(rows))
}

/// Shape-only validator; profile re-checks. 1-64 chars, lowercase
/// ascii alnum + `[._-]`, with `.` / `..` / leading-dot rejected to
/// match the canonical rule in profile-service.
fn is_workspace_name_shape(s: &str) -> bool {
    let len = s.len();
    if !(1..=64).contains(&len) {
        return false;
    }
    if s == "." || s == ".." || s.starts_with('.') {
        return false;
    }
    s.bytes()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'-' | b'_' | b'.'))
}

// ---------------------------------------------------------------------------
// Share landing
// ---------------------------------------------------------------------------

/// Public entry point for a copied share link.
///
/// Flow:
///   1. If the caller has no session, stash `/s/:owner/:workspace` and
///      303 to `/` so the SPA shows the OAuth picker. The callback
///      reads the stash and 303s back here after sign-in.
///   2. With a session, resolve `:owner` (username -> User) and call
///      profile `workspace_access?as=<self>`. The owner case and the
///      grantee case both return a role; no-access returns 404.
///   3. On access, mint an entry JWT against the owner's
///      `{owner}.workspace.chan.app` host and 303 to workspace-proxy so
///      workspace-proxy sets its `workspace_gate` cookie and serves the
///      content. The same 30s short-lived entry token shape used by
///      `/api/workspaces/open`.
async fn share_landing(
    State(state): State<AppState>,
    session: Session,
    Path((owner, workspace)): Path<(String, String)>,
) -> Result<Redirect> {
    let owner = owner.trim().to_ascii_lowercase();
    let workspace = workspace.trim().to_ascii_lowercase();
    if !valid_username(&owner) || !is_workspace_name_shape(&workspace) {
        return Err(Error::NotFound);
    }

    // Unauthenticated: stash + send to login. Use a 303 (See Other)
    // so a refresh on the SPA root doesn't re-trigger the share flow.
    let uid = session
        .get::<Uuid>(KEY_USER)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session get: {e}")))?;
    let Some(uid) = uid else {
        let dest = format!("/s/{owner}/{workspace}");
        session
            .insert(KEY_POST_LOGIN_REDIRECT, &dest)
            .await
            .map_err(|e| Error::Anyhow(anyhow::anyhow!("session insert: {e}")))?;
        return Ok(Redirect::to("/"));
    };

    // Resolve the owner handle. 404 is the same shape as "no access"
    // and "unknown workspace", so a stranger cannot probe the existence
    // of a handle through this route.
    let owner_user = state
        .cfg
        .profile_client
        .find_user_by_username(&owner)
        .await?
        .ok_or(Error::NotFound)?;

    let access = state
        .cfg
        .profile_client
        .workspace_access(owner_user.id, &workspace, uid)
        .await?
        .ok_or(Error::NotFound)?;

    let host = state.cfg.workspace_host_for(&owner_user.username);
    let token = gateway_common::workspace_gate::encode_entry(
        state.cfg.workspace_gate_secret.as_bytes(),
        uid,
        &workspace,
        &host,
    )
    .map_err(|e| Error::Anyhow(anyhow::anyhow!("mint entry token: {e}")))?;

    tracing::info!(
        owner = %owner_user.username,
        workspace = %workspace,
        caller = %uid,
        role = %access.role,
        "share landing: minting entry token",
    );

    let url = format!(
        "{scheme}://{host}{port}/{workspace}/?t={token}",
        scheme = state.cfg.workspace_public_scheme,
        port = state.cfg.workspace_public_port,
    );
    Ok(Redirect::to(&url))
}

async fn internal_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> std::result::Result<Response, Error> {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match provided {
        Some(t) if ct_eq(t, &state.cfg.internal_auth_token) => Ok(next.run(request).await),
        _ => Err(Error::Unauthorized),
    }
}

#[derive(Debug, Deserialize)]
struct ValidateBody {
    token: String,
}

async fn validate_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ValidateBody>,
) -> Result<Json<ValidatedToken>> {
    // Reject garbage before touching the throttle map. Otherwise every
    // random fingerprint takes a bucket slot under the LRU cap and an
    // attacker spamming malformed tokens can evict legitimate
    // fingerprints' state. A real PAT starts with `chan_pat_`; the same
    // 401 we'd return on "throttled" / "unknown token" keeps the shape
    // indistinguishable on the wire.
    if !body.token.starts_with("chan_pat_") {
        return Err(Error::Unauthorized);
    }
    // Per-token-fingerprint rate limit before the DB lookup. Same
    // shape as workspace-proxy's outer throttle: a throttled call comes
    // back as the same 401 an unknown-token call returns, so the
    // throttle is not observable on the wire. See the module doc
    // for the threat model.
    if !state.token_throttle.try_admit(&body.token) {
        tracing::warn!("internal validate_token throttled");
        return Err(Error::Unauthorized);
    }
    // chan-tunnel forwards the originating client IP via
    // X-Forwarded-For; we record that as the validate-IP for audit.
    let v = state
        .api_tokens
        .validate(&body.token, &request_meta(&headers))
        .await?;
    Ok(Json(v))
}

/// Bundle the audit-only request context (`client_ip` + `user_agent`)
/// for `ApiTokenService` calls.
pub(crate) fn request_meta(headers: &HeaderMap) -> RequestMeta {
    RequestMeta {
        ip: client_ip(headers),
        user_agent: user_agent(headers),
    }
}

pub(crate) fn client_ip(headers: &HeaderMap) -> Option<String> {
    // Production sits behind a reverse proxy that sets
    // X-Forwarded-For; in dev/test the header is absent and we
    // store NULL in the audit row. Stored as text so we don't
    // pull in the sqlx ipnetwork feature for an audit-only field.
    //
    // Trust boundary: the value is audit-only. *Never* use it for
    // authorization. If the service is ever reachable without nginx
    // in front, an attacker can spoof XFF and forge audit rows.
    // Operators must terminate XFF at nginx and either drop or
    // rewrite any inbound XFF so the chain we see comes only from
    // trusted hops.
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn user_agent(headers: &HeaderMap) -> Option<String> {
    // Truncate at 256 *bytes* on a UTF-8 boundary, not chars. A 256-
    // char limit lets a UA string with 4-byte code points reach
    // ~1 KiB in the DB row, which serves no purpose. UA strings in
    // the wild are ASCII so the typical path is char_indices = byte
    // indices anyway; this only matters for adversarial inputs.
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            const MAX: usize = 256;
            if s.len() <= MAX {
                s.to_string()
            } else {
                // Walk back to the nearest UTF-8 boundary at or before MAX.
                let mut end = MAX;
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                s[..end].to_string()
            }
        })
}
