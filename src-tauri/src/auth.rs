//! id.chan.app sign-in flow + keychain-backed PAT storage.
//!
//! Flow:
//!   1. Drive Manager calls `open_signin`. We generate a random
//!      state nonce, remember it in-process, and shell out to the
//!      user's default browser pointing at
//!      `https://id.chan.app/desktop/authorize?...&redirect_uri=chan://auth/callback&state=<nonce>`.
//!   2. id.chan.app handles OAuth (passkeys, autofill, all native to
//!      the user's real browser), mints a 30-day `tunnel` PAT, and
//!      302s to the redirect_uri with the PAT in the URL fragment.
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
/// successful callback or a sign-out). Drive Manager listens and
/// re-renders the toolbar button.
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

fn entry() -> Result<Entry, String> {
    Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).map_err(|e| format!("keychain entry: {e}"))
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

fn store(pat: &StoredPat) -> Result<(), String> {
    let json = serde_json::to_string(pat).map_err(|e| format!("encoding PAT: {e}"))?;
    entry()?
        .set_password(&json)
        .map_err(|e| format!("writing keychain: {e}"))
}

fn clear() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("clearing keychain: {e}")),
    }
}

/// Best-effort hostname for the PAT label. `hostname(1)` exists on
/// every supported target; fall back to a generic string so we never
/// fail the sign-in for cosmetic reasons.
pub fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "this machine".to_string())
}

/// In-flight authorization state. Set when `open_signin` launches a
/// browser; consumed and cleared by `handle_callback`. A second
/// `open_signin` while one is pending simply overwrites: the user
/// re-clicked, the prior browser tab is now stale, and only the
/// latest nonce will pass the callback check.
fn pending_state() -> &'static Mutex<Option<String>> {
    static PENDING: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

/// 128 bits of randomness, hex-encoded. Used as the OAuth-style
/// state nonce to bind the browser leg to the callback leg.
fn new_state() -> String {
    let mut buf = [0u8; 16];
    if getrandom::getrandom(&mut buf).is_err() {
        // getrandom failure is essentially impossible on the platforms
        // we ship to; if it does, fall back to time-based bytes so the
        // sign-in still completes (we lose CSRF protection but the
        // worst case is a stale chan:// callback being honored, which
        // requires the attacker to know our state shape).
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        buf[..16].copy_from_slice(&now.to_le_bytes());
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
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
    let state = new_state();
    *pending_state().lock().unwrap() = Some(state.clone());

    let label = format!("chan-desktop @ {}", hostname());
    let url = format!(
        "{AUTHORIZE_URL}?redirect_uri={redirect}&state={state}&label={label}&scopes={scopes}&expires_in={expires}",
        redirect = urlencode(REDIRECT_URI),
        state = urlencode(&state),
        label = urlencode(&label),
        scopes = urlencode(SCOPES),
        expires = EXPIRES_IN_SECONDS,
    );
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("opening browser: {e}"))?;
    Ok(())
}

/// Handle a `chan://auth/callback#...` URL delivered by the deep-link
/// plugin. Validates state, persists the PAT, and emits AUTH_CHANGED
/// on success or AUTH_ERROR on any failure.
pub fn handle_callback(app: &AppHandle, raw: &str) {
    if let Err(msg) = do_handle_callback(app, raw) {
        let _ = app.emit(AUTH_ERROR, &msg);
    }
}

fn do_handle_callback(app: &AppHandle, raw: &str) -> Result<(), String> {
    let url = Url::parse(raw).map_err(|e| format!("malformed callback URL: {e}"))?;
    // Only accept our exact path. Anything else is a confused redirect
    // or a maliciously crafted chan:// URL.
    if url.scheme() != "chan" || url.host_str() != Some("auth") || url.path() != "/callback" {
        return Err(format!("unexpected callback URL: {raw}"));
    }
    let fragment = url.fragment().unwrap_or("");
    let mut params = std::collections::HashMap::<String, String>::new();
    for pair in fragment.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(urldecode(k).to_string(), urldecode(v).to_string());
    }

    // Pop the in-flight state regardless of outcome — a failed leg
    // shouldn't leave a stale nonce around for a later callback to
    // replay.
    let expected = pending_state().lock().unwrap().take();

    if let Some(err) = params.get("error") {
        return Err(format!("sign-in cancelled: {err}"));
    }

    let got_state = params.get("state").cloned().unwrap_or_default();
    match expected {
        Some(s) if s == got_state => {}
        Some(_) => return Err("sign-in state mismatch (stale browser tab?)".into()),
        None => return Err("no sign-in in progress".into()),
    }

    let id = params.get("id").cloned().unwrap_or_default();
    let secret = params.get("secret").cloned().unwrap_or_default();
    let label = params
        .get("label")
        .cloned()
        .unwrap_or_else(|| format!("chan-desktop @ {}", hostname()));
    let expires_at = params.get("expires_at").cloned().unwrap_or_default();
    if id.is_empty() || secret.is_empty() {
        return Err("callback missing id or secret".into());
    }

    let pat = StoredPat {
        id,
        secret,
        label: label.clone(),
        expires_at: expires_at.clone(),
    };
    store(&pat)?;
    let status = AuthStatus {
        is_signed_in: true,
        label: Some(label),
        expires_at: if expires_at.is_empty() {
            None
        } else {
            Some(expires_at)
        },
    };
    let _ = app.emit(AUTH_CHANGED, &status);
    Ok(())
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

/// Minimal RFC3986-ish percent encoder for the small set of chars we
/// pass through the authorize URL (`@`, ` `, `:`, `/`). Avoids pulling
/// the full `percent-encoding` crate.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        let is_unreserved = matches!(b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
