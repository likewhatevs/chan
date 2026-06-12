//! Thin reqwest client for workspace-proxy's `/admin/v1/*` tree.
//!
//! Used by:
//!   * identity-service on PAT revoke, account delete, and dashboard
//!     reads (`/api/me` merges the live workspace list), so id can
//!     render and gate without going through workspace-proxy's wildcard
//!     surface.
//!   * profile-service on admin block, so the in-process yamux
//!     registrations workspace-proxy holds for that user are torn down
//!     at the same time the DB state changes.
//!
//! Errors are surfaced but every write call site should treat this
//! as best-effort: a brief workspace-proxy outage shouldn't block the
//! primary action (revoke, block, delete). On the wire the existing
//! tokens stop validating immediately; the live substreams just
//! linger a bit longer than ideal. Read calls bubble up directly
//! because the dashboard genuinely needs the answer to render.
//!
//! `WorkspaceAdminError` is the client's own error enum so this crate
//! has no axum / IntoResponse dependency. Each consumer maps it onto
//! its local error via a `From` impl.

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceAdminError {
    #[error("workspace-proxy admin upstream: {0}")]
    Upstream(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

pub type WorkspaceAdminResult<T> = Result<T, WorkspaceAdminError>;

/// One live tunnel as workspace-proxy reports it. Mirrors workspace-proxy's
/// `admin::TunnelView`; the duplication is deliberate (this crate
/// stays independent of workspace-proxy's internal types so it can be
/// pulled by identity and profile without a circular dep).
#[derive(Debug, Clone, Deserialize)]
pub struct TunnelView {
    pub user: String,
    pub workspace: String,
    pub public: bool,
    pub peer_addr: Option<String>,
    pub connected_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct WorkspaceAdminClient {
    base: Url,
    http: reqwest::Client,
    token: String,
}

impl std::fmt::Debug for WorkspaceAdminClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceAdminClient")
            .field("base", &self.base)
            // token deliberately elided
            .finish()
    }
}

/// Percent-encode one path segment. Usernames are normally
/// `[a-z0-9-]` but the admin tree may have to handle pre-normalized
/// inputs (transient migration data, future relaxed validators), so
/// any byte outside the unreserved set per RFC 3986 §2.3 is escaped.
/// `url::Url::set_path` does not handle this automatically.
fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl WorkspaceAdminClient {
    pub fn new(base: Url, token: String) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .connect_timeout(std::time::Duration::from_secs(2))
            .build()?;
        Ok(Self { base, http, token })
    }

    /// Force-evict every tunnel `username` has live. Idempotent;
    /// "nothing to kill" returns `0`.
    pub async fn kill_user_tunnels(&self, username: &str) -> WorkspaceAdminResult<usize> {
        let mut url = self.base.clone();
        let user = encode_segment(username);
        url.set_path(&format!("/admin/v1/users/{user}/tunnels/kill"));
        let res = self.http.post(url).bearer_auth(&self.token).send().await?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            tracing::warn!(%status, body = %body, "workspace-proxy admin upstream error");
            return Err(WorkspaceAdminError::Upstream(format!("{status}")));
        }
        // The endpoint returns 200 with a JSON body; tolerate 204 to
        // leave room for a future "noop" optimisation.
        if status == StatusCode::NO_CONTENT {
            return Ok(0);
        }
        let body: KillResponse = res.json().await?;
        Ok(body.killed)
    }

    /// Snapshot of every live tunnel for `username`. Identity's
    /// dashboard calls this on every `/api/me` so the SPA renders
    /// the user's workspace cards. Empty list when the user has nothing
    /// connected; absent user returns 200 with an empty list (the
    /// endpoint doesn't 404 on unknown users so callers don't have
    /// to special-case the steady state where a fresh sign-in has
    /// nothing registered yet).
    pub async fn list_user_tunnels(&self, username: &str) -> WorkspaceAdminResult<Vec<TunnelView>> {
        let mut url = self.base.clone();
        let user = encode_segment(username);
        url.set_path(&format!("/admin/v1/users/{user}/tunnels"));
        let res = self.http.get(url).bearer_auth(&self.token).send().await?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            tracing::warn!(%status, body = %body, "workspace-proxy admin upstream error");
            return Err(WorkspaceAdminError::Upstream(format!("{status}")));
        }
        let tunnels: Vec<TunnelView> = res.json().await?;
        Ok(tunnels)
    }
}

#[derive(Debug, Deserialize)]
struct KillResponse {
    killed: usize,
}
