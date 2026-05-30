//! Rate limiting implementation

use crate::error::{Result, SdkError};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Token bucket rate limiter
pub struct TokenBucket {
    capacity: u32,
    refill_rate: u32,
    tokens: Arc<AtomicU32>,
    last_refill: Arc<parking_lot::Mutex<Instant>>,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            refill_rate,
            tokens: Arc::new(AtomicU32::new(capacity)),
            last_refill: Arc::new(parking_lot::Mutex::new(Instant::now())),
        }
    }

    /// Try to acquire tokens
    pub fn try_acquire(&self, tokens: u32) -> Result<()> {
        self.refill();

        let current = self.tokens.load(Ordering::SeqCst);

        if current >= tokens {
            self.tokens
                .store(current - tokens, Ordering::SeqCst);
            Ok(())
        } else {
            Err(SdkError::RateLimitExceeded(format!(
                "Not enough tokens: required {}, available {}",
                tokens, current
            )))
        }
    }

    /// Refill tokens
    fn refill(&self) {
        let mut last_refill = self.last_refill.lock();
        let elapsed = last_refill.elapsed();

        let refill_tokens = (elapsed.as_millis() as u32 / 1000) * self.refill_rate;

        if refill_tokens > 0 {
            let current = self.tokens.load(Ordering::SeqCst);
            let new_tokens = std::cmp::min(current + refill_tokens, self.capacity);
            self.tokens.store(new_tokens, Ordering::SeqCst);
            *last_refill = Instant::now();
        }
    }

    /// Get current tokens
    pub fn current_tokens(&self) -> u32 {
        self.tokens.load(Ordering::SeqCst)
    }
}

impl Clone for TokenBucket {
    fn clone(&self) -> Self {
        Self {
            capacity: self.capacity,
            refill_rate: self.refill_rate,
            tokens: Arc::clone(&self.tokens),
            last_refill: Arc::clone(&self.last_refill),
        }
    }
}

/// Sliding window rate limiter
pub struct SlidingWindowRateLimiter {
    max_requests: u32,
    window_duration: Duration,
    requests: Arc<DashMap<String, Vec<Instant>>>,
}

impl SlidingWindowRateLimiter {
    /// Create new sliding window rate limiter
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            requests: Arc::new(DashMap::new()),
        }
    }

    /// Check if request is allowed
    pub fn allow_request(&self, client_id: &str) -> Result<()> {
        let now = Instant::now();

        let mut entry = self.requests.entry(client_id.to_string()).or_insert_with(Vec::new);

        // Remove expired requests
        entry.retain(|req_time| now.duration_since(*req_time) < self.window_duration);

        if entry.len() < self.max_requests as usize {
            entry.push(now);
            Ok(())
        } else {
            Err(SdkError::RateLimitExceeded(format!(
                "Rate limit exceeded: {} requests per {:?}",
                self.max_requests, self.window_duration
            )))
        }
    }

    /// Get remaining requests for client
    pub fn remaining_requests(&self, client_id: &str) -> u32 {
        if let Some(mut entry) = self.requests.get_mut(client_id) {
            let now = Instant::now();
            entry.retain(|req_time| now.duration_since(*req_time) < self.window_duration);
            (self.max_requests - entry.len() as u32).max(0)
        } else {
            self.max_requests
        }
    }
}

impl Clone for SlidingWindowRateLimiter {
    fn clone(&self) -> Self {
        Self {
            max_requests: self.max_requests,
            window_duration: self.window_duration,
            requests: Arc::clone(&self.requests),
        }
    }
}

/// Per-user rate limiter
pub struct PerUserRateLimiter {
    limiters: Arc<DashMap<String, TokenBucket>>,
    capacity: u32,
    refill_rate: u32,
}

impl PerUserRateLimiter {
    /// Create new per-user rate limiter
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            limiters: Arc::new(DashMap::new()),
            capacity,
            refill_rate,
        }
    }

    /// Try to acquire tokens for user
    pub fn try_acquire(&self, user_id: &str, tokens: u32) -> Result<()> {
        let limiter = self
            .limiters
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        limiter.try_acquire(tokens)
    }

    /// Get current tokens for user
    pub fn current_tokens(&self, user_id: &str) -> u32 {
        if let Some(limiter) = self.limiters.get(user_id) {
            limiter.current_tokens()
        } else {
            self.capacity
        }
    }
}

impl Clone for PerUserRateLimiter {
    fn clone(&self) -> Self {
        Self {
            limiters: Arc::clone(&self.limiters),
            capacity: self.capacity,
            refill_rate: self.refill_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let bucket = TokenBucket::new(10, 1);

        assert!(bucket.try_acquire(5).is_ok());
        assert_eq!(bucket.current_tokens(), 5);

        assert!(bucket.try_acquire(10).is_err());
    }

    #[test]
    fn test_sliding_window() {
        let limiter = SlidingWindowRateLimiter::new(3, Duration::from_secs(1));

        assert!(limiter.allow_request("client1").is_ok());
        assert!(limiter.allow_request("client1").is_ok());
        assert!(limiter.allow_request("client1").is_ok());
        assert!(limiter.allow_request("client1").is_err());

        assert_eq!(limiter.remaining_requests("client1"), 0);
    }

    #[test]
    fn test_per_user_rate_limiter() {
        let limiter = PerUserRateLimiter::new(10, 1);

        assert!(limiter.try_acquire("user1", 5).is_ok());
        assert_eq!(limiter.current_tokens("user1"), 5);

        assert!(limiter.try_acquire("user2", 5).is_ok());
        assert_eq!(limiter.current_tokens("user2"), 5);
    }
}
