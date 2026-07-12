//! id.chan.app sign-in flow + keychain-backed PAT storage.
//!
//! Flow:
//!   1. The Workspaces window calls `open_signin`. We generate a random
//!      state nonce, remember it in-process, and shell out to the
//!      user's default browser pointing at
//!      `https://id.chan.app/desktop/authorize?...&redirect_uri=chan://auth/callback&state=<nonce>`.
//!   2. id.chan.app handles OAuth (passkeys, autofill, all native to
//!      the user's real browser), mints a 30-day `tunnel` PAT, and
//!      serves a handoff page that navigates to the redirect_uri
//!      (zero-delay meta refresh, with a manual fallback link) with
//!      the PAT in the URL fragment.
//!   3. macOS routes the `chan://` URL to chan-desktop. The deep-link
//!      plugin invokes `handle_callback`, which validates the state
//!      nonce, persists the PAT to the OS keychain, and emits
//!      `auth-changed`. Errors emit `auth-error` with a string body.
//!
//! Fragments, not query: the secret lives in `#…` so it never appears
//! in any intermediate http log or referer. The browser hands the
//! whole URL (fragment included) to the OS URL handler.
//!
//! Keychain layout: service `chan-desktop`, account `id.chan.app`.
//! Value is JSON `{id, secret, label, expires_at}` so sign-out can
//! both clear locally and surface the token id for a future
//! server-side revoke pass.
//!
//! v1 sign-out is local-only: we drop the keychain entry. Server-
//! side revoke needs the id.chan.app session, which only the user's
//! browser has — wiring that is a follow-up.

use std::sync::{Mutex, OnceLock};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Url};
use tauri_plugin_opener::OpenerExt;

/// Event emitted whenever the local sign-in state changes (after a
/// successful callback or a sign-out). The Workspaces window listens
/// and re-renders the toolbar button.
pub const AUTH_CHANGED: &str = "auth-changed";

/// Event emitted when the callback fails (state mismatch, missing
/// fields, id.chan.app returned `error=...`). Body is a human
/// string suitable for a banner.
pub const AUTH_ERROR: &str = "auth-error";

const KEYCHAIN_SERVICE: &str = "chan-desktop";
const KEYCHAIN_ACCOUNT: &str = "id.chan.app";
const AUTHORIZE_URL: &str = "https://id.chan.app/desktop/authorize";
const REDIRECT_URI: &str = "chan://auth/callback";
const SCOPES: &str = "tunnel";
const DESKTOP_CONNECT_SCOPES: &str = "desktop.connect";
const EXPIRES_IN_SECONDS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPat {
    pub id: String,
    pub secret: String,
    pub label: String,
    /// RFC3339 timestamp, or empty when the token never expires.
    #[serde(default)]
    pub expires_at: String,
}

/// Public view of the sign-in state. `secret` is intentionally never
/// surfaced to the frontend — only `is_signed_in` and the metadata
/// the user can use to decide whether to re-mint.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub is_signed_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

fn entry_for(account: &str) -> Result<Entry, String> {
    Entry::new(KEYCHAIN_SERVICE, account).map_err(|e| format!("keychain entry: {e}"))
}

fn entry() -> Result<Entry, String> {
    entry_for(KEYCHAIN_ACCOUNT)
}

fn load() -> Result<Option<StoredPat>, String> {
    match entry()?.get_password() {
        Ok(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| format!("decoding stored PAT: {e}")),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("reading keychain: {e}")),
    }
}

fn store_for(account: &str, pat: &StoredPat) -> Result<(), String> {
    let json = serde_json::to_string(pat).map_err(|e| format!("encoding PAT: {e}"))?;
    entry_for(account)?
        .set_password(&json)
        .map_err(|e| format!("writing keychain: {e}"))
}

fn clear() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("clearing keychain: {e}")),
    }
}

