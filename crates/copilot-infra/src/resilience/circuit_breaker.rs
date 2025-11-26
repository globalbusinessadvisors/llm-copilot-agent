//! Circuit breaker implementation
//!
//! Prevents cascading failures by stopping operations when too many failures occur.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, allowing limited test requests
    HalfOpen,
}

impl std::fmt::Display for CircuitBreakerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerState::Closed => write!(f, "closed"),
            CircuitBreakerState::Open => write!(f, "open"),
            CircuitBreakerState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Name of this circuit breaker (for logging)
    pub name: String,
    /// Failure threshold before opening the circuit
    pub failure_threshold: u32,
    /// Success threshold to close the circuit from half-open
    pub success_threshold: u32,
    /// Duration to wait before transitioning from open to half-open
    pub open_duration: Duration,
    /// Time window for counting failures
    pub failure_window: Duration,
    /// Minimum number of requests before the circuit can open
    pub minimum_requests: u32,
    /// Failure rate percentage threshold (0.0 to 1.0)
    pub failure_rate_threshold: f64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            failure_threshold: 5,
            success_threshold: 3,
            open_duration: Duration::from_secs(30),
            failure_window: Duration::from_secs(60),
            minimum_requests: 10,
            failure_rate_threshold: 0.5,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker config with a name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Set failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set success threshold
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set open duration
    pub fn with_open_duration(mut self, duration: Duration) -> Self {
        self.open_duration = duration;
        self
    }

    /// Set failure window
    pub fn with_failure_window(mut self, duration: Duration) -> Self {
        self.failure_window = duration;
        self
    }

    /// Set minimum requests
    pub fn with_minimum_requests(mut self, min: u32) -> Self {
        self.minimum_requests = min;
        self
    }

    /// Set failure rate threshold
    pub fn with_failure_rate_threshold(mut self, rate: f64) -> Self {
        self.failure_rate_threshold = rate.clamp(0.0, 1.0);
        self
    }
}

/// Internal state for the circuit breaker
struct CircuitBreakerInner {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitBreakerState>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    total_count: AtomicUsize,
    last_failure_time: RwLock<Option<Instant>>,
    opened_at: RwLock<Option<Instant>>,
    window_start: RwLock<Instant>,
}

