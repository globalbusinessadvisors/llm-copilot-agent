use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::{AdapterError, AdapterResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub timeout: Duration,
    pub half_open_max_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

struct CircuitBreakerState {
    state: CircuitState,
    failure_count: usize,
    success_count: usize,
    last_failure_time: Option<Instant>,
    half_open_requests: usize,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                half_open_requests: 0,
            })),
        }
    }

    pub async fn call<F, Fut, T>(&self, f: F) -> AdapterResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = AdapterResult<T>>,
    {
        // Check if we can proceed
        {
            let mut state = self.state.lock().await;

            match state.state {
                CircuitState::Open => {
                    // Check if timeout has elapsed
                    if let Some(last_failure) = state.last_failure_time {
                        if last_failure.elapsed() >= self.config.timeout {
                            debug!("Circuit breaker transitioning to half-open state");
                            state.state = CircuitState::HalfOpen;
                            state.half_open_requests = 0;
                            state.success_count = 0;
                        } else {
                            warn!("Circuit breaker is open, rejecting request");
                            return Err(AdapterError::CircuitBreakerOpen);
                        }
                    }
                }
                CircuitState::HalfOpen => {
                    if state.half_open_requests >= self.config.half_open_max_requests {
                        warn!("Circuit breaker half-open limit reached, rejecting request");
                        return Err(AdapterError::CircuitBreakerOpen);
                    }
                    state.half_open_requests += 1;
                }
                CircuitState::Closed => {
                    // Allow the request
                }
            }
        }

        // Execute the function
        let result = f().await;

        // Update state based on result
        match &result {
            Ok(_) => {
                self.record_success().await;
            }
            Err(_) => {
                self.record_failure().await;
            }
        }

        result
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::HalfOpen => {
                state.success_count += 1;

                if state.success_count >= self.config.success_threshold {
                    debug!("Circuit breaker transitioning to closed state after {} successes",
                           state.success_count);
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.half_open_requests = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                if state.failure_count > 0 {
                    debug!("Resetting failure count after success");
                    state.failure_count = 0;
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                state.failure_count = 0;
            }
        }
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;

        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        match state.state {
            CircuitState::HalfOpen => {
                warn!("Circuit breaker transitioning to open state after failure in half-open state");
                state.state = CircuitState::Open;
                state.success_count = 0;
                state.half_open_requests = 0;
            }
            CircuitState::Closed => {
                if state.failure_count >= self.config.failure_threshold {
                    warn!("Circuit breaker transitioning to open state after {} failures",
                          state.failure_count);
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.lock().await.state
    }

    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        debug!("Manually resetting circuit breaker");
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
        state.half_open_requests = 0;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Record failures
        for _ in 0..3 {
            cb.record_failure().await;
        }

        assert_eq!(cb.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Record failures to open circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Next call should transition to half-open
        let result = cb.call(|| async {
            Ok::<(), AdapterError>(())
        }).await;

        assert!(result.is_ok());
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Transition to half-open and record successes
        let _ = cb.call(|| async { Ok::<(), AdapterError>(()) }).await;
        cb.record_success().await;

        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::default();

        cb.record_failure().await;
        cb.record_failure().await;

        cb.reset().await;

        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}