/// Best-effort hostname for the PAT label. Fall back to a generic
/// string so we never fail sign-in for cosmetic reasons.
pub fn hostname() -> String {
    gethostname::gethostname()
        .into_string()
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "this machine".to_string())
}

/// In-flight authorization state. Set when `open_signin` launches a
/// browser; consumed and cleared by `handle_callback`. A second
/// `open_signin` while one is pending simply overwrites: the user
/// re-clicked, the prior browser tab is now stale, and only the
/// latest nonce will pass the callback check.
#[derive(Debug, Clone)]
struct PendingAuth {
    state: String,
    account: String,
    resume_devserver_id: Option<String>,
}

fn pending_state() -> &'static Mutex<Option<PendingAuth>> {
    static PENDING: OnceLock<Mutex<Option<PendingAuth>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

/// 128 bits of randomness, hex-encoded. Used as the OAuth-style
/// state nonce to bind the browser leg to the callback leg.
fn new_state() -> Result<String, String> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).map_err(|e| format!("CSPRNG unavailable: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

#[tauri::command]
pub fn auth_status() -> AuthStatus {
    match load() {
        Ok(Some(pat)) => AuthStatus {
            is_signed_in: true,
            label: Some(pat.label),
            expires_at: if pat.expires_at.is_empty() {
                None
            } else {
                Some(pat.expires_at)
            },
        },
        _ => AuthStatus {
            is_signed_in: false,
            label: None,
            expires_at: None,
        },
    }
}

/// Open id.chan.app/desktop/authorize in the user's default browser.
/// Short-circuits when already signed in.
#[tauri::command]
pub fn open_signin(app: AppHandle) -> Result<(), String> {
    if load().ok().flatten().is_some() {
        return Ok(());
    }
    let state = new_state()?;
    *pending_state().lock().unwrap() = Some(PendingAuth {
        state: state.clone(),
        account: KEYCHAIN_ACCOUNT.to_string(),
        resume_devserver_id: None,
    });

    let label = format!("chan-desktop @ {}", hostname());
    let url = url::Url::parse_with_params(
        AUTHORIZE_URL,
        &[
            ("redirect_uri", REDIRECT_URI.to_string()),
            ("state", state),
            ("label", label),
            ("scopes", SCOPES.to_string()),
            ("expires_in", EXPIRES_IN_SECONDS.to_string()),
        ],
    )
    .map_err(|e| format!("building authorize URL: {e}"))?;
    app.opener()
        .open_url(url.to_string(), None::<&str>)
        .map_err(|e| format!("opening browser: {e}"))?;
    Ok(())
}

pub fn gateway_account(identity_origin: &str) -> String {
    format!("gateway:{identity_origin}")
}

pub fn load_gateway_pat(identity_origin: &str) -> Result<Option<StoredPat>, String> {
    let account = gateway_account(identity_origin);
    match entry_for(&account)?.get_password() {
        Ok(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| format!("decoding stored gateway PAT: {e}")),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("reading gateway keychain: {e}")),
    }
}

/// Drop the stored PAT for a gateway. The connect flow calls this when the
/// gateway answers 401 (the PAT was revoked or expired server-side), so the
/// next connect attempt falls into the browser sign-in instead of replaying
/// a dead credential.
pub fn clear_gateway_pat(identity_origin: &str) -> Result<(), String> {
    let account = gateway_account(identity_origin);
    match entry_for(&account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("clearing gateway keychain: {e}")),
    }
}

