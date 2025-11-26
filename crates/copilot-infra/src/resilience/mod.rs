//! Resilience patterns for fault-tolerant services
//!
//! Provides circuit breaker and retry policies for handling transient failures.

pub mod circuit_breaker;
pub mod retry;
pub mod bulkhead;
pub mod timeout;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
pub use retry::{RetryPolicy, RetryConfig, ExponentialBackoff, FixedDelay};
pub use bulkhead::{Bulkhead, BulkheadConfig};
pub use timeout::{TimeoutPolicy, TimeoutError};

use std::future::Future;

/// A resilient operation that combines multiple resilience patterns
pub type ResilienceResult<T, E> = std::result::Result<T, ResilienceError<E>>;

/// Errors that can occur in resilient operations
#[derive(Debug)]
pub enum ResilienceError<E> {
    /// Circuit breaker is open
    CircuitOpen,
    /// Operation timed out
    Timeout,
    /// Bulkhead rejected (too many concurrent operations)
    BulkheadRejected,
    /// All retries exhausted
    RetriesExhausted(E),
    /// The underlying operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for ResilienceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResilienceError::CircuitOpen => write!(f, "Circuit breaker is open"),
            ResilienceError::Timeout => write!(f, "Operation timed out"),
            ResilienceError::BulkheadRejected => write!(f, "Bulkhead rejected request"),
            ResilienceError::RetriesExhausted(e) => write!(f, "Retries exhausted: {}", e),
            ResilienceError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ResilienceError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResilienceError::RetriesExhausted(e) | ResilienceError::OperationFailed(e) => Some(e),
            _ => None,
        }
    }
}

/// A builder for creating resilient operations
pub struct ResilienceBuilder<E> {
    circuit_breaker: Option<CircuitBreaker>,
    retry_policy: Option<RetryPolicy>,
    bulkhead: Option<Bulkhead>,
    timeout: Option<std::time::Duration>,
    _marker: std::marker::PhantomData<E>,
}

impl<E: std::error::Error + Clone + Send + 'static> ResilienceBuilder<E> {
    /// Create a new resilience builder
    pub fn new() -> Self {
        Self {
            circuit_breaker: None,
            retry_policy: None,
            bulkhead: None,
            timeout: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Add a circuit breaker
    pub fn with_circuit_breaker(mut self, cb: CircuitBreaker) -> Self {
        self.circuit_breaker = Some(cb);
        self
    }

    /// Add a retry policy
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Add a bulkhead
    pub fn with_bulkhead(mut self, bulkhead: Bulkhead) -> Self {
        self.bulkhead = Some(bulkhead);
        self
    }

    /// Add a timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Execute an operation with the configured resilience patterns
    pub async fn execute<F, Fut, T>(&self, operation: F) -> ResilienceResult<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
    {
        // Check bulkhead first
        let _permit = if let Some(ref bulkhead) = self.bulkhead {
            match bulkhead.acquire().await {
                Some(permit) => Some(permit),
                None => return Err(ResilienceError::BulkheadRejected),
            }
        } else {
            None
        };

        // Check circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.allow_request().await {
                return Err(ResilienceError::CircuitOpen);
            }
        }

        // Execute with retry
        let result = if let Some(ref retry) = self.retry_policy {
            self.execute_with_retry(retry, &operation).await
        } else {
            self.execute_once(&operation).await
        };

        // Record result in circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            match &result {
                Ok(_) => cb.record_success().await,
                Err(_) => cb.record_failure().await,
            }
        }

        result
    }

    async fn execute_once<F, Fut, T>(&self, operation: &F) -> ResilienceResult<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
    {
        let fut = operation();

        if let Some(timeout_duration) = self.timeout {
            match tokio::time::timeout(timeout_duration, fut).await {
                Ok(result) => result.map_err(ResilienceError::OperationFailed),
                Err(_) => Err(ResilienceError::Timeout),
            }
        } else {
            fut.await.map_err(ResilienceError::OperationFailed)
        }
    }

    async fn execute_with_retry<F, Fut, T>(
        &self,
        retry: &RetryPolicy,
        operation: &F,
    ) -> ResilienceResult<T, E>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send,
    {
        let mut last_error = None;
        let mut attempt = 0;

        while attempt <= retry.config().max_retries {
            // Check circuit breaker for each attempt
            if let Some(ref cb) = self.circuit_breaker {
                if !cb.allow_request().await {
                    return Err(ResilienceError::CircuitOpen);
                }
            }

            let result = self.execute_once(operation).await;

            match result {
                Ok(value) => return Ok(value),
                Err(ResilienceError::CircuitOpen) => return Err(ResilienceError::CircuitOpen),
                Err(ResilienceError::BulkheadRejected) => {
                    return Err(ResilienceError::BulkheadRejected)
                }
                Err(ResilienceError::Timeout) if !retry.config().retry_on_timeout => {
                    return Err(ResilienceError::Timeout);
                }
                Err(e) => {
                    let err = match e {
                        ResilienceError::OperationFailed(e) => e,
                        ResilienceError::Timeout => {
                            attempt += 1;
                            if attempt <= retry.config().max_retries {
                                tokio::time::sleep(retry.delay_for_attempt(attempt)).await;
                            }
                            continue;
                        }
                        other => return Err(other),
                    };

                    last_error = Some(err.clone());
                    attempt += 1;

                    if attempt <= retry.config().max_retries {
                        // Wait before retrying
                        tokio::time::sleep(retry.delay_for_attempt(attempt)).await;
                    }
                }
            }
        }

        Err(ResilienceError::RetriesExhausted(last_error.unwrap()))
    }
}

impl<E: std::error::Error + Clone + Send + 'static> Default for ResilienceBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}
