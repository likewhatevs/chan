//! OAuth-style PAT mint for chan-desktop.
//!
//! Four routes:
//!
//!   * `GET  /desktop/authorize?<query>` -- entry point. Validates the
//!     query, stashes a [`AuthorizeParams`] struct in the session, and
//!     redirects: to `/` if unauthenticated (SPA sign-in renders, then
//!     `auth_callback` bounces back here), or straight to
//!     `/desktop/authorize/consent` if authenticated.
//!   * `GET  /desktop/authorize/consent` -- renders a server-side HTML
//!     consent page showing the requesting client, label, scopes, and
//!     expiry. Includes a hidden CSRF nonce stored alongside the
//!     pending params.
//!   * `POST /desktop/authorize/confirm` -- handles the `Authorize` /
//!     `Cancel` action. Consumes the pending params + CSRF; on
//!     `allow` mints a PAT through [`ApiTokenService::create`] with
//!     [`TokenOrigin::Desktop`], stashes it in the
//!     [`RedemptionStore`] under a one-time code, and answers 200
//!     with a handoff page that navigates the browser to
//!     `chan://auth/callback#code=&label=[&expires_at=]&state=`
//!     (`deny` / blocked carry `#error=&state=` instead) via a
//!     zero-delay meta refresh plus a manual "Open chan-desktop"
//!     fallback link. A 3xx answering the form POST would put the
//!     `chan://` hop inside the form submission's redirect chain,
//!     which Chrome subjects to the page's `form-action` CSP; the
//!     handoff page keeps the custom-scheme navigation out of any
//!     form chain entirely. The PAT secret never appears in the
//!     fragment or the page: only the redemption code does, and the
//!     page itself is `no-store` / `no-referrer`.
//!   * `POST /desktop/authorize/redeem` -- swaps a one-time code for
//!     the minted PAT (`{"code": ...}` -> `{id, secret, label,
//!     expires_at}`). Single-use with a [`REDEEM_TTL`] lifetime;
//!     unknown, expired, and replayed codes all answer 410.
//!
//! Both HTML pages render in the shared [`crate::pages`] shell (SPA
//! palette, inline CSS) under one strict CSP ([`crate::pages::CSP`]):
//! `default-src 'none'` with carve-outs for the inline styles, the
//! same-origin logo mask, and the consent form's same-origin POST.
//! `X-Frame-Options: DENY` + `frame-ancestors 'none'` keep a
//! malicious page from iframing the consent and clickjacking an
//! approval.
//!
//! Hardening posture:
//!   * `redirect_uri` exact-match against [`EXPECTED_REDIRECT_URI`].
//!     A bad redirect_uri returns 400; we never bounce the user to an
//!     attacker-supplied origin.
//!   * `state` must be present and bounded; without it the desktop
//!     client cannot tie the response to its request.
//!   * `expires_in` is clamped to [`MAX_EXPIRES_IN_SECS`].
//!   * `scopes` are checked against [`ALLOWED_SCOPES`]. The general
//!     `/api/tokens` path only checks scope shape; this stricter list
//!     applies here because the desktop flow is unattended and we
//!     want a known-bounded capability surface.
//!   * The consent POST is gated by a 32-byte CSRF nonce stored in
//!     the session and compared with `subtle::ConstantTimeEq`. The
//!     session cookie itself is `SameSite=Lax`, so a cross-site POST
//!     never carries the cookie; the explicit nonce is defense in
//!     depth and proves the user passed through the rendered consent
//!     page rather than POSTing directly.
//!   * The audit row for the resulting PAT is `created_via_desktop`
//!     (not `created`), and each redemption writes a `desktop.redeem`
//!     row, so operators and users can tell the desktop flow apart
//!     from SPA mints and see when the code was cashed in.
//!   * The redeem route has no session auth: possession of the code
//!     is the credential (TLS assumed). The code is 256-bit random,
//!     single-use, and dies after [`REDEEM_TTL`], so the handoff
//!     page's DOM (which keeps the chan:// URL until the tab closes;
//!     a custom-scheme navigation never unloads the document) holds
//!     nothing durably sensitive.
//!
//! Known limitations:
//!   * No per-session rate limit. A signed-in user spam-clicking
//!     `Authorize` mints PATs into their own table; audit-visible and
//!     bounded by the user's own account, so there is no server-side
//!     limit -- the audit log is the watch surface.
//!   * The redemption store is in-process memory. A single identity
//!     instance is an existing deployment assumption (the consent
//!     flow already stashes pending state server-side in-process); a
//!     multi-replica deployment would need a shared store, and a
//!     restart during the redemption window just forces a re-auth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Form, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Json;
use base64::Engine;
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tower_sessions::Session;
use url::form_urlencoded::byte_serialize;
use uuid::Uuid;

use crate::api_tokens::{ApiToken, CreatedToken, NewToken, TokenOrigin, ACTION_DESKTOP_REDEEM};
use crate::error::{Error, Result};
use crate::http::{
    current_user_id, current_user_id_optional, request_meta, AppState, DESKTOP_ACCOUNT_SCOPE,
    DESKTOP_CONNECT_SCOPE,
};
use crate::pages;
use crate::profile_client::User;

