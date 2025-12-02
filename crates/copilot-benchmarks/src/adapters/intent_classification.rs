//! Intent Classification Benchmark Adapters
//!
//! Exposes NLP intent classification operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for simple intent classification
pub struct SimpleIntentBenchmark {
    id: String,
    iterations: usize,
}

impl SimpleIntentBenchmark {
    pub fn new() -> Self {
        Self {
            id: "nlp::intent::simple".to_string(),
            iterations: 100,
        }
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }
}

impl Default for SimpleIntentBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for SimpleIntentBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks simple intent classification for short queries")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((10, 100))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        // Simulate intent classification for simple queries
        let queries = vec![
            "Show me CPU usage",
            "What is the memory consumption?",
            "List all services",
            "Find errors in logs",
            "Show network traffic",
        ];

        let mut total_classifications = 0;

        for _ in 0..self.iterations {
            for query in &queries {
                // Simulate classification work
                let _ = simulate_intent_classification(query);
                total_classifications += 1;
            }
        }

        let duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": duration.as_millis() as u64,
                "iterations": self.iterations,
                "total_classifications": total_classifications,
                "avg_classification_us": duration.as_micros() as f64 / total_classifications as f64,
                "queries_per_second": (total_classifications as f64 / duration.as_secs_f64()).round()
            }),
        )
    }
}

/// Benchmark for complex intent classification
pub struct ComplexIntentBenchmark {
    id: String,
    iterations: usize,
}

impl ComplexIntentBenchmark {
    pub fn new() -> Self {
        Self {
            id: "nlp::intent::complex".to_string(),
            iterations: 50,
        }
    }
}

impl Default for ComplexIntentBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for ComplexIntentBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks complex intent classification for multi-part queries")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((50, 500))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let complex_queries = vec![
            "Compare the CPU and memory usage between auth-service and api-gateway in us-east-1 and eu-west-1 for the last 24 hours and show me any anomalies",
            "Find all error logs from the payment service where the response time exceeded 5 seconds and correlate them with database connection pool exhaustion events",
            "Generate a report showing the trend of API latency for all microservices over the past week, grouped by region and deployment version",
        ];

        let mut total = 0;
        for _ in 0..self.iterations {
            for query in &complex_queries {
                let _ = simulate_complex_classification(query);
                total += 1;
            }
        }

        let duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": duration.as_millis() as u64,
                "iterations": self.iterations,
                "total_classifications": total,
                "avg_classification_ms": duration.as_millis() as f64 / total as f64,
                "complexity_factor": 3.0
            }),
        )
    }
}

/// Benchmark for batch intent classification
pub struct BatchIntentBenchmark {
    id: String,
    batch_sizes: Vec<usize>,
}

impl BatchIntentBenchmark {
    pub fn new() -> Self {
        Self {
            id: "nlp::intent::batch".to_string(),
            batch_sizes: vec![10, 50, 100],
        }
    }
}

impl Default for BatchIntentBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for BatchIntentBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks batch intent classification with varying batch sizes")
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let query = "Show me metrics";
        let mut batch_results = Vec::new();

        for &batch_size in &self.batch_sizes {
            let batch_start = Instant::now();
            for _ in 0..batch_size {
                let _ = simulate_intent_classification(query);
            }
            let batch_duration = batch_start.elapsed();

            batch_results.push(serde_json::json!({
                "batch_size": batch_size,
                "duration_ms": batch_duration.as_millis(),
                "throughput": batch_size as f64 / batch_duration.as_secs_f64()
            }));
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "batch_results": batch_results,
                "batch_sizes_tested": self.batch_sizes
            }),
        )
    }
}

// Simulation functions (replace with actual implementation calls when available)

fn simulate_intent_classification(query: &str) -> &'static str {
    // Simulate processing time
    std::hint::black_box(query.len());

    // Return a mock intent type
    if query.contains("CPU") || query.contains("memory") || query.contains("usage") {
        "metrics_query"
    } else if query.contains("error") || query.contains("log") {
        "log_query"
    } else if query.contains("service") || query.contains("list") {
        "service_query"
    } else {
        "general_query"
    }
}

fn simulate_complex_classification(query: &str) -> Vec<&'static str> {
    // Simulate more complex processing
    std::hint::black_box(query.len() * 10);

    let mut intents = Vec::new();
    if query.contains("compare") || query.contains("between") {
        intents.push("comparison");
    }
    if query.contains("time") || query.contains("hours") || query.contains("week") {
        intents.push("time_range");
    }
    if query.contains("anomal") || query.contains("error") {
        intents.push("anomaly_detection");
    }
    if query.contains("report") || query.contains("trend") {
        intents.push("reporting");
    }

    if intents.is_empty() {
        intents.push("general_query");
    }

    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_intent_benchmark() {
        let benchmark = SimpleIntentBenchmark::new();
        assert_eq!(benchmark.id(), "nlp::intent::simple");

        let result = benchmark.run().await;
        assert!(result.is_success());
        assert!(result.duration_ms().is_some());
    }

    #[tokio::test]
    async fn test_complex_intent_benchmark() {
        let benchmark = ComplexIntentBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_batch_intent_benchmark() {
        let benchmark = BatchIntentBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