pub fn open_gateway_signin(
    app: &AppHandle,
    identity_origin: &str,
    authorize_url: &str,
    devserver_id: &str,
) -> Result<(), String> {
    let state = new_state()?;
    *pending_state().lock().unwrap() = Some(PendingAuth {
        state: state.clone(),
        account: gateway_account(identity_origin),
        resume_devserver_id: Some(devserver_id.to_string()),
    });
    let label = format!("chan-desktop @ {}", hostname());
    let url = url::Url::parse_with_params(
        authorize_url,
        &[
            ("redirect_uri", REDIRECT_URI.to_string()),
            ("state", state),
            ("label", label),
            ("scopes", DESKTOP_CONNECT_SCOPES.to_string()),
            ("expires_in", EXPIRES_IN_SECONDS.to_string()),
        ],
    )
    .map_err(|e| format!("building gateway authorize URL: {e}"))?;
    app.opener()
        .open_url(url.to_string(), None::<&str>)
        .map_err(|e| format!("opening browser: {e}"))?;
    Ok(())
}

/// What a `chan://auth/callback` delivery amounted to, so the caller can
/// resume a devserver connect or clear waiting rows. `PendingAuth` is a
/// single slot popped by any processed callback, so a consumed failure means
/// NO waiting sign-in can complete anymore (its browser leg lost the nonce),
/// not just the one this callback belonged to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackOutcome {
    /// The PAT was stored and AUTH_CHANGED emitted. `resume_devserver_id`
    /// names the devserver whose connect launched this sign-in, if any.
    SignedIn { resume_devserver_id: Option<String> },
    /// The callback failed and AUTH_ERROR was emitted. `consumed_pending` is
    /// true when the failure popped the in-flight sign-in state; false means
    /// the URL itself was malformed and any waiting sign-in is still live.
    Failed { consumed_pending: bool },
    /// A well-formed callback arrived with no sign-in pending: the handoff
    /// page's fallback link re-clicked after the meta refresh already
    /// delivered, or a stray delivery. Nothing stored, nothing emitted.
    Ignored,
}

/// Handle a `chan://auth/callback#...` URL delivered by the deep-link
/// plugin. Validates state, persists the PAT, and emits AUTH_CHANGED
/// on success or AUTH_ERROR on failure; a duplicate delivery for a
/// sign-in that already settled is ignored without a banner.
pub fn handle_callback(app: &AppHandle, raw: &str) -> CallbackOutcome {
    let action = {
        let mut pending = pending_state().lock().unwrap();
        classify_callback(raw, &mut pending)
    };
    match action {
        CallbackAction::SignIn {
            pat,
            account,
            resume_devserver_id,
        } => {
            if let Err(e) = store_for(&account, &pat) {
                let _ = app.emit(AUTH_ERROR, &e);
                return CallbackOutcome::Failed {
                    consumed_pending: true,
                };
            }
            let status = AuthStatus {
                is_signed_in: true,
                label: Some(pat.label),
                expires_at: if pat.expires_at.is_empty() {
                    None
                } else {
                    Some(pat.expires_at)
                },
            };
            let _ = app.emit(AUTH_CHANGED, &status);
            CallbackOutcome::SignedIn {
                resume_devserver_id,
            }
        }
        CallbackAction::Fail {
            message,
            consumed_pending,
        } => {
            let _ = app.emit(AUTH_ERROR, &message);
            CallbackOutcome::Failed { consumed_pending }
        }
        CallbackAction::Ignore => CallbackOutcome::Ignored,
    }
}

/// Map an id.chan.app `#error=` reason token to the banner string. The
/// tokens are the gateway's stable desktop-authorize vocabulary; the human
/// strings live here on the desktop so gateway deploys never reword the UI.
fn signin_error_message(reason: &str) -> String {
    match reason {
        "user_cancelled" => "sign-in was cancelled in the browser".to_string(),
        "oauth_denied" => "sign-in was denied by the identity provider".to_string(),
        "account_blocked" => "sign-in failed: this account is blocked".to_string(),
        "mint_failed" => "sign-in failed: the gateway could not issue an access token".to_string(),
        other => format!("sign-in failed in the browser: {other}"),
    }
}

