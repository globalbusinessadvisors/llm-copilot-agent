//! Bulkhead pattern implementation
//!
//! Limits concurrent access to a resource to prevent overload.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{debug, warn};

/// Bulkhead configuration
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    /// Name of this bulkhead (for logging)
    pub name: String,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Maximum wait time for acquiring a permit
    pub max_wait: Option<Duration>,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_concurrent: 10,
            max_wait: Some(Duration::from_secs(30)),
        }
    }
}

impl BulkheadConfig {
    /// Create a new bulkhead config with a name
    pub fn new(name: &str, max_concurrent: usize) -> Self {
        Self {
            name: name.to_string(),
            max_concurrent,
            ..Default::default()
        }
    }

    /// Set max wait time
    pub fn with_max_wait(mut self, wait: Duration) -> Self {
        self.max_wait = Some(wait);
        self
    }

    /// Disable max wait (wait indefinitely)
    pub fn without_max_wait(mut self) -> Self {
        self.max_wait = None;
        self
    }
}

/// A permit that must be held while executing a bulkhead-protected operation
pub struct BulkheadPermit {
    _permit: OwnedSemaphorePermit,
    bulkhead_name: String,
}

impl Drop for BulkheadPermit {
    fn drop(&mut self) {
        debug!(bulkhead = %self.bulkhead_name, "Bulkhead permit released");
    }
}

/// Bulkhead for limiting concurrent executions
#[derive(Clone)]
pub struct Bulkhead {
    config: BulkheadConfig,
    semaphore: Arc<Semaphore>,
}

impl Bulkhead {
    /// Create a new bulkhead with the given configuration
    pub fn new(config: BulkheadConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        Self { config, semaphore }
    }

    /// Create with a simple max concurrent limit
    pub fn with_limit(name: &str, max_concurrent: usize) -> Self {
        Self::new(BulkheadConfig::new(name, max_concurrent))
    }

    /// Try to acquire a permit immediately
    pub fn try_acquire(&self) -> Option<BulkheadPermit> {
        match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                debug!(bulkhead = %self.config.name, "Bulkhead permit acquired");
                Some(BulkheadPermit {
                    _permit: permit,
                    bulkhead_name: self.config.name.clone(),
                })
            }
            Err(_) => {
                warn!(
                    bulkhead = %self.config.name,
                    max_concurrent = self.config.max_concurrent,
                    "Bulkhead rejected request (no permits available)"
                );
                None
            }
        }
    }

    /// Acquire a permit, waiting up to max_wait if configured
    pub async fn acquire(&self) -> Option<BulkheadPermit> {
        match self.config.max_wait {
            Some(max_wait) => {
                match tokio::time::timeout(
                    max_wait,
                    self.semaphore.clone().acquire_owned(),
                )
                .await
                {
                    Ok(Ok(permit)) => {
                        debug!(bulkhead = %self.config.name, "Bulkhead permit acquired");
                        Some(BulkheadPermit {
                            _permit: permit,
                            bulkhead_name: self.config.name.clone(),
                        })
                    }
                    Ok(Err(_)) => {
                        warn!(
                            bulkhead = %self.config.name,
                            "Bulkhead semaphore closed"
                        );
                        None
                    }
                    Err(_) => {
                        warn!(
                            bulkhead = %self.config.name,
                            max_wait_ms = max_wait.as_millis(),
                            "Bulkhead timed out waiting for permit"
                        );
                        None
                    }
                }
            }
            None => {
                // Wait indefinitely
                match self.semaphore.clone().acquire_owned().await {
                    Ok(permit) => {
                        debug!(bulkhead = %self.config.name, "Bulkhead permit acquired");
                        Some(BulkheadPermit {
                            _permit: permit,
                            bulkhead_name: self.config.name.clone(),
                        })
                    }
                    Err(_) => {
                        warn!(
                            bulkhead = %self.config.name,
                            "Bulkhead semaphore closed"
                        );
                        None
                    }
                }
            }
        }
    }

    /// Get the number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Get the maximum concurrent limit
    pub fn max_concurrent(&self) -> usize {
        self.config.max_concurrent
    }

    /// Get the bulkhead name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Execute an operation with bulkhead protection
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, BulkheadError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let _permit = self.acquire().await.ok_or(BulkheadError::Rejected)?;
        operation().await.map_err(BulkheadError::Operation)
    }

    /// Try to execute an operation immediately (non-blocking)
    pub async fn try_execute<F, Fut, T, E>(&self, operation: F) -> Result<T, BulkheadError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let _permit = self.try_acquire().ok_or(BulkheadError::Rejected)?;
        operation().await.map_err(BulkheadError::Operation)
    }
}

