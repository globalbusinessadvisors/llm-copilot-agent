use std::time::Duration;
use tracing::{debug, warn};
use rand::Rng;

use crate::{AdapterError, AdapterResult};

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    pub fn with_backoff(mut self, initial: Duration, max: Duration) -> Self {
        self.initial_backoff = initial;
        self.max_backoff = max;
        self
    }

    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    fn calculate_backoff(&self, attempt: usize) -> Duration {
        let base_backoff = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);

        let backoff = base_backoff.min(self.max_backoff.as_millis() as f64);

        let backoff = if self.jitter {
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.5..1.5);
            backoff * jitter_factor
        } else {
            backoff
        };

        Duration::from_millis(backoff as u64)
    }

    pub async fn execute<F, Fut, T>(&self, mut f: F) -> AdapterResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = AdapterResult<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                let backoff = self.calculate_backoff(attempt - 1);
                debug!(
                    "Retry attempt {}/{}, backing off for {:?}",
                    attempt + 1,
                    self.max_attempts,
                    backoff
                );
                tokio::time::sleep(backoff).await;
            }

            match f().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Operation succeeded on retry attempt {}", attempt + 1);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    warn!(
                        "Operation failed on attempt {}/{}: {}",
                        attempt + 1,
                        self.max_attempts,
                        e
                    );

                    // Check if error is retryable
                    if !is_retryable(&e) {
                        debug!("Error is not retryable, stopping retry attempts");
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AdapterError::Unknown("All retry attempts failed".to_string())
        }))
    }
}

pub async fn with_retry<F, Fut, T>(max_attempts: usize, f: F) -> AdapterResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = AdapterResult<T>>,
{
    RetryPolicy::new(max_attempts).execute(f).await
}

pub async fn with_retry_policy<F, Fut, T>(policy: RetryPolicy, f: F) -> AdapterResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = AdapterResult<T>>,
{
    policy.execute(f).await
}

fn is_retryable(error: &AdapterError) -> bool {
    match error {
        AdapterError::ConnectionError(_) => true,
        AdapterError::RequestFailed(_) => true,
        AdapterError::Timeout(_) => true,
        AdapterError::ServiceUnavailable(_) => true,
        AdapterError::CircuitBreakerOpen => false,
        AdapterError::SerializationError(_) => false,
        AdapterError::InvalidResponse(_) => false,
        AdapterError::Unknown(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let policy = RetryPolicy::new(3);
        let attempts = Arc::new(AtomicUsize::new(0));

        let result = policy.execute(|| {
            let attempts = attempts.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok::<_, AdapterError>(42)
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let policy = RetryPolicy::new(3);
        let attempts = Arc::new(AtomicUsize::new(0));

        let result = policy.execute(|| {
            let attempts = attempts.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err(AdapterError::RequestFailed("First attempt failed".to_string()))
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let policy = RetryPolicy::new(3);
        let attempts = Arc::new(AtomicUsize::new(0));

        let result = policy.execute(|| {
            let attempts = attempts.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(AdapterError::RequestFailed("Always fails".to_string()))
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let policy = RetryPolicy::new(3);
        let attempts = Arc::new(AtomicUsize::new(0));

        let result = policy.execute(|| {
            let attempts = attempts.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>(AdapterError::CircuitBreakerOpen)
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_backoff_calculation() {
        let policy = RetryPolicy::default()
            .with_backoff(Duration::from_millis(100), Duration::from_secs(10))
            .with_multiplier(2.0)
            .with_jitter(false);

        let backoff_0 = policy.calculate_backoff(0);
        let backoff_1 = policy.calculate_backoff(1);
        let backoff_2 = policy.calculate_backoff(2);

        assert_eq!(backoff_0.as_millis(), 100);
        assert_eq!(backoff_1.as_millis(), 200);
        assert_eq!(backoff_2.as_millis(), 400);
    }

    #[tokio::test]
    async fn test_with_retry_helper() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = with_retry(3, || async {
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                Err(AdapterError::RequestFailed("Failed".to_string()))
            } else {
                Ok(42)
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
