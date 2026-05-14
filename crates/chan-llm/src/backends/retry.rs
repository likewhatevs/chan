//! Shared retry policy for the HTTP backends.
//!
//! Backends call `send_with_retry` once per turn to dispatch the
//! initial request. Retries kick in for transient failures only:
//!
//!   - Network errors (connect, TLS, DNS) the reqwest client raises
//!     before getting a response - retried with exponential backoff.
//!   - 5xx response statuses - upstream had a momentary problem
//!     (Anthropic / Gemini both 503 occasionally during deploys).
//!   - 408 Request Timeout, 425 Too Early - transient signals from
//!     proxies / CDNs that the request just didn't make it.
//!   - 429 with `Retry-After` honored - the standard rate-limit
//!     dance. The header may be a number of seconds or an
//!     RFC 7231 HTTP-date; both are parsed and clamped to
//!     `policy.max_wait`.
//!   - 529 Site Overloaded - Anthropic's "we're at capacity" signal,
//!     non-standard but common enough to bite during incident
//!     windows.
//!
//! 4xx other than the transient set above are NOT retried: they're
//! caller errors (bad key, bad model, malformed body) that won't fix
//! themselves.
//!
//! Retries only happen on the *initial* request. Once we start
//! consuming the streaming body, errors propagate as `on_error`
//! into the listener; we can't replay a stream.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

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
                let retriable = is_retryable(status.as_u16());
                if !retriable || attempt >= policy.attempts {
                    return Ok(r);
                }
                let wait = pick_wait(&r, attempt, policy);
                // Read the body with a hard byte cap; we only need a
                // short snippet for the log line and a runaway upstream
                // returning megabytes of HTML on a 503 must not force
                // us to allocate it just to discard it. 2 KiB is well
                // above what any structured error envelope needs.
                let (body, _) = super::error_body::read_capped_text(r, 2 * 1024).await;
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

/// Statuses worth retrying. Mirrors the doc comment at the top of
/// this module; pulled into a free function so callers / tests
/// agree on the set.
pub(crate) fn is_retryable(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 529 | 500..=599)
}

fn pick_wait(resp: &Response, attempt: u32, policy: RetryPolicy) -> Duration {
    if let Some(retry_after) = resp.headers().get("retry-after") {
        if let Ok(s) = retry_after.to_str() {
            if let Some(wait) = parse_retry_after(s.trim()) {
                return wait.min(policy.max_wait);
            }
            tracing::warn!(
                retry_after = %s,
                "retry-after header could not be parsed as seconds or HTTP-date; \
                 falling back to exponential backoff",
            );
        }
    }
    backoff(attempt, policy)
}

/// Parse a `Retry-After` header value. Accepts both shapes RFC 7231
/// defines: a non-negative integer count of seconds, or an
/// IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`) we resolve against
/// the local wall clock.
///
/// Returns `None` when the value is neither shape, or when an
/// HTTP-date resolves to a time already in the past (in which case
/// the upstream's hint is meaningless and we fall back to exponential
/// backoff to keep the cadence smooth).
pub(crate) fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let target = httpdate::parse_http_date(value).ok()?;
    let now = SystemTime::now();
    target.duration_since(now).ok()
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

    #[test]
    fn is_retryable_covers_the_transient_set() {
        // Documented retryables.
        assert!(is_retryable(408));
        assert!(is_retryable(425));
        assert!(is_retryable(429));
        assert!(is_retryable(529));
        assert!(is_retryable(500));
        assert!(is_retryable(502));
        assert!(is_retryable(503));
        assert!(is_retryable(504));
        // Non-retryable caller errors.
        assert!(!is_retryable(400));
        assert!(!is_retryable(401));
        assert!(!is_retryable(403));
        assert!(!is_retryable(404));
        assert!(!is_retryable(422));
        // 2xx / 3xx never retry (callers short-circuit on success).
        assert!(!is_retryable(200));
        assert!(!is_retryable(301));
    }

    #[test]
    fn parse_retry_after_accepts_numeric_seconds() {
        assert_eq!(parse_retry_after("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_retry_after("0"), Some(Duration::from_secs(0)));
    }

    #[test]
    fn parse_retry_after_accepts_imf_fixdate_in_future() {
        // Build a date 120 seconds from now and confirm we get
        // roughly that delta back. Allow a wide window because the
        // test machine's clock can drift between samples; the
        // assertion is "we got something positive and within
        // 30 seconds of the requested 120s".
        let now = SystemTime::now();
        let target = now + Duration::from_secs(120);
        let s = httpdate::fmt_http_date(target);
        let parsed = parse_retry_after(&s).expect("future date parses");
        let lo = Duration::from_secs(90);
        let hi = Duration::from_secs(150);
        assert!(
            parsed >= lo && parsed <= hi,
            "expected ~120s, got {parsed:?}",
        );
    }

    #[test]
    fn parse_retry_after_rejects_past_dates() {
        // A date in the past returns None so the caller falls back
        // to exponential backoff instead of waiting zero.
        let past = SystemTime::now() - Duration::from_secs(3600);
        let s = httpdate::fmt_http_date(past);
        assert!(parse_retry_after(&s).is_none());
    }

    #[test]
    fn parse_retry_after_rejects_garbage() {
        assert!(parse_retry_after("not a date").is_none());
        assert!(parse_retry_after("").is_none());
        assert!(parse_retry_after("-1").is_none());
    }
}