/// Errors that can occur when using a bulkhead
#[derive(Debug)]
pub enum BulkheadError<E> {
    /// Request was rejected (no permits available)
    Rejected,
    /// The underlying operation failed
    Operation(E),
}

impl<E: std::fmt::Display> std::fmt::Display for BulkheadError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BulkheadError::Rejected => write!(f, "Bulkhead rejected request"),
            BulkheadError::Operation(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for BulkheadError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BulkheadError::Rejected => None,
            BulkheadError::Operation(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_bulkhead_limits_concurrency() {
        let bulkhead = Bulkhead::with_limit("test", 2);

        // Should be able to acquire 2 permits
        let permit1 = bulkhead.try_acquire();
        assert!(permit1.is_some());

        let permit2 = bulkhead.try_acquire();
        assert!(permit2.is_some());

        // Third should fail
        let permit3 = bulkhead.try_acquire();
        assert!(permit3.is_none());

        // Release one
        drop(permit1);

        // Now should be able to acquire
        let permit4 = bulkhead.try_acquire();
        assert!(permit4.is_some());
    }

    #[tokio::test]
    async fn test_bulkhead_available_permits() {
        let bulkhead = Bulkhead::with_limit("test", 5);

        assert_eq!(bulkhead.available_permits(), 5);

        let permit = bulkhead.try_acquire();
        assert!(permit.is_some());
        assert_eq!(bulkhead.available_permits(), 4);

        drop(permit);
        assert_eq!(bulkhead.available_permits(), 5);
    }

    #[tokio::test]
    async fn test_bulkhead_execute() {
        let bulkhead = Bulkhead::with_limit("test", 10);

        let result: Result<i32, BulkheadError<&str>> = bulkhead
            .execute(|| async { Ok(42) })
            .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_bulkhead_execute_error() {
        let bulkhead = Bulkhead::with_limit("test", 10);

        let result: Result<i32, BulkheadError<&str>> = bulkhead
            .execute(|| async { Err("operation failed") })
            .await;

        match result {
            Err(BulkheadError::Operation(e)) => assert_eq!(e, "operation failed"),
            _ => panic!("Expected Operation error"),
        }
    }

    #[tokio::test]
    async fn test_bulkhead_wait_timeout() {
        let config = BulkheadConfig::new("test", 1)
            .with_max_wait(Duration::from_millis(50));

        let bulkhead = Bulkhead::new(config);

        // Acquire the only permit
        let _permit = bulkhead.acquire().await;

        // This should timeout
        let result = bulkhead.acquire().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_bulkhead_concurrent_execution() {
        let bulkhead = Arc::new(Bulkhead::with_limit("test", 3));
        let counter = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let bulkhead = Arc::clone(&bulkhead);
            let counter = Arc::clone(&counter);
            let max_concurrent = Arc::clone(&max_concurrent);

            handles.push(tokio::spawn(async move {
                if let Some(_permit) = bulkhead.acquire().await {
                    let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    max_concurrent.fetch_max(current, Ordering::SeqCst);

                    tokio::time::sleep(Duration::from_millis(10)).await;

                    counter.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Max concurrent should not exceed bulkhead limit
        assert!(max_concurrent.load(Ordering::SeqCst) <= 3);
    }
}
