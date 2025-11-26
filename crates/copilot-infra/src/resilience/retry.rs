//! Retry policies
//!
//! Provides configurable retry strategies with exponential backoff.

use std::time::Duration;
use rand::Rng;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries (0 means no retries)
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Whether to add random jitter
    pub jitter: bool,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Whether to retry on timeout
    pub retry_on_timeout: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
            jitter_factor: 0.3,
            retry_on_timeout: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with max retries
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Create a config with no retries (for non-idempotent operations)
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set max delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Set jitter factor
    pub fn with_jitter_factor(mut self, factor: f64) -> Self {
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// Set whether to retry on timeout
    pub fn with_retry_on_timeout(mut self, retry: bool) -> Self {
        self.retry_on_timeout = retry;
        self
    }
}

/// Retry policy implementation
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    config: RetryConfig,
}

impl RetryPolicy {
    /// Create a new retry policy with the given configuration
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Create with fixed delay (no exponential backoff)
    pub fn fixed(max_retries: u32, delay: Duration) -> Self {
        Self::new(RetryConfig {
            max_retries,
            initial_delay: delay,
            max_delay: delay,
            multiplier: 1.0,
            jitter: false,
            ..Default::default()
        })
    }

    /// Create with exponential backoff
    pub fn exponential(max_retries: u32) -> Self {
        Self::new(RetryConfig::new(max_retries))
    }

    /// Get the configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Calculate the delay for a specific attempt (1-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        // Calculate base delay with exponential backoff
        let base_delay = self
            .config
            .initial_delay
            .mul_f64(self.config.multiplier.powi((attempt - 1) as i32));

        // Cap at max delay
        let delay = base_delay.min(self.config.max_delay);

        // Add jitter if enabled
        if self.config.jitter {
            self.add_jitter(delay)
        } else {
            delay
        }
    }

    /// Add random jitter to a delay
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_range = delay.mul_f64(self.config.jitter_factor);
        let jitter = rng.gen_range(Duration::ZERO..jitter_range);

        // Randomly add or subtract jitter
        if rng.gen_bool(0.5) {
            delay + jitter
        } else {
            delay.saturating_sub(jitter)
        }
    }

    /// Check if we should retry for a given attempt
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt <= self.config.max_retries
    }
}

/// Exponential backoff helper
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    policy: RetryPolicy,
    current_attempt: u32,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff
    pub fn new(max_retries: u32) -> Self {
        Self {
            policy: RetryPolicy::exponential(max_retries),
            current_attempt: 0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        Self {
            policy: RetryPolicy::new(config),
            current_attempt: 0,
        }
    }

    /// Get the next delay and increment the attempt counter
    /// Returns None if max retries exceeded
    pub fn next_delay(&mut self) -> Option<Duration> {
        self.current_attempt += 1;
        if self.policy.should_retry(self.current_attempt) {
            Some(self.policy.delay_for_attempt(self.current_attempt))
        } else {
            None
        }
    }

    /// Reset the backoff
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    /// Get current attempt number
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_delay()
    }
}

/// Fixed delay helper
#[derive(Debug, Clone)]
pub struct FixedDelay {
    delay: Duration,
    remaining: u32,
}

impl FixedDelay {
    /// Create a new fixed delay
    pub fn new(max_retries: u32, delay: Duration) -> Self {
        Self {
            delay,
            remaining: max_retries,
        }
    }

    /// Get the next delay
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.remaining > 0 {
            self.remaining -= 1;
            Some(self.delay)
        } else {
            None
        }
    }

    /// Reset the counter
    pub fn reset(&mut self, max_retries: u32) {
        self.remaining = max_retries;
    }
}

impl Iterator for FixedDelay {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_delay()
    }
}

/// Retry with a custom predicate
pub async fn retry_with<F, Fut, T, E, P>(
    policy: &RetryPolicy,
    mut operation: F,
    should_retry: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    let mut attempt = 0;
    let mut last_error;

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                last_error = e;

                if attempt >= policy.config.max_retries || !should_retry(&last_error) {
                    return Err(last_error);
                }

                attempt += 1;
                tokio::time::sleep(policy.delay_for_attempt(attempt)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert!(config.jitter);
    }

    #[test]
    fn test_exponential_backoff_delays() {
        let policy = RetryPolicy::new(RetryConfig {
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            jitter: false,
            ..Default::default()
        });

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy::new(RetryConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            multiplier: 10.0,
            jitter: false,
            ..Default::default()
        });

        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(5)); // Capped
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(5)); // Capped
    }

    #[test]
    fn test_fixed_delay() {
        let policy = RetryPolicy::fixed(3, Duration::from_millis(500));

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(500));
    }

    #[test]
    fn test_exponential_backoff_iterator() {
        let mut backoff = ExponentialBackoff::with_config(RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            multiplier: 2.0,
            jitter: false,
            ..Default::default()
        });

        assert_eq!(backoff.next(), Some(Duration::from_millis(100)));
        assert_eq!(backoff.next(), Some(Duration::from_millis(200)));
        assert_eq!(backoff.next(), Some(Duration::from_millis(400)));
        assert_eq!(backoff.next(), None);
    }

    #[test]
    fn test_fixed_delay_iterator() {
        let mut fixed = FixedDelay::new(2, Duration::from_millis(100));

        assert_eq!(fixed.next(), Some(Duration::from_millis(100)));
        assert_eq!(fixed.next(), Some(Duration::from_millis(100)));
        assert_eq!(fixed.next(), None);
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::new(RetryConfig::new(3));

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(policy.should_retry(3));
        assert!(!policy.should_retry(4));
        assert!(!policy.should_retry(5));
    }

    #[test]
    fn test_jitter_applied() {
        let policy = RetryPolicy::new(RetryConfig {
            initial_delay: Duration::from_secs(1),
            jitter: true,
            jitter_factor: 0.5,
            multiplier: 1.0,
            ..Default::default()
        });

        // With jitter, delays should vary
        let delays: Vec<_> = (0..100).map(|_| policy.delay_for_attempt(1)).collect();
        let min = delays.iter().min().unwrap();
        let max = delays.iter().max().unwrap();

        // Should have some variation
        assert!(max > min);
    }
}