/// What [`handle_callback`] should do with a delivered URL, decided
/// before any I/O so the validation and dedup rules are unit-testable.
#[derive(Debug)]
enum CallbackAction {
    /// Store the PAT and emit AUTH_CHANGED.
    SignIn {
        pat: StoredPat,
        account: String,
        resume_devserver_id: Option<String>,
    },
    /// Emit AUTH_ERROR. `consumed_pending` as on [`CallbackOutcome::Failed`].
    Fail {
        message: String,
        consumed_pending: bool,
    },
    /// A well-formed callback with no sign-in pending (the handoff page's
    /// fallback link after the meta refresh already delivered, or a stray
    /// delivery). Nothing to store or emit.
    Ignore,
}

/// Classify a delivered URL against the pending sign-in slot. Pops the
/// slot only once the URL is a well-formed `chan://auth/callback` -- a
/// malformed delivery must not kill a live browser leg.
fn classify_callback(raw: &str, pending: &mut Option<PendingAuth>) -> CallbackAction {
    let fail = |message: String, consumed_pending: bool| CallbackAction::Fail {
        message,
        consumed_pending,
    };
    let url = match Url::parse(raw) {
        Ok(u) => u,
        Err(e) => return fail(format!("malformed callback URL: {e}"), false),
    };
    // Only accept our exact path. Anything else is a confused redirect
    // or a maliciously crafted chan:// URL.
    if url.scheme() != "chan" || url.host_str() != Some("auth") || url.path() != "/callback" {
        return fail(format!("unexpected callback URL: {raw}"), false);
    }
    let fragment = url.fragment().unwrap_or("");
    let params: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(fragment.as_bytes())
            .into_owned()
            .collect();

    // Pop the in-flight state regardless of outcome -- a failed leg
    // shouldn't leave a stale nonce around for a later callback to
    // replay. With nothing pending, the sign-in already settled (or never
    // existed): ignore the delivery rather than banner over a success.
    let Some(expected) = pending.take() else {
        return CallbackAction::Ignore;
    };

    if let Some(err) = params.get("error") {
        return fail(signin_error_message(err), true);
    }

    let got_state = params.get("state").cloned().unwrap_or_default();
    if expected.state != got_state {
        return fail(
            "sign-in state mismatch (stale browser tab?)".to_string(),
            true,
        );
    }

    let id = params.get("id").cloned().unwrap_or_default();
    let secret = params.get("secret").cloned().unwrap_or_default();
    let label = params
        .get("label")
        .cloned()
        .unwrap_or_else(|| format!("chan-desktop @ {}", hostname()));
    let expires_at = params.get("expires_at").cloned().unwrap_or_default();
    if id.is_empty() || secret.is_empty() {
        return fail("callback missing id or secret".to_string(), true);
    }

    CallbackAction::SignIn {
        pat: StoredPat {
            id,
            secret,
            label,
            expires_at,
        },
        account: expected.account,
        resume_devserver_id: expected.resume_devserver_id,
    }
}

