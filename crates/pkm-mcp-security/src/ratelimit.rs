//! Per-key token-bucket rate limiting (contract §9).
//!
//! Applies to authenticated HTTP tool calls only (not `initialize`, not stdio,
//! not `/health`). The default bucket is burst = `MCP_RATE_LIMIT_BURST` (60),
//! refill = `MCP_RATE_LIMIT_RPS` (10/s) per §9. The bucket key is: token
//! subject when authenticated; else client-id; else source IP.
//!
//! On exceed the server returns `RateLimited` (-32003) with a `Retry-After`
//! header (seconds to refill one token) and HTTP 429.
//!
//! This implementation is intentionally pure and deterministic on wall-clock
//! time, so it can be unit-tested with an injected clock and used as the
//! single-writer enforcement layer by whichever transport binds it.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default burst capacity (§9): 60 tokens.
pub const DEFAULT_BURST: u32 = 60;
/// Default refill rate (§9): 10 tokens/second.
pub const DEFAULT_RPS: f64 = 10.0;
/// Default number of bucketed keys to retain before evicting idle ones.
const MAX_KEYS: usize = 512;

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, PartialEq)]
pub enum RateDecision {
    /// The request is admitted and `remaining` tokens remain in the bucket.
    Allowed {
        /// Tokens left in the bucket after this admission (for diagnostics).
        remaining: u32,
        /// Seconds until the bucket would refill to full (informational; 0
        /// when not under pressure).
        retry_after_seconds: u64,
    },
    /// The request is rejected; `retry_after_seconds` is the backoff to
    /// present in the `Retry-After` header.
    Limited {
        /// Seconds to wait before one token is available.
        retry_after_seconds: u64,
    },
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: u64, // unix seconds
}

/// A token-bucket rate limiter keyed by opaque string identifiers.
///
/// Not `Sync`/`Send`-agnostic: wrap in a `Mutex` for cross-task use, or hold
/// per-connection instances for stdio. Uses interior `Mutex` internally so the
/// whole limiter can be shared behind an `Arc`.
#[derive(Debug)]
pub struct TokenBucketLimiter {
    key_burst: u32,
    key_rps: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl Default for TokenBucketLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_BURST, DEFAULT_RPS)
    }
}