/// Session key under which `/desktop/authorize` stashes a pending
/// authorize. Read by `auth_callback` (to redirect to consent after
/// OAuth completes) and by the consent / confirm handlers.
pub const KEY_DESKTOP_AUTHORIZE: &str = "desktop_authorize";

/// Session key for the consent-form CSRF nonce. Regenerated each time
/// the consent page is rendered; consumed by the confirm POST.
const KEY_DESKTOP_CSRF: &str = "desktop_authorize_csrf";

/// Locked redirect target. Anything else is rejected with 400; we do
/// not maintain an allowlist because there is exactly one legitimate
/// chan-desktop scheme handler.
const EXPECTED_REDIRECT_URI: &str = "chan://auth/callback";

/// 90 days. Matches what the spec example sends (30d) with headroom
/// for future longer-lived desktop sessions. The clamp prevents a
/// hostile or buggy desktop build from issuing year-long credentials.
const MAX_EXPIRES_IN_SECS: i64 = 90 * 86_400;

/// Sanity cap on the echoed `state` value. 512 bytes is more than
/// enough for any reasonable nonce + extension data; anything larger
/// is either a misuse or an attempt to balloon the session row.
const MAX_STATE_LEN: usize = 512;

/// Scope allowlist for the desktop flow. Stricter than the shape
/// check `ApiTokenService::create` runs: scopes here must be one of
/// the desktop/tunnel vocabulary entries, so a desktop build cannot
/// mint a token carrying a typo'd or future-only scope. `tunnel` and
/// `desktop.connect` stay listed for shipped desktops (dropping
/// either would 400 their sign-in); new desktops request
/// `desktop.account` alone (see the sole-scope rule in [`validate`]).
const ALLOWED_SCOPES: &[&str] = &["tunnel", DESKTOP_CONNECT_SCOPE, DESKTOP_ACCOUNT_SCOPE];

/// Default when the client omits `scopes`. Matches the SPA / general
/// PAT default so silence means "private tunnel only".
const DEFAULT_SCOPES: &[&str] = &["tunnel"];

/// Path the consent page lives at. Exported so other modules (today
/// `auth_callback`) can build a redirect without restating the literal.
pub const CONSENT_PATH: &str = "/desktop/authorize/consent";

/// Lifetime of a one-time redemption code: long enough for the OS to
/// route the chan:// URL and the desktop to call back, short enough
/// that a code lifted from an open handoff tab is stale before anyone
/// could plausibly exfiltrate and replay it.
const REDEEM_TTL: Duration = Duration::from_secs(120);

/// What `/desktop/authorize/redeem` answers with, exactly once per
/// code. Besides `POST /api/tokens`, this is the only response that
/// ever carries a PAT secret.
#[derive(Debug, Clone, Serialize)]
pub struct RedeemPayload {
    pub id: Uuid,
    pub secret: String,
    pub label: String,
    /// Always present on the wire (`null` for a token that never
    /// expires); the desktop contract reads the key unconditionally.
    pub expires_at: Option<DateTime<Utc>>,
}

/// In-process single-use store for pending redemptions, shared via
/// `AppState`. Expired entries are swept on every insert and lookup,
/// so the map never outgrows the codes minted inside one TTL window.
#[derive(Clone, Default)]
pub struct RedemptionStore {
    inner: Arc<Mutex<HashMap<String, StoredRedemption>>>,
}

#[derive(Debug)]
struct StoredRedemption {
    payload: RedeemPayload,
    expires_at: Instant,
}

impl RedemptionStore {
    /// Stash `payload` under a fresh 256-bit code and return the code.
    pub fn insert(&self, payload: RedeemPayload) -> String {
        self.insert_with_ttl(payload, REDEEM_TTL)
    }

    fn insert_with_ttl(&self, payload: RedeemPayload, ttl: Duration) -> String {
        let code = generate_code();
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, v| v.expires_at > now);
        map.insert(
            code.clone(),
            StoredRedemption {
                payload,
                expires_at: now + ttl,
            },
        );
        code
    }

    /// Single-use lookup: the first take wins; every later take, and
    /// any take past the TTL, gets `None`.
    fn take(&self, code: &str) -> Option<RedeemPayload> {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, v| v.expires_at > now);
        map.remove(code).map(|s| s.payload)
    }
}

/// 32 random bytes, base64url: same entropy class as the PAT secret
/// the code stands in for.
fn generate_code() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Query shape parsed from the request URL.
#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    redirect_uri: String,
    state: String,
    label: String,
    /// Comma-separated scope list. Absent / empty -> [`DEFAULT_SCOPES`].
    #[serde(default)]
    scopes: Option<String>,
    /// Token lifetime in seconds. Clamped to [`MAX_EXPIRES_IN_SECS`];
    /// non-positive values are rejected (the desktop flow expects a
    /// finite token, never an immortal one).
    expires_in: Option<i64>,
}