/// Local sign-out. Clears the keychain entry. Server-side revoke is
/// a follow-up — it needs the id.chan.app session which only the
/// user's browser has access to.
#[tauri::command]
pub fn signout(app: AppHandle) -> Result<AuthStatus, String> {
    clear()?;
    let status = AuthStatus {
        is_signed_in: false,
        label: None,
        expires_at: None,
    };
    let _ = app.emit(AUTH_CHANGED, &status);
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signin_error_message_maps_the_authorize_reason_tokens() {
        // The gateway's stable `#error=` vocabulary (desktop_authorize):
        // banner strings live desktop-side, one per token, and an unknown
        // token still names itself so a future reason is never swallowed.
        assert_eq!(
            signin_error_message("user_cancelled"),
            "sign-in was cancelled in the browser"
        );
        assert_eq!(
            signin_error_message("oauth_denied"),
            "sign-in was denied by the identity provider"
        );
        assert_eq!(
            signin_error_message("account_blocked"),
            "sign-in failed: this account is blocked"
        );
        assert_eq!(
            signin_error_message("mint_failed"),
            "sign-in failed: the gateway could not issue an access token"
        );
        assert_eq!(
            signin_error_message("quota_exceeded"),
            "sign-in failed in the browser: quota_exceeded"
        );
    }

    fn pending(state: &str) -> Option<PendingAuth> {
        Some(PendingAuth {
            state: state.to_string(),
            account: "id.chan.app".to_string(),
            resume_devserver_id: Some("ds-1".to_string()),
        })
    }

    #[test]
    fn classify_signs_in_on_a_matching_pending_state() {
        let mut slot = pending("nonce-1");
        let action = classify_callback(
            "chan://auth/callback#id=t-1&secret=chan_pat_x&label=mbp&state=nonce-1",
            &mut slot,
        );
        match action {
            CallbackAction::SignIn {
                pat,
                account,
                resume_devserver_id,
            } => {
                assert_eq!(pat.id, "t-1");
                assert_eq!(pat.secret, "chan_pat_x");
                assert_eq!(pat.label, "mbp");
                assert_eq!(account, "id.chan.app");
                assert_eq!(resume_devserver_id.as_deref(), Some("ds-1"));
            }
            other => panic!("expected SignIn, got {other:?}"),
        }
        assert!(slot.is_none(), "the pending slot is consumed");
    }

    #[test]
    fn classify_ignores_duplicates_with_nothing_pending() {
        // The handoff page keeps a live "Open chan-desktop" link after its
        // meta refresh already delivered; a re-click must not banner over
        // the completed sign-in. Same for a duplicate deny.
        let mut slot = None;
        for raw in [
            "chan://auth/callback#id=t-1&secret=chan_pat_x&state=nonce-1",
            "chan://auth/callback#error=user_cancelled&state=nonce-1",
        ] {
            assert!(
                matches!(classify_callback(raw, &mut slot), CallbackAction::Ignore),
                "{raw} should be ignored with no pending sign-in"
            );
        }
    }

    #[test]
    fn classify_surfaces_a_deny_that_consumes_the_pending_leg() {
        let mut slot = pending("nonce-1");
        match classify_callback(
            "chan://auth/callback#error=user_cancelled&state=nonce-1",
            &mut slot,
        ) {
            CallbackAction::Fail {
                message,
                consumed_pending,
            } => {
                assert_eq!(message, "sign-in was cancelled in the browser");
                assert!(consumed_pending);
            }
            other => panic!("expected Fail, got {other:?}"),
        }
        assert!(slot.is_none());
    }

    #[test]
    fn classify_keeps_the_pending_leg_alive_on_a_malformed_url() {
        // A malformed or foreign URL must not kill a live browser leg: the
        // slot stays untouched so the real callback can still complete.
        let mut slot = pending("nonce-1");
        for raw in ["not a url", "chan://evil/callback#id=x&secret=y"] {
            match classify_callback(raw, &mut slot) {
                CallbackAction::Fail {
                    consumed_pending, ..
                } => assert!(!consumed_pending, "{raw}"),
                other => panic!("expected Fail for {raw}, got {other:?}"),
            }
            assert!(slot.is_some(), "pending survives {raw}");
        }
    }

    #[test]
    fn classify_rejects_a_state_mismatch_and_missing_credentials() {
        let mut slot = pending("nonce-1");
        match classify_callback("chan://auth/callback#id=t&secret=s&state=WRONG", &mut slot) {
            CallbackAction::Fail {
                message,
                consumed_pending,
            } => {
                assert!(message.contains("state mismatch"), "{message}");
                assert!(consumed_pending);
            }
            other => panic!("expected Fail, got {other:?}"),
        }

        let mut slot = pending("nonce-1");
        match classify_callback("chan://auth/callback#state=nonce-1", &mut slot) {
            CallbackAction::Fail {
                message,
                consumed_pending,
            } => {
                assert!(message.contains("missing id or secret"), "{message}");
                assert!(consumed_pending);
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }
}
