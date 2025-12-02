//! Canonical BenchmarkResult struct
//!
//! This module defines the standardized BenchmarkResult struct used across
//! all benchmark targets in the LLM-CoPilot-Agent repository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical BenchmarkResult struct with exactly the required fields:
/// - target_id: String - Identifier for the benchmark target
/// - metrics: serde_json::Value - Flexible JSON metrics payload
/// - timestamp: chrono::DateTime<chrono::Utc> - When the benchmark was run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target
    pub target_id: String,

    /// Flexible JSON payload containing benchmark metrics
    ///
    /// Common fields include:
    /// - duration_ms: Execution time in milliseconds
    /// - iterations: Number of iterations run
    /// - throughput: Operations per second
    /// - memory_bytes: Memory usage in bytes
    /// - success: Boolean indicating success/failure
    /// - error: Optional error message
    pub metrics: serde_json::Value,

    /// UTC timestamp when the benchmark was executed
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Create a new BenchmarkResult with the current timestamp
    pub fn new(target_id: impl Into<String>, metrics: serde_json::Value) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp: Utc::now(),
        }
    }

    /// Create a new BenchmarkResult with a specific timestamp
    pub fn with_timestamp(
        target_id: impl Into<String>,
        metrics: serde_json::Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp,
        }
    }

    /// Create a success result with duration
    pub fn success(target_id: impl Into<String>, duration_ms: u64) -> Self {
        Self::new(
            target_id,
            serde_json::json!({
                "success": true,
                "duration_ms": duration_ms
            }),
        )
    }

    /// Create a failure result with error message
    pub fn failure(target_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::new(
            target_id,
            serde_json::json!({
                "success": false,
                "error": error.into()
            }),
        )
    }

    /// Check if the benchmark was successful
    pub fn is_success(&self) -> bool {
        self.metrics
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    /// Get the duration in milliseconds if available
    pub fn duration_ms(&self) -> Option<u64> {
        self.metrics.get("duration_ms").and_then(|v| v.as_u64())
    }

    /// Get the error message if available
    pub fn error(&self) -> Option<&str> {
        self.metrics.get("error").and_then(|v| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_new() {
        let result = BenchmarkResult::new(
            "test_target",
            serde_json::json!({
                "duration_ms": 100,
                "iterations": 1000
            }),
        );

        assert_eq!(result.target_id, "test_target");
        assert_eq!(result.metrics["duration_ms"], 100);
        assert_eq!(result.metrics["iterations"], 1000);
    }

    #[test]
    fn test_benchmark_result_success() {
        let result = BenchmarkResult::success("test_target", 150);
        assert!(result.is_success());
        assert_eq!(result.duration_ms(), Some(150));
    }

    #[test]
    fn test_benchmark_result_failure() {
        let result = BenchmarkResult::failure("test_target", "Something went wrong");
        assert!(!result.is_success());
        assert_eq!(result.error(), Some("Something went wrong"));
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let result = BenchmarkResult::success("test_target", 100);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.target_id, result.target_id);
        assert_eq!(deserialized.duration_ms(), result.duration_ms());
    }
}
