//! Timeout handling
//!
//! Provides configurable timeouts for operations.

use std::future::Future;
use std::time::Duration;
use tracing::warn;

/// Timeout error
#[derive(Debug, Clone)]
pub struct TimeoutError {
    /// Duration that was exceeded
    pub duration: Duration,
    /// Operation name
    pub operation: String,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Operation '{}' timed out after {:?}",
            self.operation, self.duration
        )
    }
}

impl std::error::Error for TimeoutError {}

/// Timeout policy configuration
#[derive(Debug, Clone)]
pub struct TimeoutPolicy {
    /// Default timeout duration
    pub default_timeout: Duration,
    /// Timeout for read operations
    pub read_timeout: Duration,
    /// Timeout for write operations
    pub write_timeout: Duration,
    /// Timeout for connection operations
    pub connect_timeout: Duration,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

impl TimeoutPolicy {
    /// Create a new timeout policy
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            default_timeout,
            read_timeout: default_timeout,
            write_timeout: default_timeout,
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Create a strict policy with shorter timeouts
    pub fn strict() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(3),
        }
    }

    /// Create a relaxed policy with longer timeouts
    pub fn relaxed() -> Self {
        Self {
            default_timeout: Duration::from_secs(60),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(30),
        }
    }

    /// Set default timeout
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set read timeout
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set write timeout
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set connect timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Execute an operation with the default timeout
    pub async fn execute<F, Fut, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, TimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        self.execute_with_timeout(operation_name, self.default_timeout, operation)
            .await
    }

    /// Execute a read operation
    pub async fn execute_read<F, Fut, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, TimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        self.execute_with_timeout(operation_name, self.read_timeout, operation)
            .await
    }

    /// Execute a write operation
    pub async fn execute_write<F, Fut, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, TimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        self.execute_with_timeout(operation_name, self.write_timeout, operation)
            .await
    }

    /// Execute an operation with a specific timeout
    pub async fn execute_with_timeout<F, Fut, T>(
        &self,
        operation_name: &str,
        timeout: Duration,
        operation: F,
    ) -> Result<T, TimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        match tokio::time::timeout(timeout, operation()).await {
            Ok(result) => Ok(result),
            Err(_) => {
                warn!(
                    operation = operation_name,
                    timeout_ms = timeout.as_millis(),
                    "Operation timed out"
                );
                Err(TimeoutError {
                    duration: timeout,
                    operation: operation_name.to_string(),
                })
            }
        }
    }
}

/// Execute an operation with a timeout
pub async fn with_timeout<F, Fut, T>(
    timeout: Duration,
    operation_name: &str,
    operation: F,
) -> Result<T, TimeoutError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    match tokio::time::timeout(timeout, operation()).await {
        Ok(result) => Ok(result),
        Err(_) => Err(TimeoutError {
            duration: timeout,
            operation: operation_name.to_string(),
        }),
    }
}

/// Execute a future with a timeout, returning the result or the error
pub async fn timeout_result<F, Fut, T, E>(
    timeout: Duration,
    operation_name: &str,
    operation: F,
) -> Result<T, TimeoutOrError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    match tokio::time::timeout(timeout, operation()).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(TimeoutOrError::Error(e)),
        Err(_) => Err(TimeoutOrError::Timeout(TimeoutError {
            duration: timeout,
            operation: operation_name.to_string(),
        })),
    }
}

/// Either a timeout or an operation error
#[derive(Debug)]
pub enum TimeoutOrError<E> {
    /// The operation timed out
    Timeout(TimeoutError),
    /// The operation failed with an error
    Error(E),
}

impl<E: std::fmt::Display> std::fmt::Display for TimeoutOrError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutOrError::Timeout(e) => write!(f, "{}", e),
            TimeoutOrError::Error(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for TimeoutOrError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TimeoutOrError::Timeout(e) => Some(e),
            TimeoutOrError::Error(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_success() {
        let policy = TimeoutPolicy::default();

        let result = policy
            .execute("test", || async { 42 })
            .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_timeout_exceeded() {
        let policy = TimeoutPolicy::new(Duration::from_millis(10));

        let result = policy
            .execute("slow_operation", || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                42
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.operation, "slow_operation");
        assert_eq!(err.duration, Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_with_timeout_function() {
        let result = with_timeout(
            Duration::from_millis(10),
            "test",
            || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_timeout_result() {
        // Success case
        let result: Result<i32, TimeoutOrError<&str>> = timeout_result(
            Duration::from_secs(1),
            "test",
            || async { Ok(42) },
        )
        .await;

        assert_eq!(result.unwrap(), 42);

        // Error case
        let result: Result<i32, TimeoutOrError<&str>> = timeout_result(
            Duration::from_secs(1),
            "test",
            || async { Err("operation failed") },
        )
        .await;

        match result {
            Err(TimeoutOrError::Error(e)) => assert_eq!(e, "operation failed"),
            _ => panic!("Expected Error"),
        }

        // Timeout case
        let result: Result<i32, TimeoutOrError<&str>> = timeout_result(
            Duration::from_millis(10),
            "slow_op",
            || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(42)
            },
        )
        .await;

        match result {
            Err(TimeoutOrError::Timeout(e)) => assert_eq!(e.operation, "slow_op"),
            _ => panic!("Expected Timeout"),
        }
    }

    #[test]
    fn test_policy_configurations() {
        let strict = TimeoutPolicy::strict();
        assert_eq!(strict.default_timeout, Duration::from_secs(5));

        let relaxed = TimeoutPolicy::relaxed();
        assert_eq!(relaxed.default_timeout, Duration::from_secs(60));
    }
}
