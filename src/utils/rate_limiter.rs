// src/utils/rate_limiter.rs

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Simple token bucket rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    config: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: usize,
    /// Time window duration
    pub window: Duration,
    /// Whether to enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: usize,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if a request from the given identifier should be allowed
    /// Returns Ok(()) if allowed, Err with retry-after seconds if rate limited
    pub async fn check_rate_limit(&self, identifier: &str) -> Result<(), u64> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        let bucket = buckets.entry(identifier.to_string()).or_insert(TokenBucket {
            tokens: self.config.max_requests,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        if elapsed >= self.config.window {
            // Full refill
            bucket.tokens = self.config.max_requests;
            bucket.last_refill = now;
        } else {
            // Partial refill based on time passed
            let refill_rate = self.config.max_requests as f64 / self.config.window.as_secs_f64();
            let tokens_to_add = (elapsed.as_secs_f64() * refill_rate) as usize;
            bucket.tokens = (bucket.tokens + tokens_to_add).min(self.config.max_requests);
            if tokens_to_add > 0 {
                bucket.last_refill = now;
            }
        }

        // Check if we have tokens available
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            Ok(())
        } else {
            // Calculate retry-after in seconds
            let time_until_refill = self.config.window
                .checked_sub(now.duration_since(bucket.last_refill))
                .unwrap_or(Duration::from_secs(1));
            Err(time_until_refill.as_secs().max(1))
        }
    }

    /// Check rate limit for an IP address
    pub async fn check_ip(&self, ip: IpAddr) -> Result<(), u64> {
        self.check_rate_limit(&ip.to_string()).await
    }

    /// Check rate limit for an endpoint (combination of IP and path)
    pub async fn check_endpoint(&self, ip: IpAddr, endpoint: &str) -> Result<(), u64> {
        let identifier = format!("{}:{}", ip, endpoint);
        self.check_rate_limit(&identifier).await
    }

    /// Cleanup old buckets (call periodically)
    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let cleanup_threshold = self.config.window * 2;

        buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_refill) < cleanup_threshold
        });
    }

    /// Get current stats (for monitoring)
    pub async fn get_stats(&self) -> RateLimiterStats {
        let buckets = self.buckets.read().await;
        RateLimiterStats {
            total_buckets: buckets.len(),
            config: self.config.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub total_buckets: usize,
    pub config: RateLimitConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        // First 5 requests should succeed
        for i in 0..5 {
            assert!(
                limiter.check_rate_limit("test-client").await.is_ok(),
                "Request {} should succeed",
                i
            );
        }

        // 6th request should be rate limited
        assert!(limiter.check_rate_limit("test-client").await.is_err());

        // Wait for window to pass
        sleep(Duration::from_millis(1100)).await;

        // Should be able to make requests again
        assert!(limiter.check_rate_limit("test-client").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_clients() {
        let config = RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        // Client A can make 2 requests
        assert!(limiter.check_rate_limit("client-a").await.is_ok());
        assert!(limiter.check_rate_limit("client-a").await.is_ok());
        assert!(limiter.check_rate_limit("client-a").await.is_err());

        // Client B should still be able to make requests
        assert!(limiter.check_rate_limit("client-b").await.is_ok());
        assert!(limiter.check_rate_limit("client-b").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(1),
            enabled: false, // Disabled
        };
        let limiter = RateLimiter::new(config);

        // Should allow unlimited requests when disabled
        for _ in 0..100 {
            assert!(limiter.check_rate_limit("test-client").await.is_ok());
        }
    }
}