/// Thread-safe circuit breaker
#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<CircuitBreakerInner>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(CircuitBreakerInner {
                config,
                state: RwLock::new(CircuitBreakerState::Closed),
                failure_count: AtomicUsize::new(0),
                success_count: AtomicUsize::new(0),
                total_count: AtomicUsize::new(0),
                last_failure_time: RwLock::new(None),
                opened_at: RwLock::new(None),
                window_start: RwLock::new(Instant::now()),
            }),
        }
    }

    /// Create with default configuration
    pub fn default_config(name: &str) -> Self {
        Self::new(CircuitBreakerConfig::new(name))
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitBreakerState {
        *self.inner.state.read().await
    }

    /// Check if a request should be allowed
    pub async fn allow_request(&self) -> bool {
        let state = *self.inner.state.read().await;

        match state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                if self.should_attempt_reset().await {
                    self.transition_to_half_open().await;
                    true
                } else {
                    debug!(
                        circuit_breaker = %self.inner.config.name,
                        "Request rejected, circuit is open"
                    );
                    false
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow limited requests in half-open state
                true
            }
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        self.inner.total_count.fetch_add(1, Ordering::SeqCst);
        self.inner.success_count.fetch_add(1, Ordering::SeqCst);

        let state = *self.inner.state.read().await;

        match state {
            CircuitBreakerState::HalfOpen => {
                let successes = self.inner.success_count.load(Ordering::SeqCst);
                if successes >= self.inner.config.success_threshold as usize {
                    self.transition_to_closed().await;
                }
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                self.maybe_reset_window().await;
            }
            CircuitBreakerState::Open => {
                // Shouldn't happen, but handle it gracefully
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        self.inner.total_count.fetch_add(1, Ordering::SeqCst);
        self.inner.failure_count.fetch_add(1, Ordering::SeqCst);
        *self.inner.last_failure_time.write().await = Some(Instant::now());

        let state = *self.inner.state.read().await;

        match state {
            CircuitBreakerState::Closed => {
                self.maybe_reset_window().await;
                self.check_failure_threshold().await;
            }
            CircuitBreakerState::HalfOpen => {
                // Immediately transition back to open on any failure
                self.transition_to_open().await;
            }
            CircuitBreakerState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Check if we should transition from open to half-open
    async fn should_attempt_reset(&self) -> bool {
        if let Some(opened_at) = *self.inner.opened_at.read().await {
            opened_at.elapsed() >= self.inner.config.open_duration
        } else {
            true
        }
    }

    /// Check if failure threshold is exceeded
    async fn check_failure_threshold(&self) {
        let total = self.inner.total_count.load(Ordering::SeqCst) as u32;
        let failures = self.inner.failure_count.load(Ordering::SeqCst) as u32;

        // Check minimum requests
        if total < self.inner.config.minimum_requests {
            return;
        }

        // Check failure count threshold
        if failures >= self.inner.config.failure_threshold {
            self.transition_to_open().await;
            return;
        }

        // Check failure rate threshold
        let failure_rate = failures as f64 / total as f64;
        if failure_rate >= self.inner.config.failure_rate_threshold {
            self.transition_to_open().await;
        }
    }

    /// Maybe reset the failure window
    async fn maybe_reset_window(&self) {
        let window_start = *self.inner.window_start.read().await;
        if window_start.elapsed() >= self.inner.config.failure_window {
            self.reset_counters().await;
        }
    }

    /// Reset counters
    async fn reset_counters(&self) {
        self.inner.failure_count.store(0, Ordering::SeqCst);
        self.inner.success_count.store(0, Ordering::SeqCst);
        self.inner.total_count.store(0, Ordering::SeqCst);
        *self.inner.window_start.write().await = Instant::now();
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.inner.state.write().await;
        if *state != CircuitBreakerState::Closed {
            info!(
                circuit_breaker = %self.inner.config.name,
                from = %*state,
                to = "closed",
                "Circuit breaker state transition"
            );
            *state = CircuitBreakerState::Closed;
            drop(state);
            self.reset_counters().await;
            *self.inner.opened_at.write().await = None;
        }
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.inner.state.write().await;
        if *state != CircuitBreakerState::Open {
            warn!(
                circuit_breaker = %self.inner.config.name,
                from = %*state,
                to = "open",
                failures = self.inner.failure_count.load(Ordering::SeqCst),
                total = self.inner.total_count.load(Ordering::SeqCst),
                "Circuit breaker state transition"
            );
            *state = CircuitBreakerState::Open;
            *self.inner.opened_at.write().await = Some(Instant::now());
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.inner.state.write().await;
        if *state == CircuitBreakerState::Open {
            info!(
                circuit_breaker = %self.inner.config.name,
                from = "open",
                to = "half-open",
                "Circuit breaker state transition"
            );
            *state = CircuitBreakerState::HalfOpen;
            drop(state);
            // Reset counters for half-open testing
            self.inner.success_count.store(0, Ordering::SeqCst);
            self.inner.failure_count.store(0, Ordering::SeqCst);
        }
    }

    /// Get circuit breaker statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.inner.config.name.clone(),
            failure_count: self.inner.failure_count.load(Ordering::SeqCst),
            success_count: self.inner.success_count.load(Ordering::SeqCst),
            total_count: self.inner.total_count.load(Ordering::SeqCst),
        }
    }

    /// Force the circuit to open (for testing or manual intervention)
    pub async fn force_open(&self) {
        self.transition_to_open().await;
    }

    /// Force the circuit to close (for testing or manual intervention)
    pub async fn force_close(&self) {
        self.transition_to_closed().await;
    }
}

/// Statistics for a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Name of the circuit breaker
    pub name: String,
    /// Current failure count
    pub failure_count: usize,
    /// Current success count
    pub success_count: usize,
    /// Total request count
    pub total_count: usize,
}

impl CircuitBreakerStats {
    /// Get the failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.failure_count as f64 / self.total_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state().await, CircuitBreakerState::Closed);
        assert!(cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_opens_on_failures() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(3)
            .with_minimum_requests(1);

        let cb = CircuitBreaker::new(config);

        // Record failures
        for _ in 0..3 {
            cb.record_failure().await;
        }

        assert_eq!(cb.state().await, CircuitBreakerState::Open);
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_minimum_requests(1)
            .with_open_duration(Duration::from_millis(10));

        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;

        assert_eq!(cb.state().await, CircuitBreakerState::Open);

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should transition to half-open on next request attempt
        assert!(cb.allow_request().await);
        assert_eq!(cb.state().await, CircuitBreakerState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_closes_on_success() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_success_threshold(2)
            .with_minimum_requests(1)
            .with_open_duration(Duration::from_millis(10));

        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Transition to half-open
        cb.allow_request().await;

        // Record successes
        cb.record_success().await;
        cb.record_success().await;

        assert_eq!(cb.state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_fails_reopens() {
        let config = CircuitBreakerConfig::default()
            .with_failure_threshold(2)
            .with_minimum_requests(1)
            .with_open_duration(Duration::from_millis(10));

        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait for open duration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Transition to half-open
        cb.allow_request().await;

        assert_eq!(cb.state().await, CircuitBreakerState::HalfOpen);

        // Record failure in half-open state
        cb.record_failure().await;

        assert_eq!(cb.state().await, CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_stats() {
        let cb = CircuitBreaker::default_config("test");

        cb.record_success().await;
        cb.record_success().await;
        cb.record_failure().await;

        let stats = cb.stats();
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.total_count, 3);
        assert!((stats.failure_rate() - 0.333).abs() < 0.01);
    }
}
