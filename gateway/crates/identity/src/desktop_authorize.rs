//! OAuth-style PAT mint for chan-desktop.
//!
//! Three routes:
//!
//!   * `GET  /desktop/authorize?<query>` — entry point. Validates the
//!     query, stashes a [`AuthorizeParams`] struct in the session, and
//!     redirects: to `/` if unauthenticated (SPA sign-in renders, then
//!     `auth_callback` bounces back here), or straight to
//!     `/desktop/authorize/consent` if authenticated.
//!   * `GET  /desktop/authorize/consent` — renders a server-side HTML
//!     consent page showing the requesting client, label, scopes, and
//!     expiry. Includes a hidden CSRF nonce stored alongside the
//!     pending params. Responds with `X-Frame-Options: DENY` and CSP
//!     `frame-ancestors 'none'` so a malicious page cannot iframe the
//!     consent and trick a user into approving via clickjack.
//!   * `POST /desktop/authorize/confirm` — handles the `Authorize` /
//!     `Cancel` action. Consumes the pending params + CSRF; on
//!     `allow` mints a PAT through [`ApiTokenService::create`] with
//!     [`TokenOrigin::Desktop`] and 302s to
//!     `chan://auth/callback#id=&secret=&label=&expires_at=&state=`
//!     (fragment, not query: secrets never reach access logs along
//!     the redirect chain). On `deny`, 302s to
//!     `chan://...#error=user_cancelled&state=`.
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
//!     (not `created`) so operators and users can tell the desktop
//!     flow apart from SPA mints.
//!
//! Follow-ups (not in v1):
//!   * Per-session rate limit. Today a signed-in user spam-clicking
//!     `Authorize` mints PATs into their own table; audit-visible and
//!     bounded by the user's own account, so we ship without a
//!     server-side limit and watch the audit log.

use axum::extract::{Form, Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use axum::response::{Html, IntoResponse, Redirect, Response};
use base64::Engine;
use chrono::{DateTime, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tower_sessions::Session;
use url::form_urlencoded::byte_serialize;

use crate::api_tokens::{ApiToken, CreatedToken, TokenOrigin};
use crate::error::{Error, Result};
use crate::http::{client_ip, current_user_id, current_user_id_optional, user_agent, AppState};
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
/// the chan-tunnel-server vocabulary entries, so a desktop build
/// cannot mint a token carrying a typo'd or future-only scope.
const ALLOWED_SCOPES: &[&str] = &["tunnel", "tunnel.public"];

/// Default when the client omits `scopes`. Matches the SPA / general
/// PAT default so silence means "private tunnel only".
const DEFAULT_SCOPES: &[&str] = &["tunnel"];

/// Path the consent page lives at. Exported so other modules (today
/// `auth_callback`) can build a redirect without restating the literal.
pub const CONSENT_PATH: &str = "/desktop/authorize/consent";

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

/// Build the success redirect target. Secret rides in the URL
/// fragment so it does not appear in any access log along the
/// redirect chain (browsers do not send the fragment to servers).
pub fn success_url(params: &AuthorizeParams, token: &ApiToken, secret: &str) -> String {
    let mut frag = String::new();
    push_pair(&mut frag, "id", &token.id.to_string());
    push_pair(&mut frag, "secret", secret);
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

/// Mint the PAT and return the chan:// success URL. Called by the
/// confirm POST when the user clicks Authorize.
async fn complete(
    state: &AppState,
    headers: &HeaderMap,
    params: &AuthorizeParams,
    user: &User,
) -> Result<Redirect> {
    let expires_at: DateTime<Utc> = Utc::now() + chrono::Duration::seconds(params.expires_in_secs);
    let ip = client_ip(headers);
    let ua = user_agent(headers);
    let CreatedToken { token, secret } = state
        .api_tokens
        .create(
            user.id,
            &params.label,
            Some(expires_at),
            &params.scopes,
            ip.as_deref(),
            ua.as_deref(),
            TokenOrigin::Desktop,
        )
        .await
        .map_err(|e| {
            tracing::warn!(error = ?e, user = %user.username, "desktop authorize mint failed");
            e
        })?;
    let url = success_url(params, &token, &secret);
    Ok(Redirect::to(&url))
}

/// `GET /desktop/authorize` entry. Validates the query, stashes
/// params, and bounces — to `/` for unauthenticated sessions (the SPA
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

/// `GET /desktop/authorize/consent` — renders the consent HTML.
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
    let headers = [
        // Clickjacking belt: block all framing of the consent page.
        // X-Frame-Options is legacy but still respected by older
        // browsers; the CSP covers modern ones.
        (
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ),
        (
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'none'; style-src 'unsafe-inline'; \
                 form-action 'self'; frame-ancestors 'none'",
            ),
        ),
        // The CSRF nonce is in the page; never let an intermediate
        // cache hand it to another user.
        (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        (
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ),
    ];
    Ok((headers, Html(html)).into_response())
}

#[derive(Debug, Deserialize)]
pub struct ConfirmForm {
    /// `allow` or `deny`. Anything else is a 400.
    action: String,
    /// Echoed CSRF nonce. Compared constant-time to the session
    /// value stored during consent render.
    csrf: String,
}

/// `POST /desktop/authorize/confirm` — handles allow / deny.
pub async fn confirm(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Form(form): Form<ConfirmForm>,
) -> Result<Redirect> {
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
                return Ok(Redirect::to(&error_url(&params, "account_blocked")));
            }
            complete(&state, &headers, &params, &user).await
        }
        "deny" => Ok(Redirect::to(&error_url(&params, "user_cancelled"))),
        _ => Err(Error::BadRequest("invalid action".into())),
    }
}

