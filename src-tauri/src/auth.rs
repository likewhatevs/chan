//! id.chan.app sign-in flow + keychain-backed PAT storage.
//!
//! Flow:
//!   1. Drive Manager calls `open_signin`. We open a Tauri webview to
//!      https://id.chan.app/. The webview carries its own cookie jar,
//!      so subsequent visits skip OAuth if the user's session is
//!      still live.
//!   2. The injected init script polls `/api/me` after every
//!      navigation; on the first 200 it POSTs `/api/tokens` with a
//!      30-day `tunnel` PAT labelled `chan-desktop @ <hostname>`,
//!      hands the result to Rust via `save_pat`, and the auth window
//!      closes itself.
//!   3. The PAT secret is the token chan-tunnel-server accepts at
//!      connection time. chan-desktop never calls id.chan.app with
//!      the PAT — it only stores it and lets chan use it.
//!
//! Keychain layout: service `chan-desktop`, account `id.chan.app`.
//! Value is JSON `{id, secret, label, expires_at}` so sign-out can
//! both clear locally and surface the token id for a future
//! server-side revoke pass.
//!
//! v1 sign-out is local-only: we drop the keychain entry. Server-
//! side revoke needs the `id_session` cookie, which only the
//! webview has — wiring that is a follow-up.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Event emitted whenever the local sign-in state changes (after a
/// successful PAT mint or a sign-out). Drive Manager listens and
/// re-renders the toolbar button.
pub const AUTH_CHANGED: &str = "auth-changed";

const KEYCHAIN_SERVICE: &str = "chan-desktop";
const KEYCHAIN_ACCOUNT: &str = "id.chan.app";
const AUTH_WINDOW_LABEL: &str = "auth";
const AUTH_URL: &str = "https://id.chan.app/";

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

/// Open the id.chan.app sign-in webview. Reuses the existing window
/// if the user already clicked Sign In and the auth flow is in
/// progress. The webview lives until either `save_pat` closes it on
/// success or the user closes it manually.
///
/// `async` is load-bearing: Tauri 2 dispatches sync commands on the
/// main thread, and building a webview from inside that path races
/// the IPC reply. An async command runs on the async runtime and
/// lets Tauri hop to the main thread internally to build the window.
#[tauri::command]
pub async fn open_signin(app: AppHandle) -> Result<(), String> {
    if load().ok().flatten().is_some() {
        return Ok(());
    }
    if let Some(w) = app.get_webview_window(AUTH_WINDOW_LABEL) {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let url = AUTH_URL
        .parse::<tauri::Url>()
        .map_err(|e| format!("bad auth URL: {e}"))?;
    let init = signin_init_script(&hostname());
    WebviewWindowBuilder::new(&app, AUTH_WINDOW_LABEL, WebviewUrl::External(url))
        .title("Sign in to chan")
        .inner_size(520.0, 680.0)
        .min_inner_size(420.0, 520.0)
        .resizable(true)
        .center()
        .focused(true)
        .initialization_script(&init)
        .build()
        .map_err(|e| format!("building auth window: {e}"))?;
    Ok(())
}

/// Receives `{id, secret, label, expires_at}` from the webview after
/// a successful `POST /api/tokens`. Persists to keychain and closes
/// the auth window. Returns the public auth status so the frontend
/// can re-render without a follow-up `auth_status` round-trip.
#[tauri::command]
pub fn save_pat(app: AppHandle, pat: StoredPat) -> Result<AuthStatus, String> {
    store(&pat)?;
    if let Some(w) = app.get_webview_window(AUTH_WINDOW_LABEL) {
        let _ = w.close();
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
    Ok(status)
}

/// Local sign-out. Clears the keychain entry. Server-side revoke is
/// a follow-up — it needs the id.chan.app session cookie which only
/// the webview has access to.
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

/// JS injected into the id.chan.app webview. Polls `/api/me` and
/// fires the token mint as soon as the session is alive, regardless
/// of which screen the SPA chose to render. The user never sees the
/// profile page on a fresh sign-in.
fn signin_init_script(hostname: &str) -> String {
    let label = format!("chan-desktop @ {hostname}");
    let label_json = serde_json::to_string(&label).unwrap_or_else(|_| "\"chan-desktop\"".into());
    format!(
        r#"
(() => {{
  if (window.__chanAuthInstalled) return;
  window.__chanAuthInstalled = true;

  const LABEL = {label_json};
  const EXPIRES_IN = 30 * 24 * 60 * 60;
  const SCOPES = ['tunnel'];
  const COOLDOWN_MS = 500;
  let inflight = false;
  let done = false;
  let lastAttempt = 0;
  let obs = null;

  async function tryMint() {{
    if (done || inflight) return;
    const now = Date.now();
    if (now - lastAttempt < COOLDOWN_MS) return;
    lastAttempt = now;
    inflight = true;
    try {{
      const me = await fetch('/api/me', {{ credentials: 'include' }});
      if (!me.ok) return;
      const tk = await fetch('/api/tokens', {{
        method: 'POST',
        credentials: 'include',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{ label: LABEL, expires_in: EXPIRES_IN, scopes: SCOPES }}),
      }});
      if (!tk.ok) return;
      const d = await tk.json();
      if (!d || !d.secret || !d.id) return;
      done = true;
      if (obs) {{ obs.disconnect(); obs = null; }}
      const invoke = (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke)
        || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke);
      if (!invoke) return;
      await invoke('save_pat', {{
        pat: {{
          id: String(d.id),
          secret: String(d.secret),
          label: String(d.label || LABEL),
          expires_at: d.expires_at ? String(d.expires_at) : '',
        }},
      }});
    }} catch (_e) {{
      // Swallow; we'll retry on the next navigation / DOM change.
    }} finally {{
      inflight = false;
    }}
  }}

  // Run on first load, then watch for SPA route changes (the
  // identity SPA replaces history.state on login). A passive
  // MutationObserver catches the re-render that follows the
  // OAuth callback redirect.
  window.addEventListener('DOMContentLoaded', tryMint);
  window.addEventListener('load', tryMint);
  obs = new MutationObserver(() => {{
    if (done) {{ obs && obs.disconnect(); obs = null; return; }}
    tryMint();
  }});
  obs.observe(document.documentElement, {{ childList: true, subtree: true }});
}})();
"#
    )
}

