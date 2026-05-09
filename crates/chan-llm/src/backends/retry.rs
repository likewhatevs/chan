//! Shared retry policy for the HTTP backends.
//!
//! Backends call `send_with_retry` once per turn to dispatch the
//! initial request. Retries kick in for transient failures only:
//!
//!   - Network errors (connect, TLS, DNS) the reqwest client raises
//!     before getting a response - retried with exponential backoff.
//!   - 5xx response statuses - upstream had a momentary problem
//!     (Anthropic / Gemini both 503 occasionally during deploys).
//!   - 429 with `Retry-After` honored - the standard rate-limit
//!     dance.
//!
//! 4xx other than 429 are NOT retried: they're caller errors (bad
//! key, bad model, malformed body) that won't fix themselves.
//!
//! Retries only happen on the *initial* request. Once we start
//! consuming the streaming body, errors propagate as `on_error`
//! into the listener; we can't replay a stream.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Response;

/// How many attempts beyond the initial one. With `attempts=2`
/// we issue at most three requests total before giving up.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub attempts: u32,
    /// Base delay; each subsequent retry doubles it. The first
    /// retry waits `base`, the second `2 * base`, etc.
    pub base: Duration,
    /// Cap so a server-supplied Retry-After can't pin us forever.
    pub max_wait: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            attempts: 2,
            base: Duration::from_millis(500),
            max_wait: Duration::from_secs(20),
        }
    }
}

/// Send the request returned by `build`, retrying transient
/// failures per `policy`. `build` is invoked fresh on every
/// attempt because `RequestBuilder` isn't reusable after `send`.
///
/// The cancel flag is checked between attempts so a user hitting
/// "stop" during a backoff window doesn't have to wait for the
/// timer to fire.
pub async fn send_with_retry<F>(
    build: F,
    policy: RetryPolicy,
    cancel: &Arc<AtomicBool>,
    label: &'static str,
) -> Result<Response, RetryError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempt: u32 = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(RetryError::Cancelled);
        }
        let resp = build().send().await;
        match resp {
            Ok(r) if r.status().is_success() => return Ok(r),
            Ok(r) => {
                let status = r.status();
                let retriable = status.is_server_error() || status.as_u16() == 429;
                if !retriable || attempt >= policy.attempts {
                    return Ok(r);
                }
                let wait = pick_wait(&r, attempt, policy);
                let body = r.text().await.unwrap_or_default();
                tracing::warn!(
                    backend = label,
                    %status,
                    attempt,
                    next_wait_ms = wait.as_millis() as u64,
                    body_snippet = %body.chars().take(160).collect::<String>(),
                    "retrying transient error",
                );
                if sleep_or_cancel(wait, cancel).await {
                    return Err(RetryError::Cancelled);
                }
                attempt += 1;
            }
            Err(e) => {
                if attempt >= policy.attempts {
                    return Err(RetryError::Network(e.to_string()));
                }
                let wait = backoff(attempt, policy);
                tracing::warn!(
                    backend = label,
                    attempt,
                    next_wait_ms = wait.as_millis() as u64,
                    err = %e,
                    "retrying network error",
                );
                if sleep_or_cancel(wait, cancel).await {
                    return Err(RetryError::Cancelled);
                }
                attempt += 1;
            }
        }
    }
}

fn pick_wait(resp: &Response, attempt: u32, policy: RetryPolicy) -> Duration {
    if let Some(retry_after) = resp.headers().get("retry-after") {
        if let Ok(s) = retry_after.to_str() {
            if let Ok(secs) = s.trim().parse::<u64>() {
                return Duration::from_secs(secs).min(policy.max_wait);
            }
            // RFC date format isn't worth implementing here; fall
            // through to exponential backoff if we can't parse the
            // numeric form.
        }
    }
    backoff(attempt, policy)
}

fn backoff(attempt: u32, policy: RetryPolicy) -> Duration {
    // saturating_shl is nightly-only; emulate. attempt > 31 saturates
    // by clamping to max_wait via the .min() below, so 1u32 << 31 is
    // already safe enough for realistic attempt counts.
    let shift = attempt.min(31);
    let factor = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
    policy.base.saturating_mul(factor).min(policy.max_wait)
}

/// Sleep for `wait`, returning early when cancel flips. Returns
/// true if cancel was observed; false if the sleep completed.
async fn sleep_or_cancel(wait: Duration, cancel: &Arc<AtomicBool>) -> bool {
    let poll = Duration::from_millis(50);
    let mut elapsed = Duration::ZERO;
    while elapsed < wait {
        if cancel.load(Ordering::Relaxed) {
            return true;
        }
        let chunk = poll.min(wait - elapsed);
        tokio::time::sleep(chunk).await;
        elapsed += chunk;
    }
    cancel.load(Ordering::Relaxed)
}

#[derive(Debug)]
pub enum RetryError {
    Network(String),
    Cancelled,
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetryError::Network(s) => write!(f, "network error: {s}"),
            RetryError::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_per_attempt() {
        let p = RetryPolicy {
            attempts: 5,
            base: Duration::from_millis(100),
            max_wait: Duration::from_secs(10),
        };
        assert_eq!(backoff(0, p), Duration::from_millis(100));
        assert_eq!(backoff(1, p), Duration::from_millis(200));
        assert_eq!(backoff(2, p), Duration::from_millis(400));
        assert_eq!(backoff(3, p), Duration::from_millis(800));
    }

    #[test]
    fn backoff_caps_at_max_wait() {
        let p = RetryPolicy {
            attempts: 100,
            base: Duration::from_secs(1),
            max_wait: Duration::from_secs(5),
        };
        assert_eq!(backoff(20, p), Duration::from_secs(5));
    }
}
