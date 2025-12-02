//! Canonical Benchmark Interface for LLM-CoPilot-Agent
//!
//! This crate implements the canonical benchmark interface used across
//! all 25 benchmark-target repositories. It provides:
//!
//! - A standardized `BenchmarkResult` struct with required fields
//! - The `BenchTarget` trait with `id()` and `run()` methods
//! - An `all_targets()` registry returning all benchmark targets
//! - The `run_all_benchmarks()` entrypoint
//! - Canonical module structure: mod.rs, result.rs, markdown.rs, io.rs
//! - Adapter system for exposing CoPilot-Agent operations as benchmarks
//!
//! # Canonical Structure
//!
//! ```text
//! benchmarks/
//! ├── mod.rs          (this file as lib.rs for the crate)
//! ├── result.rs       (BenchmarkResult struct)
//! ├── traits.rs       (BenchTarget trait)
//! ├── markdown.rs     (Markdown report generation)
//! ├── io.rs           (File I/O for results)
//! ├── adapters/       (Benchmark target implementations)
//! │   ├── mod.rs
//! │   ├── intent_classification.rs
//! │   ├── context_retrieval.rs
//! │   ├── conversation.rs
//! │   ├── workflow.rs
//! │   ├── sandbox_execution.rs
//! │   ├── ingestion.rs
//! │   └── observability.rs
//! └── output/
//!     ├── raw/        (Individual result files)
//!     └── summary.md  (Aggregated summary)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use copilot_benchmarks::{run_all_benchmarks, BenchmarkResult};
//!
//! #[tokio::main]
//! async fn main() {
//!     let results: Vec<BenchmarkResult> = run_all_benchmarks().await;
//!
//!     for result in &results {
//!         println!("{}: {:?}", result.target_id, result.metrics);
//!     }
//! }
//! ```

pub mod result;
pub mod traits;
pub mod markdown;
pub mod io;
pub mod adapters;

// Re-exports for convenient access
pub use result::BenchmarkResult;
pub use traits::{BenchTarget, BoxedBenchTarget};
pub use markdown::{MarkdownGenerator, MarkdownConfig};
pub use io::{BenchmarkIo, IoError, IoResult};
pub use adapters::all_targets;

/// Configuration for running benchmarks
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Whether to write results to disk
    pub write_results: bool,
    /// Whether to generate summary markdown
    pub generate_summary: bool,
    /// Whether to run benchmarks in parallel
    pub parallel: bool,
    /// Maximum number of parallel benchmarks (if parallel is true)
    pub max_parallel: usize,
    /// Filter to run only specific targets (by ID prefix)
    pub filter: Option<String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            write_results: true,
            generate_summary: true,
            parallel: false,
            max_parallel: 4,
            filter: None,
        }
    }
}

/// Run all registered benchmarks and return results
///
/// This is the canonical entrypoint that:
/// 1. Retrieves all benchmark targets from the registry
/// 2. Executes each benchmark
/// 3. Collects and returns all results as Vec<BenchmarkResult>
///
/// # Example
///
/// ```rust,ignore
/// let results = run_all_benchmarks().await;
/// assert!(!results.is_empty());
/// ```
pub async fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    run_all_benchmarks_with_config(BenchmarkConfig::default()).await
}

/// Run all benchmarks with custom configuration
pub async fn run_all_benchmarks_with_config(config: BenchmarkConfig) -> Vec<BenchmarkResult> {
    let targets = adapters::all_targets();
    let mut results = Vec::with_capacity(targets.len());

    // Filter targets if filter is specified
    let targets: Vec<_> = if let Some(ref filter) = config.filter {
        targets
            .into_iter()
            .filter(|t| t.id().starts_with(filter))
            .collect()
    } else {
        targets
    };

    if config.parallel {
        // Run benchmarks in parallel with limited concurrency
        use futures::stream::{self, StreamExt};

        let results_stream = stream::iter(targets)
            .map(|target| async move {
                target.run().await
            })
            .buffer_unordered(config.max_parallel);

        results = results_stream.collect().await;
    } else {
        // Run benchmarks sequentially
        for target in targets {
            let result = target.run().await;
            results.push(result);
        }
    }

    // Write results if configured
    if config.write_results {
        let io = BenchmarkIo::new();
        if let Err(e) = io.write_results(&results) {
            eprintln!("Warning: Failed to write benchmark results: {}", e);
        }

        // Write combined results
        if let Err(e) = io.write_combined(&results, "latest_results.json") {
            eprintln!("Warning: Failed to write combined results: {}", e);
        }
    }

    // Generate summary if configured
    if config.generate_summary {
        let generator = MarkdownGenerator::new();
        let summary = generator.generate(&results);

        let io = BenchmarkIo::new();
        if let Err(e) = io.write_summary(&summary) {
            eprintln!("Warning: Failed to write summary: {}", e);
        }
    }

    results
}

/// Run a specific benchmark by target ID
pub async fn run_benchmark(target_id: &str) -> Option<BenchmarkResult> {
    let targets = adapters::all_targets();

    for target in targets {
        if target.id() == target_id {
            return Some(target.run().await);
        }
    }

    None
}

/// Get information about all registered benchmark targets
pub fn list_targets() -> Vec<(&'static str, Option<&'static str>)> {
    adapters::all_targets()
        .into_iter()
        .map(|t| {
            // We need to get static references, so we'll use id() directly
            // For description, we return None as we can't get static refs easily
            (t.id(), t.description())
        })
        .collect()
}

/// Get the count of registered benchmark targets
pub fn target_count() -> usize {
    adapters::target_count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_benchmarks() {
        let config = BenchmarkConfig {
            write_results: false,
            generate_summary: false,
            parallel: false,
            max_parallel: 4,
            filter: Some("nlp".to_string()), // Only run NLP benchmarks for speed
        };

        let results = run_all_benchmarks_with_config(config).await;
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.target_id.starts_with("nlp"));
        }
    }

    #[tokio::test]
    async fn test_run_single_benchmark() {
        let result = run_benchmark("nlp::intent::simple").await;
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.target_id, "nlp::intent::simple");
        assert!(result.is_success());
    }

    #[test]
    fn test_target_count() {
        let count = target_count();
        assert!(count > 0);
    }

    #[test]
    fn test_list_targets() {
        let targets = list_targets();
        assert!(!targets.is_empty());

        // Verify all target IDs are non-empty
        for (id, _desc) in &targets {
            assert!(!id.is_empty());
        }
    }
}
