//! Rate limiter to prevent abuse and ensure fair usage
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed per window
    pub max_requests: u32,
    /// Time window for rate limiting
    pub window: Duration,
}

/// Tracks requests for a specific identifier
#[derive(Debug)]
struct RequestTracker {
    /// Timestamps of recent requests
    timestamps: Vec<Instant>,
    /// Config for this tracker
    config: RateLimitConfig,
}

impl RequestTracker {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            timestamps: Vec::new(),
            config,
        }
    }

    /// Check if a request is allowed and record it
    fn allow_request(&mut self) -> bool {
        let now = Instant::now();
        let window_start = now - self.config.window;

        // Remove expired timestamps
        self.timestamps
            .retain(|&timestamp| timestamp > window_start);

        // Check if we're under the limit
        if self.timestamps.len() < self.config.max_requests as usize {
            self.timestamps.push(now);
            true
        } else {
            false
        }
    }
}

/// Global rate limiter
pub struct RateLimiter {
    trackers: Arc<Mutex<HashMap<String, RequestTracker>>>,
    default_config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with default configuration
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            trackers: Arc::new(Mutex::new(HashMap::new())),
            default_config: RateLimitConfig {
                max_requests,
                window: Duration::from_secs(window_seconds),
            },
        }
    }

    /// Check if an action is allowed for a specific identifier
    pub async fn is_allowed(&self, identifier: String) -> bool {
        let mut trackers = self.trackers.lock().await;

        let tracker = trackers
            .entry(identifier.clone())
            .or_insert_with(|| RequestTracker::new(self.default_config.clone()));

        tracker.allow_request()
    }

    /// Reset rate limit tracking for an identifier
    pub async fn reset(&self, identifier: &str) {
        let mut trackers = self.trackers.lock().await;
        trackers.remove(identifier);
    }

    /// Get current request count for an identifier
    pub async fn request_count(&self, identifier: &str) -> usize {
        let trackers = self.trackers.lock().await;
        if let Some(tracker) = trackers.get(identifier) {
            let now = Instant::now();
            let window_start = now - tracker.config.window;
            tracker
                .timestamps
                .iter()
                .filter(|&&t| t > window_start)
                .count()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = RateLimiter::new(3, 1); // 3 requests per second

        let identifier = "test_user";

        // First 3 requests should be allowed
        assert!(limiter.is_allowed(identifier.to_string()).await);
        assert!(limiter.is_allowed(identifier.to_string()).await);
        assert!(limiter.is_allowed(identifier.to_string()).await);

        // 4th request should be blocked
        assert!(!limiter.is_allowed(identifier.to_string()).await);
    }

    #[tokio::test]
    async fn test_rate_limit_expiration() {
        let limiter = RateLimiter::new(2, 1); // 2 requests per second

        let identifier = "test_user2";

        // First 2 requests should be allowed
        assert!(limiter.is_allowed(identifier.to_string()).await);
        assert!(limiter.is_allowed(identifier.to_string()).await);

        // 3rd request should be blocked
        assert!(!limiter.is_allowed(identifier.to_string()).await);

        // Wait for window to expire
        sleep(Duration::from_millis(1100)).await;

        // Should be allowed again
        assert!(limiter.is_allowed(identifier.to_string()).await);
    }
}
