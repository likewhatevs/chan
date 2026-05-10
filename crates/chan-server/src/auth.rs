//! Per-launch bearer token + axum middleware that gates `/api/*` and `/ws`.
//!
//! Token persistence lives at `<paths.tokens>/token` (mode 0600 on Unix)
//! so a `cargo build && chan serve` cycle does not invalidate the
//! browser's cached sessionStorage token. Atomic write goes through
//! chan-drive's `fs_ops::atomic_write` so the parent-dir fsync invariant
//! matches the rest of the app.
//!
//! Tunnel mode forces the gate off (`AppState::token == None`); the
//! drive.chan.app gateway is the trust boundary in that path.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use chan_drive::paths::DrivePaths;
use rand::RngCore;

use crate::signal::now_unix_secs;
use crate::state::AppState;

const TOKEN_LEN: usize = 32;
const TOKEN_ALPHABET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub fn random_token() -> String {
    let mut bytes = [0u8; TOKEN_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| TOKEN_ALPHABET[(*b as usize) % TOKEN_ALPHABET.len()] as char)
        .collect()
}

/// Load the persisted server token, generating one on first run.
/// Lives at `<paths.tokens>/token` (mode 0600 on Unix). The token
/// survives a binary rebuild so the browser's cached sessionStorage
/// token stays valid across `cargo build && chan serve` cycles.
pub fn load_or_create_token(paths: &DrivePaths) -> std::io::Result<String> {
    std::fs::create_dir_all(&paths.tokens)?;
    let token_path = paths.tokens.join("token");
    if let Ok(s) = std::fs::read_to_string(&token_path) {
        let s = s.trim();
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Ok(s.to_owned());
        }
    }
    let token = random_token();
    write_token_atomic(&token_path, &token)?;
    Ok(token)
}

/// Write the token via chan-drive's atomic_write helper (tmpfile +
/// fsync of file AND parent dir + rename). Sets 0600 permissions on
/// Unix to keep the secret out of `ls -l` snooping.
fn write_token_atomic(token_path: &Path, token: &str) -> std::io::Result<()> {
    chan_drive::fs_ops::atomic_write(token_path, token.as_bytes())
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(token_path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Reject requests that don't carry the right token.
///
/// Auth scope: only `/api/*` and `/ws` routes are gated. Static
/// assets stay open: the browser issues those via `<script src>` /
/// `<link href>` before our JS runs and they can't carry the token.
/// The data plane is what needs protecting.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // The activity bump lives here so it only fires on requests we
    // actually serve: a brute-forcing client with bad tokens keeps
    // hitting the 401 path below and won't pin the idle timer open.
    let bump = || {
        state
            .last_activity
            .store(now_unix_secs(), Ordering::Relaxed)
    };
    let Some(expected) = state.token.as_deref() else {
        bump();
        return next.run(req).await;
    };
    let path = req.uri().path();
    if !(path.starts_with("/api") || path == "/ws") {
        bump();
        return next.run(req).await;
    }
    if extract_token(req.uri().query(), req.headers()) == Some(expected) {
        bump();
        return next.run(req).await;
    }
    (StatusCode::UNAUTHORIZED, "missing or invalid token").into_response()
}

pub fn extract_token<'a>(query: Option<&'a str>, headers: &'a HeaderMap) -> Option<&'a str> {
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some(rest) = pair.strip_prefix("t=") {
                return Some(rest);
            }
        }
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_token_is_alphanumeric_and_long() {
        let t = random_token();
        assert_eq!(t.len(), TOKEN_LEN);
        assert!(t.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn extract_token_query_param() {
        let h = HeaderMap::new();
        assert_eq!(
            extract_token(Some("foo=bar&t=secret&x=y"), &h),
            Some("secret")
        );
    }

    #[test]
    fn extract_token_authorization_header() {
        let mut h = HeaderMap::new();
        h.insert(header::AUTHORIZATION, "Bearer secret".parse().unwrap());
        assert_eq!(extract_token(None, &h), Some("secret"));
    }

    #[test]
    fn extract_token_missing() {
        let h = HeaderMap::new();
        assert_eq!(extract_token(None, &h), None);
    }
}