/// Validated form of [`AuthorizeQuery`] suitable for stashing in the
/// session across the OAuth roundtrip + the consent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeParams {
    /// Already-validated to equal [`EXPECTED_REDIRECT_URI`]. Stored so
    /// a future second valid redirect target only changes the
    /// constant.
    redirect_uri: String,
    state: String,
    label: String,
    scopes: Vec<String>,
    expires_in_secs: i64,
}

/// Parse + validate. `Err` is a 400; the desktop client expects to
/// fix and retry its query string before any chan:// redirect happens.
fn validate(q: AuthorizeQuery) -> Result<AuthorizeParams> {
    if q.redirect_uri != EXPECTED_REDIRECT_URI {
        return Err(Error::BadRequest("invalid redirect_uri".into()));
    }
    let state = q.state.trim();
    if state.is_empty() || state.len() > MAX_STATE_LEN {
        return Err(Error::BadRequest("invalid state".into()));
    }
    let label = q.label.trim();
    if label.is_empty() || label.len() > 64 {
        return Err(Error::BadRequest("invalid label".into()));
    }
    let raw_scopes = q.scopes.unwrap_or_default();
    let scopes: Vec<String> = if raw_scopes.trim().is_empty() {
        DEFAULT_SCOPES.iter().map(|s| (*s).to_string()).collect()
    } else {
        raw_scopes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    if scopes.is_empty() {
        return Err(Error::BadRequest("invalid scopes".into()));
    }
    for s in &scopes {
        if !ALLOWED_SCOPES.contains(&s.as_str()) {
            return Err(Error::BadRequest("invalid scopes".into()));
        }
    }
    // Sole-scope rule: desktop.account already covers the whole
    // account (roster read + entry mint), so a request mixing it with
    // the per-devserver vocabulary is a confused client, not a
    // capability request we can honor.
    if scopes.iter().any(|s| s == DESKTOP_ACCOUNT_SCOPE) && scopes.len() > 1 {
        return Err(Error::BadRequest("invalid scopes".into()));
    }
    // Clamp instead of reject: the spec note says "Cap expires_in to
    // whatever your policy max is. Don't trust the client." Clamping
    // keeps an over-eager desktop build working at the policy ceiling
    // instead of failing outright.
    let expires_in_secs = match q.expires_in {
        Some(n) if n > 0 => n.min(MAX_EXPIRES_IN_SECS),
        _ => return Err(Error::BadRequest("invalid expires_in".into())),
    };
    Ok(AuthorizeParams {
        redirect_uri: q.redirect_uri,
        state: state.to_string(),
        label: label.to_string(),
        scopes,
        expires_in_secs,
    })
}

/// Build the success redirect target. The fragment carries the
/// one-time redemption code -- never the PAT secret -- plus the token
/// metadata the desktop shows while it redeems. Fragment (not query)
/// keeps it out of any access log along the redirect chain (browsers
/// do not send the fragment to servers).
pub fn success_url(params: &AuthorizeParams, code: &str, token: &ApiToken) -> String {
    let mut frag = String::new();
    push_pair(&mut frag, "code", code);
    push_pair(&mut frag, "label", &token.label);
    if let Some(exp) = token.expires_at {
        push_pair(&mut frag, "expires_at", &exp.to_rfc3339());
    }
    push_pair(&mut frag, "state", &params.state);
    format!("{}#{}", params.redirect_uri, frag)
}

/// Build the error redirect target. Same fragment encoding as
/// [`success_url`]; the desktop client decides how to surface
/// `reason` to the user. `reason` is a short stable token
/// (`account_blocked`, `oauth_denied`, `user_cancelled`,
/// `mint_failed`) so logs and downstream UI can branch without
/// parsing English.
pub fn error_url(params: &AuthorizeParams, reason: &str) -> String {
    let mut frag = String::new();
    push_pair(&mut frag, "error", reason);
    push_pair(&mut frag, "state", &params.state);
    format!("{}#{}", params.redirect_uri, frag)
}

fn push_pair(buf: &mut String, key: &str, value: &str) {
    if !buf.is_empty() {
        buf.push('&');
    }
    buf.push_str(key);
    buf.push('=');
    let encoded: String = byte_serialize(value.as_bytes()).collect();
    buf.push_str(&encoded);
}

/// Peek at a pending authorize without consuming it. Used by
/// `auth_callback` to decide whether to redirect to consent.
pub async fn peek_pending(session: &Session) -> Result<Option<AuthorizeParams>> {
    session
        .get::<AuthorizeParams>(KEY_DESKTOP_AUTHORIZE)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session get desktop_authorize: {e}")))
}

/// Read + remove a pending authorize. Used by the deny branches in
/// `auth_callback` (blocked account, oauth_login deny) and by the
/// confirm POST.
pub async fn take_pending(session: &Session) -> Result<Option<AuthorizeParams>> {
    session
        .remove::<AuthorizeParams>(KEY_DESKTOP_AUTHORIZE)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session remove desktop_authorize: {e}")))
}

/// Generate a 32-byte CSRF nonce, base64url-encoded. Stored in the
/// session and surfaced into the consent form as a hidden field.
fn generate_csrf() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Which handoff variant `confirm` renders: the copy differs, the
/// mechanics (meta refresh + manual link to the `chan://` target) do
/// not.
enum Handoff {
    /// PAT minted; the target carries `#code=…`.
    Success,
    /// The user clicked Cancel; the target carries `#error=user_cancelled`.
    Cancelled,
    /// The account is blocked; the target carries `#error=account_blocked`.
    Error,
}

impl Handoff {
    /// `(title, blurb)` for the card. The title doubles as the `<h1>`.
    fn copy(&self) -> (&'static str, &'static str) {
        match self {
            Handoff::Success => ("Authorized", "Returning you to chan-desktop\u{2026}"),
            Handoff::Cancelled => (
                "Request cancelled",
                "No token was issued. Returning you to chan-desktop\u{2026}",
            ),
            Handoff::Error => (
                "Sign-in failed",
                "Returning you to chan-desktop with the details\u{2026}",
            ),
        }
    }
}

/// Render the handoff page `confirm` answers with: a zero-delay meta
/// refresh to the `chan://` target plus a manual fallback link, so the
/// custom-scheme navigation never rides a form-POST redirect chain
/// (see the module doc). The target appears exactly twice, both times
/// attribute-escaped; its only user-influenced parts are percent-
/// encoded by [`success_url`] / [`error_url`].
fn render_handoff_html(kind: &Handoff, target: &str) -> String {
    let (title, blurb) = kind.copy();
    let url = pages::html_escape(target);
    let head_extra = format!("<meta http-equiv=\"refresh\" content=\"0;url={url}\">\n  ");
    let body = format!(
        r#"
    <span class="mark" aria-hidden="true"></span>
    <h1>{title}</h1>
    <p class="muted">{blurb}</p>
    <a class="btn primary" href="{url}">Open chan-desktop</a>
    <p class="muted small">You can close this tab.</p>
  "#,
    );
    pages::render(&pages::Page {
        title,
        head_extra: &head_extra,
        body: &body,
    })
}

/// The 200 response wrapping [`render_handoff_html`], with the shared
/// security headers (the page embeds a PAT secret on the success
/// path: `no-store`, `no-referrer`, `nosniff`).
fn handoff_response(kind: &Handoff, target: &str) -> Response {
    (
        pages::security_headers(),
        Html(render_handoff_html(kind, target)),
    )
        .into_response()
}

/// Mint the PAT, stash it under a one-time redemption code, and
/// return the chan:// success URL carrying the code. Called by the
/// confirm POST when the user clicks Authorize.
async fn complete(
    state: &AppState,
    headers: &HeaderMap,
    params: &AuthorizeParams,
    user: &User,
) -> Result<String> {
    let expires_at: DateTime<Utc> = Utc::now() + chrono::Duration::seconds(params.expires_in_secs);
    let CreatedToken { token, secret } = state
        .api_tokens
        .create(
            NewToken {
                user_id: user.id,
                label: &params.label,
                expires_at: Some(expires_at),
                scopes: &params.scopes,
                origin: TokenOrigin::Desktop,
            },
            &request_meta(headers),
        )
        .await
        .map_err(|e| {
            tracing::warn!(error = ?e, user = %user.username, "desktop authorize mint failed");
            e
        })?;
    // The PAT IS a devserver (1 token : 1 devserver) when it carries
    // the tunnel scope: register the roster row so the owner sees it
    // and can grant on it before it ever dials in, exactly like the
    // SPA mint. Best-effort: the row also auto-creates on first
    // grant, and the PAT is already persisted, so a profile hiccup
    // must not fail the mint.
    if params.scopes.iter().any(|s| s == "tunnel") {
        let devserver_id = crate::api_tokens::devserver_id_from_pat(&secret);
        if let Err(e) = state
            .cfg
            .profile_client
            .create_devserver(user.id, &devserver_id, &params.label)
            .await
        {
            tracing::warn!(
                error = ?e,
                user = %user.username,
                "register devserver after desktop PAT mint failed",
            );
        }
    }
    let code = state.desktop_redemptions.insert(RedeemPayload {
        id: token.id,
        secret,
        label: token.label.clone(),
        expires_at: token.expires_at,
    });
    Ok(success_url(params, &code, &token))
}

#[derive(Debug, Deserialize)]
pub struct RedeemRequest {
    code: String,
}

/// `POST /desktop/authorize/redeem` -- swap a one-time code for the
/// minted PAT. No session auth: possession of the code is the
/// credential (see the module doc's hardening notes). Unknown,
/// expired, and replayed codes share one 410 so an off-path caller
/// cannot probe code state.
pub async fn redeem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RedeemRequest>,
) -> Result<Json<RedeemPayload>> {
    let Some(payload) = state.desktop_redemptions.take(&req.code) else {
        return Err(Error::Gone(
            "unknown, expired, or already-redeemed code".into(),
        ));
    };
    state
        .api_tokens
        .write_audit(payload.id, ACTION_DESKTOP_REDEEM, &request_meta(&headers))
        .await?;
    Ok(Json(payload))
}

