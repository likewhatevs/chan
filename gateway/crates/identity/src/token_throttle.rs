//! Per-token-fingerprint rate limiter for /internal/v1/tokens/validate.
//!
//! devserver-proxy already runs an identical throttle one hop earlier
//! (`crates/devserver-proxy/src/throttle_validator.rs`). This is the
//! defense-in-depth twin: if the shared internal bearer leaks and
//! someone hits identity-service directly, the per-fingerprint
//! token bucket caps brute force even when the upstream throttle
//! is bypassed. The two throttles do not coordinate; either alone
//! is enough to make a guess loop glacial.
//!
//! The bucket primitive itself lives in
//! `gateway_common::token_bucket`; this module exposes a tiny
//! handler-friendly wrapper. Throttled requests return `false` from
//! `try_admit`; the handler maps that to a 401, identical on the
//! wire to "unknown token" so the throttle is not observable.

use gateway_common::token_bucket::{
    TokenBucket, DEFAULT_CAPACITY, DEFAULT_MAP_CAP, DEFAULT_REFILL_PER_SEC,
};

#[derive(Clone)]
pub struct TokenThrottle {
    bucket: TokenBucket,
}

impl TokenThrottle {
    pub fn new() -> Self {
        Self {
            bucket: TokenBucket::new(DEFAULT_REFILL_PER_SEC, DEFAULT_CAPACITY, DEFAULT_MAP_CAP),
        }
    }

    pub fn with_limits(refill_per_sec: f32, capacity: f32, map_cap: usize) -> Self {
        Self {
            bucket: TokenBucket::new(refill_per_sec, capacity, map_cap),
        }
    }

    /// Returns `true` if a token has been consumed for this
    /// fingerprint, `false` if the bucket is empty. Caller maps
    /// `false` to the same 401 a real "unknown token" returns.
    pub fn try_admit(&self, token: &str) -> bool {
        self.bucket.try_admit(token)
    }
}

impl Default for TokenThrottle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_fingerprint_admits_one_then_blocks() {
        // New token-bucket entries start with 1.0 token, not capacity,
        // so a fresh fingerprint admits exactly once before refill.
        let t = TokenThrottle::with_limits(4.0, 3.0, 16);
        assert!(t.try_admit("chan_pat_a"));
        assert!(!t.try_admit("chan_pat_a"));
    }

    #[test]
    fn distinct_tokens_have_distinct_buckets() {
        let t = TokenThrottle::with_limits(4.0, 1.0, 16);
        assert!(t.try_admit("chan_pat_a"));
        assert!(!t.try_admit("chan_pat_a"));
        assert!(t.try_admit("chan_pat_b"));
    }
}