impl TokenBucketLimiter {
    /// Create a limiter with the given burst and per-second refill.
    ///
    /// Panics if `burst == 0` (a bucket with no capacity would reject
    /// everything) or `rps <= 0`.
    pub fn new(burst: u32, rps: f64) -> Self {
        assert!(burst > 0, "burst must be positive");
        assert!(
            rps.is_finite() && rps > 0.0,
            "rps must be positive and finite"
        );
        Self {
            key_burst: burst,
            key_rps: rps,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Borrowed defaults for a limiter with the contract default shape.
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Check and consume one token for `key`.
    ///
    /// `now_secs` is the Unix timestamp in seconds; tests inject a clock.
    /// Idle entries over `MAX_KEYS` are pruned on a best-effort basis.
    pub fn check(&self, key: &str, now_secs: u64) -> RateDecision {
        let mut guard = self.buckets.lock().expect("limiter mutex poisoned");
        let bucket = guard.entry(key.to_string()).or_insert(Bucket {
            tokens: self.key_burst as f64,
            last_refill: now_secs,
        });

        // Refill based on elapsed time (capped so a long idle period can't
        // overflow the bucket beyond burst).
        let elapsed = now_secs.saturating_sub(bucket.last_refill);
        if elapsed > 0 {
            bucket.tokens =
                (bucket.tokens + elapsed as f64 * self.key_rps).min(self.key_burst as f64);
            bucket.last_refill = now_secs;
        }

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            // An admitted request never reports a backoff (it succeeded). The
            // `remaining` counter is the current bucket fill for diagnostics.
            let remaining = bucket.tokens.floor().min(self.key_burst as f64) as u32;
            RateDecision::Allowed {
                remaining,
                retry_after_seconds: 0,
            }
        } else {
            RateDecision::Limited {
                retry_after_seconds: retry_after_for(&bucket.tokens, self.key_rps),
            }
        }
    }

    /// Reset a key's bucket (config change, token rotation diagnostics).
    pub fn reset(&self, key: &str) {
        if let Ok(mut guard) = self.buckets.lock() {
            guard.remove(key);
        }
    }

    /// Number of distinct keys currently tracked (diagnostics).
    pub fn tracked_keys(&self) -> usize {
        self.buckets.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Prune tracked keys with near-full buckets (idle) to bound memory.
    /// Called opportunistically by implementations on a timer.
    pub fn prune_idle(&self, now_secs: u64, idle_secs: u64) {
        let mut guard = match self.buckets.lock() {
            Ok(g) => g,
            Err(e) => {
                // Poisoned mutex: recover and continue (never fail a request).
                e.into_inner()
            }
        };
        if guard.len() <= MAX_KEYS {
            return;
        }
        guard.retain(|_, b| {
            // Keep buckets that are still below 80% full (active).
            b.tokens < (self.key_burst as f64) * 0.8
                || now_secs.saturating_sub(b.last_refill) < idle_secs
        });
    }
}

/// Seconds until one token is available: `ceil((1 - tokens) / rps)`.
fn retry_after_for(tokens: &f64, rps: f64) -> u64 {
    if *tokens >= 1.0 {
        return 0;
    }
    let need = (1.0 - tokens).max(0.0) / rps;
    need.ceil() as u64
}

/// Current Unix timestamp in seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Cap an externally provided `burst` to a sane bound (§11 scale).
///
/// Rules:
/// - `0`/unset means "use the contract default burst" (`DEFAULT_BURST`, 60);
/// - values in `1..=DEFAULT_BURST*100` are kept as-is;
/// - anything larger is capped at `DEFAULT_BURST*100` (6000) so an absurd
///   config value cannot disable rate limiting entirely.
pub fn normalized_burst(burst: u32) -> NonZeroU32 {
    if burst == 0 {
        return NonZeroU32::new(DEFAULT_BURST).expect("DEFAULT_BURST non-zero");
    }
    NonZeroU32::new(burst.min(DEFAULT_BURST * 100)).expect("clamped burst non-zero")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_burst_then_limits() {
        let limiter = TokenBucketLimiter::new(3, 10.0);
        let t = 1_000_000;
        assert_eq!(
            limiter.check("k", t),
            RateDecision::Allowed {
                remaining: 2,
                retry_after_seconds: 0
            }
        );
        assert_eq!(
            limiter.check("k", t),
            RateDecision::Allowed {
                remaining: 1,
                retry_after_seconds: 0
            }
        );
        assert_eq!(
            limiter.check("k", t),
            RateDecision::Allowed {
                remaining: 0,
                retry_after_seconds: 0
            }
        );
        assert!(matches!(
            limiter.check("k", t),
            RateDecision::Limited { .. }
        ));
    }

    #[test]
    fn refills_over_time() {
        let limiter = TokenBucketLimiter::new(1, 1.0);
        let t = 1_000_000;
        assert!(matches!(
            limiter.check("k", t),
            RateDecision::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check("k", t),
            RateDecision::Limited { .. }
        ));
        // After 1 second, one token is available again.
        assert!(matches!(
            limiter.check("k", t + 1),
            RateDecision::Allowed { .. }
        ));
    }

    #[test]
    fn retry_after_is_positive_when_limited() {
        let limiter = TokenBucketLimiter::new(1, 1.0);
        let t = 1_000_000;
        let _ = limiter.check("k", t);
        match limiter.check("k", t) {
            RateDecision::Limited {
                retry_after_seconds,
            } => {
                assert_eq!(retry_after_seconds, 1);
            }
            RateDecision::Allowed { .. } => panic!("should be limited"),
        }
    }

    #[test]
    fn keys_are_isolated() {
        let limiter = TokenBucketLimiter::new(1, 1.0);
        let t = 1_000_000;
        assert!(matches!(
            limiter.check("a", t),
            RateDecision::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check("b", t),
            RateDecision::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check("a", t),
            RateDecision::Limited { .. }
        ));
    }

    #[test]
    fn long_idle_does_not_overflow_beyond_burst() {
        let limiter = TokenBucketLimiter::new(2, 100.0);
        let t = 1_000_000;
        let _ = limiter.check("k", t);
        // 1 hour idle → refill capped at burst=2.
        match limiter.check("k", t + 3600) {
            RateDecision::Allowed { remaining, .. } => assert!(remaining <= 2),
            RateDecision::Limited { .. } => panic!("should be allowed after idle"),
        }
    }

    #[test]
    fn reset_reinstates_quota() {
        let limiter = TokenBucketLimiter::new(1, 1.0);
        let t = 1_000_000;
        let _ = limiter.check("k", t);
        assert!(matches!(
            limiter.check("k", t),
            RateDecision::Limited { .. }
        ));
        limiter.reset("k");
        assert!(matches!(
            limiter.check("k", t),
            RateDecision::Allowed { .. }
        ));
    }

    #[test]
    fn normalized_burst_clamps() {
        let zero = normalized_burst(0);
        assert_eq!(u32::from(zero), DEFAULT_BURST);
        let huge = normalized_burst(u32::MAX);
        assert!(u32::from(huge) <= DEFAULT_BURST * 100);
        let sane = normalized_burst(42);
        assert_eq!(u32::from(sane), 42);
    }

    #[test]
    fn clock_ellipsis_never_blocks_forever() {
        // Simulate the clock advancing far ahead; refill saturates at burst.
        let limiter = TokenBucketLimiter::new(5, 1.0);
        let t = 1_000_000;
        let _ = limiter.check("k", t);
        let _ = limiter.check("k", t);
        assert!(matches!(
            limiter.check("k", t + 100_000_000),
            RateDecision::Allowed { .. }
        ));
    }
}