/// `GET /desktop/authorize` entry. Validates the query, stashes
/// params, and bounces -- to `/` for unauthenticated sessions (the SPA
/// renders sign-in), or straight to the consent page otherwise.
pub async fn authorize(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<AuthorizeQuery>,
) -> Result<Redirect> {
    let params = validate(q)?;
    session
        .insert(KEY_DESKTOP_AUTHORIZE, &params)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session insert desktop_authorize: {e}")))?;

    let Some(uid) = current_user_id_optional(&session).await? else {
        // Bounce through SPA sign-in. `auth_callback` redirects to
        // CONSENT_PATH once the user is authenticated.
        let _ = state;
        return Ok(Redirect::to("/"));
    };
    // Authenticated. Short-circuit a known-blocked user to the
    // chan:// error so the desktop client gets a precise reason
    // instead of staring at a 403.
    let user = state
        .cfg
        .profile_client
        .get_user(uid)
        .await?
        .ok_or(Error::Unauthorized)?;
    if user.is_blocked() {
        // Consume the stash; the user has decided nothing yet, but
        // there is no consent to render for a blocked account.
        let _ = take_pending(&session).await?;
        return Ok(Redirect::to(&error_url(&params, "account_blocked")));
    }
    Ok(Redirect::to(CONSENT_PATH))
}

