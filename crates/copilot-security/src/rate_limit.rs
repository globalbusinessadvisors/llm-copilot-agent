//! Rate limiting
//!
//! Provides rate limiting for API endpoints with support for
//! per-user, per-IP, and per-API-key limits.

use crate::error::{Result, SecurityError};
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Default requests per second
    pub default_rps: u32,
    /// Default burst size
    pub default_burst: u32,
    /// Window duration in seconds
    pub window_secs: u64,
    /// Whether to enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_rps: 10,
            default_burst: 20,
            window_secs: 1,
            enabled: true,
        }
    }
}

/// Rate limit tier for different user/key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitTier {
    /// Anonymous/unauthenticated requests
    Anonymous,
    /// Free tier users
    Free,
    /// Standard tier users
    Standard,
    /// Pro tier users
    Pro,
    /// Enterprise tier users
    Enterprise,
    /// Unlimited (for internal services)
    Unlimited,
}

impl RateLimitTier {
    /// Get requests per minute for this tier
    pub fn requests_per_minute(&self) -> Option<u32> {
        match self {
            RateLimitTier::Anonymous => Some(10),
            RateLimitTier::Free => Some(60),
            RateLimitTier::Standard => Some(300),
            RateLimitTier::Pro => Some(1000),
            RateLimitTier::Enterprise => Some(5000),
            RateLimitTier::Unlimited => None,
        }
    }

    /// Get burst size for this tier
    pub fn burst_size(&self) -> Option<u32> {
        match self {
            RateLimitTier::Anonymous => Some(5),
            RateLimitTier::Free => Some(20),
            RateLimitTier::Standard => Some(50),
            RateLimitTier::Pro => Some(200),
            RateLimitTier::Enterprise => Some(1000),
            RateLimitTier::Unlimited => None,
        }
    }

    /// Get from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "anonymous" => RateLimitTier::Anonymous,
            "free" => RateLimitTier::Free,
            "standard" => RateLimitTier::Standard,
            "pro" => RateLimitTier::Pro,
            "enterprise" => RateLimitTier::Enterprise,
            "unlimited" => RateLimitTier::Unlimited,
            _ => RateLimitTier::Free,
        }
    }
}

/// Rate limit key for identifying limit buckets
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    /// Limit by IP address
    Ip(String),
    /// Limit by user ID
    User(String),
    /// Limit by API key
    ApiKey(String),
    /// Limit by endpoint
    Endpoint(String),
    /// Composite key
    Composite(String, String),
}

impl RateLimitKey {
    /// Create a key string
    pub fn to_key_string(&self) -> String {
        match self {
            RateLimitKey::Ip(ip) => format!("ip:{}", ip),
            RateLimitKey::User(user_id) => format!("user:{}", user_id),
            RateLimitKey::ApiKey(key_id) => format!("apikey:{}", key_id),
            RateLimitKey::Endpoint(endpoint) => format!("endpoint:{}", endpoint),
            RateLimitKey::Composite(a, b) => format!("{}:{}", a, b),
        }
    }
}

/// Rate limit result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in the current window
    pub remaining: u32,
    /// Total limit
    pub limit: u32,
    /// Seconds until limit resets
    pub reset_after_secs: u64,
    /// Retry-after header value (if rate limited)
    pub retry_after_secs: Option<u64>,
}

/// Simple in-memory rate limiter
type SimpleRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Rate limiter manager
pub struct RateLimitManager {
    config: RateLimitConfig,
    /// Per-key rate limiters
    limiters: Arc<RwLock<HashMap<String, Arc<SimpleRateLimiter>>>>,
    /// Per-key quotas (for custom limits)
    quotas: Arc<RwLock<HashMap<String, Quota>>>,
}

impl RateLimitManager {
    /// Create a new rate limit manager
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            limiters: Arc::new(RwLock::new(HashMap::new())),
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Create a quota from tier
    fn quota_from_tier(&self, tier: &RateLimitTier) -> Option<Quota> {
        let rpm = tier.requests_per_minute()?;
        let burst = tier.burst_size()?;

        Some(
            Quota::per_minute(NonZeroU32::new(rpm)?)
                .allow_burst(NonZeroU32::new(burst)?),
        )
    }

    /// Create a quota from custom values
    fn quota_from_values(&self, rpm: u32, burst: u32) -> Option<Quota> {
        Some(
            Quota::per_minute(NonZeroU32::new(rpm)?)
                .allow_burst(NonZeroU32::new(burst)?),
        )
    }

    /// Get or create a rate limiter for a key
    async fn get_or_create_limiter(
        &self,
        key: &str,
        quota: Quota,
    ) -> Arc<SimpleRateLimiter> {
        // Check if limiter exists
        {
            let limiters = self.limiters.read().await;
            if let Some(limiter) = limiters.get(key) {
                return Arc::clone(limiter);
            }
        }

        // Create new limiter
        let limiter = Arc::new(RateLimiter::direct(quota));

        // Store it
        {
            let mut limiters = self.limiters.write().await;
            limiters.insert(key.to_string(), Arc::clone(&limiter));
        }

        limiter
    }

