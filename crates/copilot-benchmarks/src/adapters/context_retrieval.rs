//! Context Retrieval Benchmark Adapters
//!
//! Exposes context retrieval operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for simple context retrieval
pub struct SimpleRetrievalBenchmark {
    id: String,
    k_values: Vec<usize>,
}

impl SimpleRetrievalBenchmark {
    pub fn new() -> Self {
        Self {
            id: "context::retrieval::simple".to_string(),
            k_values: vec![5, 10, 20],
        }
    }
}

impl Default for SimpleRetrievalBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for SimpleRetrievalBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks context retrieval with varying k values")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((20, 200))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let query = "Show me the authentication service configuration";
        let mut retrieval_results = Vec::new();

        for &k in &self.k_values {
            let retrieval_start = Instant::now();

            // Simulate context retrieval
            let contexts = simulate_context_retrieval(query, k).await;

            let retrieval_duration = retrieval_start.elapsed();

            retrieval_results.push(serde_json::json!({
                "k": k,
                "results_count": contexts.len(),
                "duration_ms": retrieval_duration.as_millis(),
                "avg_relevance_score": 0.85
            }));
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "k_value_results": retrieval_results,
                "query": query
            }),
        )
    }
}

/// Benchmark for large corpus context retrieval
pub struct LargeCorpusRetrievalBenchmark {
    id: String,
    corpus_size: usize,
}

impl LargeCorpusRetrievalBenchmark {
    pub fn new() -> Self {
        Self {
            id: "context::retrieval::large_corpus".to_string(),
            corpus_size: 10000,
        }
    }

    pub fn with_corpus_size(mut self, size: usize) -> Self {
        self.corpus_size = size;
        self
    }
}

impl Default for LargeCorpusRetrievalBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for LargeCorpusRetrievalBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks context retrieval against a large document corpus")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((100, 2000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        // Simulate building a large corpus
        let corpus_build_start = Instant::now();
        let _corpus = simulate_corpus_build(self.corpus_size);
        let corpus_build_time = corpus_build_start.elapsed();

        // Run retrieval queries
        let queries = vec![
            "authentication configuration",
            "database connection settings",
            "API rate limiting rules",
            "error handling patterns",
            "logging configuration",
        ];

        let mut query_times = Vec::new();
        for query in &queries {
            let query_start = Instant::now();
            let _ = simulate_context_retrieval(query, 10).await;
            query_times.push(query_start.elapsed().as_micros());
        }

        let total_duration = start.elapsed();
        let avg_query_time = query_times.iter().sum::<u128>() as f64 / query_times.len() as f64;

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "corpus_size": self.corpus_size,
                "corpus_build_ms": corpus_build_time.as_millis(),
                "queries_executed": queries.len(),
                "avg_query_time_us": avg_query_time,
                "min_query_time_us": query_times.iter().min().unwrap_or(&0),
                "max_query_time_us": query_times.iter().max().unwrap_or(&0)
            }),
        )
    }
}

// Simulation functions

async fn simulate_context_retrieval(query: &str, k: usize) -> Vec<String> {
    // Simulate async retrieval work
    tokio::task::yield_now().await;

    std::hint::black_box(query.len());

    (0..k)
        .map(|i| format!("Context chunk {} for query: {}", i, &query[..query.len().min(20)]))
        .collect()
}

fn simulate_corpus_build(size: usize) -> Vec<String> {
    (0..size)
        .map(|i| format!("Document {} with content about various topics including configuration, settings, and patterns.", i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_retrieval_benchmark() {
        let benchmark = SimpleRetrievalBenchmark::new();
        assert_eq!(benchmark.id(), "context::retrieval::simple");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_large_corpus_benchmark() {
        let benchmark = LargeCorpusRetrievalBenchmark::new().with_corpus_size(100);
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