/// Minimal HTML escape: covers the five characters that matter for
/// attribute + text contexts. We never render unescaped user input
/// into a `<script>` or `style` block, so this list is sufficient.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render the consent page. Inline CSS keeps the page self-contained
/// (CSP blocks external resources via `default-src 'none'`).
fn render_consent_html(params: &AuthorizeParams, user: &User, csrf: &str) -> String {
    let display = user
        .display_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&user.username);
    let scopes = params.scopes.join(", ");
    let expires_phrase = humanize_expires(params.expires_in_secs);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Authorize chan-desktop</title>
  <style>
    body {{ font: 16px/1.5 system-ui, sans-serif; max-width: 32rem;
            margin: 4rem auto; padding: 0 1rem; color: #111; }}
    h1   {{ font-size: 1.4rem; margin: 0 0 1rem; }}
    .who {{ color: #555; margin-bottom: 1.25rem; }}
    .card {{ border: 1px solid #ddd; border-radius: 6px;
             padding: 1rem 1.25rem; margin-bottom: 1.5rem; }}
    .row  {{ display: flex; justify-content: space-between; gap: 1rem;
             margin: 0.25rem 0; }}
    .k    {{ color: #555; }}
    .v    {{ font-weight: 600; word-break: break-word; }}
    form  {{ display: flex; gap: 0.75rem; }}
    button {{ flex: 1; padding: 0.6rem 1rem; font: inherit;
              border-radius: 6px; cursor: pointer; border: 1px solid #ccc; }}
    button[name=action][value=allow] {{ background: #111; color: #fff;
                                         border-color: #111; }}
  </style>
</head>
<body>
  <main>
    <h1>Authorize chan-desktop?</h1>
    <p class="who">Signed in as <strong>{display}</strong>.</p>
    <div class="card">
      <div class="row"><span class="k">Label</span><span class="v">{label}</span></div>
      <div class="row"><span class="k">Scopes</span><span class="v">{scopes}</span></div>
      <div class="row"><span class="k">Expires in</span><span class="v">{expires_phrase}</span></div>
    </div>
    <form method="post" action="/desktop/authorize/confirm">
      <input type="hidden" name="csrf" value="{csrf}">
      <button type="submit" name="action" value="deny">Cancel</button>
      <button type="submit" name="action" value="allow">Authorize</button>
    </form>
  </main>
</body>
</html>
"#,
        display = html_escape(display),
        label = html_escape(&params.label),
        scopes = html_escape(&scopes),
        expires_phrase = html_escape(&expires_phrase),
        csrf = html_escape(csrf),
    )
}

/// Best-effort coarse phrasing. "30 days", "2 hours" — never tries
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
    fn accepts_csv_scopes_with_whitespace() {
        let q = AuthorizeQuery {
            redirect_uri: EXPECTED_REDIRECT_URI.into(),
            state: "nonce".into(),
            label: "x".into(),
            scopes: Some(" tunnel , tunnel.public ".into()),
            expires_in: Some(10),
        };
        let p = validate(q).unwrap();
        assert_eq!(p.scopes, vec!["tunnel", "tunnel.public"]);
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
        let url = success_url(&params(), &token, "chan_pat_AAAA");
        assert!(url.starts_with("chan://auth/callback#"), "got {url}");
        assert!(!url.contains('?'));
        assert!(url.contains("label=chan-desktop+%40+box"), "got {url}");
        assert!(url.contains("state=abc+xyz"), "got {url}");
        assert!(url.contains("expires_at=2030-01-02T03%3A04%3A05%2B00%3A00"));
        assert!(url.contains("secret=chan_pat_AAAA"));
        assert!(url.contains("id=00000000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn success_url_omits_expires_at_when_token_has_none() {
        let token = dummy_token(Uuid::nil(), "x", None);
        let url = success_url(&params(), &token, "chan_pat_AAAA");
        assert!(!url.contains("expires_at="), "got {url}");
    }

    #[test]
    fn error_url_carries_reason_and_state() {
        let url = error_url(&params(), "account_blocked");
        assert!(url.starts_with("chan://auth/callback#"));
        assert!(url.contains("error=account_blocked"));
        assert!(url.contains("state=abc+xyz"));
    }

    #[test]
    fn html_escape_covers_attr_breakers() {
        let in_ = r#"<script>alert("xss & ' end")</script>"#;
        let out = html_escape(in_);
        assert_eq!(
            out,
            "&lt;script&gt;alert(&quot;xss &amp; &#39; end&quot;)&lt;/script&gt;"
        );
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
        // No raw <script>, <img onerror=, or unescaped quote in user fields.
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(html.contains("&lt;b&gt;Alice&lt;/b&gt;"));
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
