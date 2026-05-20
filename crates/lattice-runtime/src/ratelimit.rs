//! Rate limiting port and in-memory token-bucket implementation.
//!
//! The [`RateLimiter`] port is backend-agnostic; for Redis-backed distributed
//! rate limiting, implement the trait directly.
//!
//! [`MemoryRateLimiter`] is a per-(namespace, key) token-bucket suitable for
//! single-process deployments.

use lattice_core::Error;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Port trait
// ---------------------------------------------------------------------------

/// Rate-limiting port.
///
/// `namespace` partitions limits (e.g. workspace ID); `key` identifies the
/// subject within the namespace (e.g. user ID or IP address).
///
/// Returns `Ok(true)` if the request is allowed and the token was consumed,
/// `Ok(false)` if the limit is exceeded, or `Err` if the implementation
/// encounters an unrecoverable error.
///
/// For Redis/Valkey distributed rate limiting, implement this trait directly.
pub trait RateLimiter: Send + Sync {
    /// Attempt to consume one token for `(namespace, key)`.
    fn allow(&self, namespace: &str, key: &str) -> Result<bool, Error>;
    /// Remaining tokens for `(namespace, key)` (best-effort, may be stale).
    fn remaining(&self, namespace: &str, key: &str) -> Result<f64, Error>;
}

// ---------------------------------------------------------------------------
// MemoryRateLimiter — token-bucket, O(1) per allow()
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

impl Bucket {
    fn new(burst: f64) -> Self {
        Self {
            tokens: burst,
            last: Instant::now(),
        }
    }

    /// Replenish tokens based on elapsed time, cap at `burst`.
    fn refill(&mut self, rate: f64, burst: f64) {
        let elapsed = self.last.elapsed().as_secs_f64();
        self.tokens = (self.tokens + elapsed * rate).min(burst);
        self.last = Instant::now();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BucketKey {
    namespace: String,
    key: String,
}

/// In-memory token-bucket rate limiter, safe for concurrent use.
///
/// - `rate`: tokens replenished per second.
/// - `burst`: maximum number of tokens (initial and cap).
///
/// Buckets are created lazily on first access with a full `burst` allowance.
/// Entries are never evicted — for long-running processes with millions of
/// distinct keys, prefer a Redis-backed implementation.
pub struct MemoryRateLimiter {
    rate: f64,
    burst: f64,
    state: Mutex<HashMap<BucketKey, Bucket>>,
}

impl MemoryRateLimiter {
    /// Create a new limiter.
    ///
    /// `rate` tokens are replenished per second; `burst` is the maximum
    /// burst capacity (and the initial token count for a new bucket).
    ///
    /// # Panics
    /// Panics if `rate <= 0` or `burst <= 0`.
    pub fn new(rate: f64, burst: f64) -> Self {
        assert!(rate > 0.0, "rate must be positive");
        assert!(burst > 0.0, "burst must be positive");
        Self {
            rate,
            burst,
            state: Mutex::new(HashMap::new()),
        }
    }
}

impl RateLimiter for MemoryRateLimiter {
    fn allow(&self, namespace: &str, key: &str) -> Result<bool, Error> {
        let k = BucketKey {
            namespace: namespace.to_string(),
            key: key.to_string(),
        };
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let bucket = map.entry(k).or_insert_with(|| Bucket::new(self.burst));
        bucket.refill(self.rate, self.burst);
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn remaining(&self, namespace: &str, key: &str) -> Result<f64, Error> {
        let k = BucketKey {
            namespace: namespace.to_string(),
            key: key.to_string(),
        };
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let bucket = map.entry(k).or_insert_with(|| Bucket::new(self.burst));
        bucket.refill(self.rate, self.burst);
        Ok(bucket.tokens.floor())
    }
}

/// Rate-limit error returned from runtime operations when the limit is exceeded.
pub fn rate_limited_error(namespace: &str, key: &str) -> Error {
    Error::internal(format!("rate limit exceeded for {}/{}", namespace, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_burst() {
        let limiter = MemoryRateLimiter::new(1.0, 5.0);
        for _ in 0..5 {
            assert!(limiter.allow("ws", "user").unwrap());
        }
        assert!(
            !limiter.allow("ws", "user").unwrap(),
            "6th call should be denied"
        );
    }

    #[test]
    fn namespaces_are_independent() {
        let limiter = MemoryRateLimiter::new(1.0, 2.0);
        assert!(limiter.allow("ws1", "u").unwrap());
        assert!(limiter.allow("ws2", "u").unwrap());
        assert!(limiter.allow("ws1", "u").unwrap());
        assert!(limiter.allow("ws2", "u").unwrap());
        assert!(!limiter.allow("ws1", "u").unwrap());
        assert!(!limiter.allow("ws2", "u").unwrap());
    }

    #[test]
    fn remaining_reports_floor() {
        let limiter = MemoryRateLimiter::new(1.0, 3.0);
        assert_eq!(limiter.remaining("ws", "u").unwrap(), 3.0);
        limiter.allow("ws", "u").unwrap();
        assert_eq!(limiter.remaining("ws", "u").unwrap(), 2.0);
    }
}