    /// Check rate limit for a key with a tier
    pub async fn check_limit(
        &self,
        key: &RateLimitKey,
        tier: &RateLimitTier,
    ) -> Result<RateLimitResult> {
        if !self.config.enabled {
            return Ok(RateLimitResult {
                allowed: true,
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after_secs: 0,
                retry_after_secs: None,
            });
        }

        // Unlimited tier bypasses rate limiting
        if *tier == RateLimitTier::Unlimited {
            return Ok(RateLimitResult {
                allowed: true,
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after_secs: 0,
                retry_after_secs: None,
            });
        }

        let key_str = key.to_key_string();
        let quota = self.quota_from_tier(tier).ok_or_else(|| {
            SecurityError::Configuration("Invalid rate limit configuration".to_string())
        })?;

        let limiter = self.get_or_create_limiter(&key_str, quota).await;

        let rpm = tier.requests_per_minute().unwrap_or(60);
        let burst = tier.burst_size().unwrap_or(10);

        match limiter.check() {
            Ok(()) => Ok(RateLimitResult {
                allowed: true,
                remaining: burst.saturating_sub(1),
                limit: rpm,
                reset_after_secs: 60,
                retry_after_secs: None,
            }),
            Err(not_until) => {
                let retry_after = not_until
                    .wait_time_from(governor::clock::Clock::now(&DefaultClock::default()))
                    .as_secs();

                Err(SecurityError::RateLimitExceeded {
                    retry_after_secs: retry_after,
                })
            }
        }
    }

    /// Check rate limit with custom values
    pub async fn check_limit_custom(
        &self,
        key: &RateLimitKey,
        rpm: u32,
        burst: u32,
    ) -> Result<RateLimitResult> {
        if !self.config.enabled {
            return Ok(RateLimitResult {
                allowed: true,
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after_secs: 0,
                retry_after_secs: None,
            });
        }

        let key_str = key.to_key_string();
        let quota = self.quota_from_values(rpm, burst).ok_or_else(|| {
            SecurityError::Configuration("Invalid rate limit values".to_string())
        })?;

        let limiter = self.get_or_create_limiter(&key_str, quota).await;

        match limiter.check() {
            Ok(()) => Ok(RateLimitResult {
                allowed: true,
                remaining: burst.saturating_sub(1),
                limit: rpm,
                reset_after_secs: 60,
                retry_after_secs: None,
            }),
            Err(not_until) => {
                let retry_after = not_until
                    .wait_time_from(governor::clock::Clock::now(&DefaultClock::default()))
                    .as_secs();

                Err(SecurityError::RateLimitExceeded {
                    retry_after_secs: retry_after,
                })
            }
        }
    }

    /// Set a custom quota for a key
    pub async fn set_custom_quota(&self, key: &str, rpm: u32, burst: u32) -> Result<()> {
        let quota = self.quota_from_values(rpm, burst).ok_or_else(|| {
            SecurityError::Configuration("Invalid quota values".to_string())
        })?;

        let mut quotas = self.quotas.write().await;
        quotas.insert(key.to_string(), quota);

        Ok(())
    }

    /// Clear rate limit state for a key
    pub async fn clear_limit(&self, key: &RateLimitKey) {
        let key_str = key.to_key_string();
        let mut limiters = self.limiters.write().await;
        limiters.remove(&key_str);
    }

    /// Clear all rate limit state
    pub async fn clear_all(&self) {
        let mut limiters = self.limiters.write().await;
        limiters.clear();
    }

    /// Get rate limit headers for HTTP response
    pub fn get_headers(&self, result: &RateLimitResult) -> Vec<(String, String)> {
        let mut headers = vec![
            ("X-RateLimit-Limit".to_string(), result.limit.to_string()),
            (
                "X-RateLimit-Remaining".to_string(),
                result.remaining.to_string(),
            ),
            (
                "X-RateLimit-Reset".to_string(),
                result.reset_after_secs.to_string(),
            ),
        ];

        if let Some(retry_after) = result.retry_after_secs {
            headers.push(("Retry-After".to_string(), retry_after.to_string()));
        }

        headers
    }
}

impl Clone for RateLimitManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            limiters: Arc::clone(&self.limiters),
            quotas: Arc::clone(&self.quotas),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let manager = RateLimitManager::new(RateLimitConfig {
            enabled: true,
            ..Default::default()
        });

        let key = RateLimitKey::Ip("127.0.0.1".to_string());
        let tier = RateLimitTier::Free;

        // First few requests should succeed
        for _ in 0..5 {
            let result = manager.check_limit(&key, &tier).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_unlimited_tier() {
        let manager = RateLimitManager::default_config();

        let key = RateLimitKey::User("admin".to_string());
        let tier = RateLimitTier::Unlimited;

        // Should always succeed
        for _ in 0..1000 {
            let result = manager.check_limit(&key, &tier).await;
            assert!(result.is_ok());
            assert!(result.unwrap().allowed);
        }
    }

    #[tokio::test]
    async fn test_disabled_rate_limiting() {
        let manager = RateLimitManager::new(RateLimitConfig {
            enabled: false,
            ..Default::default()
        });

        let key = RateLimitKey::Ip("127.0.0.1".to_string());
        let tier = RateLimitTier::Anonymous;

        // Should always succeed when disabled
        for _ in 0..100 {
            let result = manager.check_limit(&key, &tier).await;
            assert!(result.is_ok());
            assert!(result.unwrap().allowed);
        }
    }

    #[test]
    fn test_rate_limit_tiers() {
        assert!(RateLimitTier::Anonymous.requests_per_minute() < RateLimitTier::Free.requests_per_minute());
        assert!(RateLimitTier::Free.requests_per_minute() < RateLimitTier::Standard.requests_per_minute());
        assert!(RateLimitTier::Standard.requests_per_minute() < RateLimitTier::Pro.requests_per_minute());
        assert!(RateLimitTier::Pro.requests_per_minute() < RateLimitTier::Enterprise.requests_per_minute());
        assert!(RateLimitTier::Unlimited.requests_per_minute().is_none());
    }

    #[test]
    fn test_rate_limit_key() {
        let ip_key = RateLimitKey::Ip("192.168.1.1".to_string());
        assert_eq!(ip_key.to_key_string(), "ip:192.168.1.1");

        let user_key = RateLimitKey::User("user-123".to_string());
        assert_eq!(user_key.to_key_string(), "user:user-123");
    }
}
