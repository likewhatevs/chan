//! Per-token-fingerprint rate limiter for the tunnel handshake.
//!
//! Identity-service used to run a `tower_governor` per-IP gate on
//! `/internal/v1/tokens/validate`, but the only peer that endpoint
//! ever sees is workspace-proxy itself: every request keys to the same
//! container IP, the "per-IP" bucket degenerates into a single
//! global one, and a noisy attacker can lock out legitimate
//! handshakes while real source-IP diversity stays invisible.
//!
//! The brute-force surface lives one hop earlier, at the tunnel
//! handshake in workspace-proxy. We can't easily key on the original
//! client IP there (the listener is raw h2, not axum, and the
//! gateway terminator wraps `chan-tunnel-server` from chan-core;
//! plumbing X-Forwarded-For into the validator means a chan-core
//! surface change). What we can do cheaply is throttle by the
//! candidate token itself: hash the bytes to a 64-bit fingerprint
//! and run a token-bucket per fingerprint. Guessing a specific
//! (possibly leaked) PAT is bounded regardless of the attacker's
//! IP distribution; random-tail brute force still has astronomical
//! odds per fresh prefix, so the cap on map size is fine.
//!
//! Throttled validates surface as `ServerError::InvalidToken`.
//! On the wire this is the same 401 an unknown token returns, so
//! an attacker can't oracle "this fingerprint is rate-limited"
//! vs "this token is unknown" by response shape. chan-tunnel-client
//! retries 401 with exponential backoff, so a legit client that
//! somehow burned its burst recovers on the next attempt once the
//! bucket has refilled; it does not lock the client out.
//!
//! The bucket primitive itself lives in
//! `gateway_common::token_bucket`; this module is the trait wrapper
//! that turns it into a `Validator` decorator.

use async_trait::async_trait;
use chan_tunnel_server::{ServerError, Validated, Validator};
use gateway_common::token_bucket::TokenBucket;

/// Default refill rate (tokens per second) per fingerprint. Matches
/// the rate the old identity governor advertised so the overall
/// budget for a single `chan serve` handshake loop is unchanged;
/// just keyed correctly now.
const DEFAULT_REFILL_PER_SEC: f32 = 4.0;

/// Default bucket capacity (burst). Same shape as the old governor.
const DEFAULT_CAPACITY: f32 = 16.0;

/// Hard cap on tracked fingerprints. An attacker hammering with
/// random tokens can fill the map; capping it bounds memory and
/// the eviction step. 4096 is well above the steady-state working
/// set (one entry per active PAT in the wild) and small enough
/// that the O(n) eviction scan is negligible.
const DEFAULT_MAP_CAP: usize = 4096;

pub struct ThrottlingValidator<V: Validator> {
    inner: V,
    bucket: TokenBucket,
}

impl<V: Validator> ThrottlingValidator<V> {
    pub fn new(inner: V) -> Self {
        Self {
            inner,
            bucket: TokenBucket::new(DEFAULT_REFILL_PER_SEC, DEFAULT_CAPACITY, DEFAULT_MAP_CAP),
        }
    }

    pub fn with_limits(inner: V, refill_per_sec: f32, capacity: f32, map_cap: usize) -> Self {
        Self {
            inner,
            bucket: TokenBucket::new(refill_per_sec, capacity, map_cap),
        }
    }
}

#[async_trait]
impl<V: Validator> Validator for ThrottlingValidator<V> {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        let fp = TokenBucket::fingerprint(token);
        if !self.bucket.try_admit_fp(fp, std::time::Instant::now()) {
            tracing::warn!(
                fingerprint = %format!("{fp:016x}"),
                "tunnel validate throttled"
            );
            return Err(ServerError::InvalidToken);
        }
        self.inner.validate(token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use uuid::Uuid;

    struct CountingValidator {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Validator for CountingValidator {
        async fn validate(&self, _token: &str) -> Result<Validated, ServerError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(Validated {
                user_id: Uuid::nil(),
                username: "u".into(),
                scopes: vec!["tunnel".into()],
            })
        }
    }

    #[tokio::test]
    async fn fresh_fingerprint_admits_one_then_blocks() {
        // Token-bucket entries start with 1.0 token, not `capacity`,
        // so a fresh fingerprint admits exactly once before refill.
        let calls = Arc::new(AtomicUsize::new(0));
        let v = ThrottlingValidator::with_limits(
            CountingValidator {
                calls: calls.clone(),
            },
            4.0,
            3.0,
            16,
        );
        assert!(v.validate("chan_pat_x").await.is_ok());
        let err = v.validate("chan_pat_x").await.unwrap_err();
        assert!(matches!(err, ServerError::InvalidToken));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn rejects_past_burst_for_same_token() {
        // With refill_per_sec = 4.0, two back-to-back calls within the
        // same tick should see 1 admit, then a near-instantaneous
        // throttle until the bucket refills.
        let calls = Arc::new(AtomicUsize::new(0));
        let v = ThrottlingValidator::with_limits(
            CountingValidator {
                calls: calls.clone(),
            },
            4.0,
            2.0,
            16,
        );
        assert!(v.validate("chan_pat_a").await.is_ok());
        let err = v.validate("chan_pat_a").await.unwrap_err();
        assert!(matches!(err, ServerError::InvalidToken));
        // Inner validator never saw the throttled call.
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn distinct_tokens_have_distinct_buckets() {
        let calls = Arc::new(AtomicUsize::new(0));
        let v = ThrottlingValidator::with_limits(
            CountingValidator {
                calls: calls.clone(),
            },
            4.0,
            1.0,
            16,
        );
        assert!(v.validate("chan_pat_a").await.is_ok());
        // Same fingerprint -> blocked.
        assert!(v.validate("chan_pat_a").await.is_err());
        // Different token -> independent bucket admits one.
        assert!(v.validate("chan_pat_b").await.is_ok());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn inner_error_propagates_without_masking() {
        // The throttle must not mask upstream failures (identity-service
        // 502, network blip) as InvalidToken. chan-tunnel-client retries
        // both, but they surface as different log messages on the
        // client and different status codes on the wire; collapsing
        // them would lose useful diagnostic signal.
        struct FailingValidator;
        #[async_trait]
        impl Validator for FailingValidator {
            async fn validate(&self, _t: &str) -> Result<Validated, ServerError> {
                Err(ServerError::Identity("upstream down".into()))
            }
        }
        let v = ThrottlingValidator::new(FailingValidator);
        let err = v.validate("chan_pat_a").await.unwrap_err();
        assert!(matches!(err, ServerError::Identity(_)));
    }
}