/// `GET /desktop/authorize/consent` -- renders the consent HTML.
pub async fn consent(State(state): State<AppState>, session: Session) -> Result<Response> {
    let uid = current_user_id(&session).await?;
    let Some(params) = peek_pending(&session).await? else {
        return Err(Error::BadRequest("no pending desktop authorize".into()));
    };
    let user = state
        .cfg
        .profile_client
        .get_user(uid)
        .await?
        .ok_or(Error::Unauthorized)?;
    if user.is_blocked() {
        // Consume + redirect: a blocked user shouldn't see the
        // consent prompt.
        let _ = take_pending(&session).await?;
        return Ok(Redirect::to(&error_url(&params, "account_blocked")).into_response());
    }

    // Fresh CSRF on every render so a leaked nonce from a previous
    // page load cannot be replayed. Overwrites any prior value.
    let csrf = generate_csrf();
    session
        .insert(KEY_DESKTOP_CSRF, &csrf)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session insert desktop_csrf: {e}")))?;

    let html = render_consent_html(&params, &user, &csrf);
    Ok((pages::security_headers(), Html(html)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct ConfirmForm {
    /// `allow` or `deny`. Anything else is a 400.
    action: String,
    /// Echoed CSRF nonce. Compared constant-time to the session
    /// value stored during consent render.
    csrf: String,
}

/// `POST /desktop/authorize/confirm` -- handles allow / deny. Every
/// outcome answers 200 with a [`Handoff`] page (see the module doc
/// for why this is not a redirect).
pub async fn confirm(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Form(form): Form<ConfirmForm>,
) -> Result<Response> {
    let uid = current_user_id(&session).await?;

    // Consume CSRF first so a replay of an old form fails even if
    // params are still stashed.
    let expected_csrf: String = session
        .remove(KEY_DESKTOP_CSRF)
        .await
        .map_err(|e| Error::Anyhow(anyhow::anyhow!("session remove desktop_csrf: {e}")))?
        .ok_or_else(|| Error::BadRequest("csrf missing".into()))?;
    if !bool::from(form.csrf.as_bytes().ct_eq(expected_csrf.as_bytes())) {
        // Drop the pending stash on CSRF mismatch: an attacker who
        // knows the URL but not the nonce should not be able to keep
        // an authorize alive across attempts.
        let _ = take_pending(&session).await?;
        return Err(Error::BadRequest("csrf mismatch".into()));
    }

    let Some(params) = take_pending(&session).await? else {
        return Err(Error::BadRequest("no pending desktop authorize".into()));
    };

    match form.action.as_str() {
        "allow" => {
            let user = state
                .cfg
                .profile_client
                .get_user(uid)
                .await?
                .ok_or(Error::Unauthorized)?;
            if user.is_blocked() {
                return Ok(handoff_response(
                    &Handoff::Error,
                    &error_url(&params, "account_blocked"),
                ));
            }
            let url = complete(&state, &headers, &params, &user).await?;
            Ok(handoff_response(&Handoff::Success, &url))
        }
        "deny" => Ok(handoff_response(
            &Handoff::Cancelled,
            &error_url(&params, "user_cancelled"),
        )),
        _ => Err(Error::BadRequest("invalid action".into())),
    }
}

/// Render the consent page in the shared [`crate::pages`] shell (the
/// SPA card look). Every interpolated value is escaped; the form's
/// `csrf` / `action` fields are the wire contract the confirm POST
/// (and the integration tests) read. Script-free within the shell's
/// strict CSP.
fn render_consent_html(params: &AuthorizeParams, user: &User, csrf: &str) -> String {
    let display = user
        .display_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.username);
    let scopes = params.scopes.join(", ");
    let expires_phrase = humanize_expires(params.expires_in_secs);
    // Account-mode consent spells out what the single scope grants;
    // the legacy scopes keep the bare details table shipped desktops
    // were reviewed against.
    let account_blurb = if params.scopes.iter().any(|s| s == DESKTOP_ACCOUNT_SCOPE) {
        "<p class=\"muted\">chan-desktop will get access to your account on this \
         gateway: your devservers and devservers shared with you.</p>\n    "
    } else {
        ""
    };
    let body = format!(
        r#"
    <span class="mark" aria-hidden="true"></span>
    <h1>Authorize chan-desktop?</h1>
    <p class="muted">Signed in as <strong>{display}</strong>.</p>
    {account_blurb}<div class="details">
      <div class="row"><span class="k">Label</span><span class="v">{label}</span></div>
      <div class="row"><span class="k">Scopes</span><span class="v">{scopes}</span></div>
      <div class="row"><span class="k">Expires in</span><span class="v">{expires_phrase}</span></div>
    </div>
    <form method="post" action="/desktop/authorize/confirm">
      <input type="hidden" name="csrf" value="{csrf}">
      <button class="btn" type="submit" name="action" value="deny">Cancel</button>
      <button class="btn primary" type="submit" name="action" value="allow">Authorize</button>
    </form>
  "#,
        display = pages::html_escape(display),
        label = pages::html_escape(&params.label),
        scopes = pages::html_escape(&scopes),
        expires_phrase = pages::html_escape(&expires_phrase),
        csrf = pages::html_escape(csrf),
    );
    pages::render(&pages::Page {
        title: "Authorize chan-desktop",
        head_extra: "",
        body: &body,
    })
}

/// Best-effort coarse phrasing. "30 days", "2 hours" -- never tries
/// to mix units. Falls back to seconds for sub-minute values.
fn humanize_expires(secs: i64) -> String {
    const MIN: i64 = 60;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;
    if secs >= DAY {
        let d = secs / DAY;
        return format!("{d} day{}", if d == 1 { "" } else { "s" });
    }
    if secs >= HOUR {
        let h = secs / HOUR;
        return format!("{h} hour{}", if h == 1 { "" } else { "s" });
    }
    if secs >= MIN {
        let m = secs / MIN;
        return format!("{m} minute{}", if m == 1 { "" } else { "s" });
    }
    format!("{secs} second{}", if secs == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn params() -> AuthorizeParams {
        AuthorizeParams {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "abc xyz".into(),
            label: "chan-desktop @ box".into(),
            scopes: vec!["tunnel".into()],
            expires_in_secs: 30 * 86_400,
        }
    }

    fn dummy_token(id: Uuid, label: &str, expires_at: Option<DateTime<Utc>>) -> ApiToken {
        ApiToken {
            id,
            user_id: Uuid::new_v4(),
            label: label.into(),
            expires_at,
            created_at: Utc::now(),
            revoked_at: None,
            last_used_at: None,
            scopes: vec!["tunnel".into()],
        }
    }

    #[test]
    fn validates_minimal_query() {
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "chan-desktop".into(),
            scopes: None,
            expires_in: Some(2_592_000),
        };
        let p = validate(q).unwrap();
        assert_eq!(p.scopes, vec!["tunnel".to_string()]);
        assert_eq!(p.expires_in_secs, 2_592_000);
    }

    #[test]
    fn rejects_wrong_redirect_uri() {
        for bad in &[
            "https://attacker.example/cb",
            "chan://auth/callback/",
            "CHAN://auth/callback",
            "",
        ] {
            let q = AuthorizeQuery {
                redirect_uri: (*bad).into(),
                state: "nonce".into(),
                label: "x".into(),
                scopes: None,
                expires_in: Some(10),
            };
            assert!(validate(q).is_err(), "{bad} should reject");
        }
    }

    #[test]
    fn rejects_blank_or_oversized_state() {
        for bad in &["", "   ", &"x".repeat(MAX_STATE_LEN + 1)] {
            let q = AuthorizeQuery {
                redirect_uri: EXPECTED_REDIRECT_URI.into(),
                state: (*bad).into(),
                label: "x".into(),
                scopes: None,
                expires_in: Some(10),
            };
            assert!(validate(q).is_err());
        }
    }

    #[test]
    fn rejects_unknown_scope() {
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "x".into(),
            scopes: Some("tunnel,admin".into()),
            expires_in: Some(10),
        };
        assert!(validate(q).is_err());
    }

    #[test]
    fn accepts_sole_account_scope() {
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "x".into(),
            scopes: Some("desktop.account".into()),
            expires_in: Some(10),
        };
        let p = validate(q).unwrap();
        assert_eq!(p.scopes, vec!["desktop.account"]);
    }

    #[test]
    fn rejects_account_scope_mixed_with_others() {
        // desktop.account is sole-scope: any companion, even an
        // otherwise-allowed one, is a 400.
        for bad in [
            "desktop.account,tunnel",
            "tunnel,desktop.account",
            "desktop.account,desktop.connect",
        ] {
            let q = AuthorizeQuery {
                redirect_uri: EXPECTED_REDIRECT_URI.into(),
                state: "nonce".into(),
                label: "x".into(),
                scopes: Some((*bad).into()),
                expires_in: Some(10),
            };
            assert!(validate(q).is_err(), "{bad} should reject");
        }
    }

    #[test]
    fn legacy_scope_pairs_still_validate() {
        // Shipped desktops send tunnel and tunnel,desktop.connect;
        // both must keep working (Contract A back-compat).
        for ok in ["tunnel", "tunnel,desktop.connect", "desktop.connect"] {
            let q = AuthorizeQuery {
                redirect_uri: EXPECTED_REDIRECT_URI.into(),
                state: "nonce".into(),
                label: "x".into(),
                scopes: Some((*ok).into()),
                expires_in: Some(10),
            };
            assert!(validate(q).is_ok(), "{ok} should validate");
        }
    }

    #[test]
    fn accepts_csv_scopes_with_whitespace() {
        // Exercises comma-split + trim + empty-element filter. `tunnel`
        // is the only live scope now (public path dropped).
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "x".into(),
            scopes: Some(" tunnel , ".into()),
            expires_in: Some(10),
        };
        let p = validate(q).unwrap();
        assert_eq!(p.scopes, vec!["tunnel"]);
    }

    #[test]
    fn clamps_expires_in() {
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "x".into(),
            scopes: None,
            expires_in: Some(MAX_EXPIRES_IN_SECS * 10),
        };
        let p = validate(q).unwrap();
        assert_eq!(p.expires_in_secs, MAX_EXPIRES_IN_SECS);
    }

    #[test]
    fn rejects_non_positive_expires_in() {
        for n in [0, -1, -3600] {
            let q = AuthorizeQuery {
                redirect_uri: EXPECTED_REDIRECT_URI.into(),
                state: "nonce".into(),
                label: "x".into(),
                scopes: None,
                expires_in: Some(n),
            };
            assert!(validate(q).is_err());
        }
    }

    #[test]
    fn success_url_uses_fragment_and_encodes_specials() {
        let token = dummy_token(
            Uuid::nil(),
            "chan-desktop @ box",
            Some(Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap()),
        );
        let url = success_url(&params(), "the-code", &token);
        assert!(url.starts_with("chan://auth/callback#"), "got {url}");
        assert!(!url.contains('?'));
        assert!(url.contains("code=the-code"), "got {url}");
        assert!(url.contains("label=chan-desktop+%40+box"), "got {url}");
        assert!(url.contains("state=abc+xyz"), "got {url}");
        assert!(url.contains("expires_at=2030-01-02T03%3A04%3A05%2B00%3A00"));
    }

    #[test]
    fn success_url_never_carries_credentials() {
        // The one-time code REPLACES id + secret in the fragment; the
        // secret only ever leaves through the redeem response.
        let token = dummy_token(
            Uuid::nil(),
            "box",
            Some(Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap()),
        );
        let url = success_url(&params(), "the-code", &token);
        assert!(!url.contains("secret="), "got {url}");
        assert!(!url.contains("chan_pat_"), "got {url}");
        assert!(!url.contains("id="), "got {url}");
    }

    #[test]
    fn success_url_omits_expires_at_when_token_has_none() {
        let token = dummy_token(Uuid::nil(), "x", None);
        let url = success_url(&params(), "the-code", &token);
        assert!(!url.contains("expires_at="), "got {url}");
    }

    #[test]
    fn success_url_never_emits_devserver_keys() {
        // The devserver_* fragment keys are retired (Contract A); the
        // desktop's handle_callback tolerates their absence, and they
        // must never come back.
        let token = dummy_token(Uuid::nil(), "x", None);
        let url = success_url(&params(), "the-code", &token);
        assert!(!url.contains("devserver_"), "got {url}");
    }

    #[test]
    fn redemption_store_is_single_use() {
        let store = RedemptionStore::default();
        let code = store.insert(RedeemPayload {
            id: Uuid::nil(),
            secret: "chan_pat_AAAA".into(),
            label: "box".into(),
            expires_at: None,
        });
        let first = store.take(&code).expect("first take wins");
        assert_eq!(first.secret, "chan_pat_AAAA");
        assert!(store.take(&code).is_none(), "replay must miss");
        assert!(store.take("no-such-code").is_none());
    }

    #[test]
    fn redemption_store_expires_codes() {
        let store = RedemptionStore::default();
        let code = store.insert_with_ttl(
            RedeemPayload {
                id: Uuid::nil(),
                secret: "chan_pat_AAAA".into(),
                label: "box".into(),
                expires_at: None,
            },
            Duration::ZERO,
        );
        assert!(store.take(&code).is_none(), "expired take must miss");
        // The sweep also evicted the entry outright.
        assert!(store.inner.lock().unwrap().is_empty());
    }

    #[test]
    fn redeem_payload_serializes_null_expires_at() {
        // The desktop reads `expires_at` unconditionally: null, not
        // absent, for a token without an expiry.
        let j = serde_json::to_value(RedeemPayload {
            id: Uuid::nil(),
            secret: "chan_pat_AAAA".into(),
            label: "box".into(),
            expires_at: None,
        })
        .unwrap();
        assert!(j.get("expires_at").is_some_and(|v| v.is_null()), "{j}");
        assert_eq!(j["secret"], "chan_pat_AAAA");
        assert_eq!(j["id"], "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn error_url_carries_reason_and_state() {
        let url = error_url(&params(), "account_blocked");
        assert!(url.starts_with("chan://auth/callback#"));
        assert!(url.contains("error=account_blocked"));
        assert!(url.contains("state=abc+xyz"));
    }

    #[test]
    fn chan_urls_contain_no_html_attr_breakers() {
        // The property that makes embedding the URL in the handoff
        // page's attributes safe: byte_serialize percent-encodes every
        // attribute breaker, so the only entity the escaped URL can
        // contain is `&amp;`.
        let mut p = params();
        p.label = r#"a"b<c>'d e"#.into();
        p.state = r#""onmouseover='x' "#.into();
        let token = dummy_token(Uuid::nil(), &p.label, None);
        for url in [
            success_url(&p, "the-code", &token),
            error_url(&p, "user_cancelled"),
        ] {
            for breaker in ['"', '<', '>', '\'', ' '] {
                assert!(!url.contains(breaker), "{breaker:?} leaked into {url}");
            }
        }
    }

    #[test]
    fn handoff_html_embeds_target_twice_and_escapes() {
        let token = dummy_token(Uuid::nil(), "chan-desktop @ box", None);
        let url = success_url(&params(), "the-code", &token);
        let html = render_handoff_html(&Handoff::Success, &url);
        // Exactly twice: the meta refresh and the manual fallback link.
        let escaped = pages::html_escape(&url);
        assert_eq!(
            html.matches(&escaped).count(),
            2,
            "meta + link, got: {html}"
        );
        assert!(
            html.contains(&format!(
                "<meta http-equiv=\"refresh\" content=\"0;url={escaped}\">"
            )),
            "{html}"
        );
        assert!(
            html.contains(&format!("<a class=\"btn primary\" href=\"{escaped}\">")),
            "{html}"
        );
        assert!(html.contains("Open chan-desktop"), "{html}");
        assert!(html.contains("You can close this tab."), "{html}");
        assert!(html.contains("<h1>Authorized</h1>"), "{html}");
    }

    #[test]
    fn handoff_cancelled_variant_carries_error_url() {
        let url = error_url(&params(), "user_cancelled");
        let html = render_handoff_html(&Handoff::Cancelled, &url);
        assert!(html.contains("error=user_cancelled"), "{html}");
        assert!(html.contains("<h1>Request cancelled</h1>"), "{html}");
        assert!(html.contains("No token was issued."), "{html}");
    }

    #[test]
    fn consent_html_includes_required_fields_and_no_unescaped_input() {
        let mut p = params();
        // Hostile-shape inputs that would XSS without escaping.
        p.label = "<img src=x onerror=alert(1)>".into();
        p.state = r#""onclick=alert(1)//"#.into();
        let user = User {
            id: Uuid::nil(),
            email: "u@example.com".into(),
            display_name: Some("<b>Alice</b>".into()),
            username: "alice".into(),
            username_edits: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            blocked_at: None,
            block_reason: None,
            avatar_url: None,
        };
        let html = render_consent_html(&p, &user, "csrf-token");
        // CSRF appears as a hidden input.
        assert!(html.contains(r#"name="csrf" value="csrf-token""#), "{html}");
        // Two action buttons.
        assert!(html.contains(r#"name="action" value="allow""#));
        assert!(html.contains(r#"name="action" value="deny""#));
        // The shared shell renders the card + logo mark.
        assert!(html.contains(r#"class="mark""#), "{html}");
        // No raw <script>, <img onerror=, or unescaped quote in user fields.
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(html.contains("&lt;b&gt;Alice&lt;/b&gt;"));
        // The devserver picker is gone: no radios, ever.
        assert!(!html.contains(r#"name="devserver""#), "{html}");
        assert!(!html.contains(r#"type="radio""#), "{html}");
        // A tunnel-scoped request renders no account blurb.
        assert!(!html.contains("access to your account"), "{html}");
    }

    #[test]
    fn consent_html_account_scope_renders_the_account_copy() {
        let mut p = params();
        p.scopes = vec!["desktop.account".into()];
        let user = User {
            id: Uuid::nil(),
            email: "u@example.com".into(),
            display_name: None,
            username: "alice".into(),
            username_edits: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            blocked_at: None,
            block_reason: None,
            avatar_url: None,
        };
        let html = render_consent_html(&p, &user, "csrf-token");
        assert!(
            html.contains(
                "chan-desktop will get access to your account on this \
                 gateway: your devservers and devservers shared with you."
            ),
            "{html}"
        );
        assert!(!html.contains(r#"type="radio""#), "{html}");
        assert!(!html.contains(r#"name="devserver""#), "{html}");
    }

    #[test]
    fn humanize_picks_coarsest_unit() {
        assert_eq!(humanize_expires(30), "30 seconds");
        assert_eq!(humanize_expires(60), "1 minute");
        assert_eq!(humanize_expires(3600), "1 hour");
        assert_eq!(humanize_expires(86_400), "1 day");
        assert_eq!(humanize_expires(2 * 86_400), "2 days");
        assert_eq!(humanize_expires(30 * 86_400), "30 days");
    }
}
