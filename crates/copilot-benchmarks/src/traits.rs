//! BenchTarget trait and registry
//!
//! This module defines the canonical BenchTarget trait with required
//! id() and run() methods, plus the all_targets() registry function.

use async_trait::async_trait;
use crate::result::BenchmarkResult;

/// Canonical BenchTarget trait for benchmark targets
///
/// All benchmark targets must implement this trait to be included
/// in the benchmark suite. The trait provides:
/// - id(): Returns the unique identifier for this target
/// - run(): Executes the benchmark and returns results
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target
    ///
    /// The ID should be descriptive and follow the naming convention:
    /// `category::subcategory::name` (e.g., "nlp::intent::classification")
    fn id(&self) -> &str;

    /// Execute the benchmark and return results
    ///
    /// This method should:
    /// 1. Set up any required test fixtures
    /// 2. Run the benchmark with timing
    /// 3. Collect metrics and return a BenchmarkResult
    ///
    /// The implementation should handle errors gracefully and return
    /// a failure BenchmarkResult rather than panicking.
    async fn run(&self) -> BenchmarkResult;

    /// Optional: Returns a description of what this benchmark measures
    fn description(&self) -> Option<&str> {
        None
    }

    /// Optional: Returns the expected duration range in milliseconds
    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        None
    }
}

/// Box type alias for benchmark targets
pub type BoxedBenchTarget = Box<dyn BenchTarget>;

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBenchTarget {
        id: String,
    }

    #[async_trait]
    impl BenchTarget for MockBenchTarget {
        fn id(&self) -> &str {
            &self.id
        }

        async fn run(&self) -> BenchmarkResult {
            BenchmarkResult::success(&self.id, 42)
        }
    }

    #[tokio::test]
    async fn test_bench_target_trait() {
        let target = MockBenchTarget {
            id: "test::mock".to_string(),
        };

        assert_eq!(target.id(), "test::mock");

        let result = target.run().await;
        assert!(result.is_success());
        assert_eq!(result.duration_ms(), Some(42));
    }
}
