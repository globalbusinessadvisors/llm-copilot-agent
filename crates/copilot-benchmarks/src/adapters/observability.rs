//! Observability Benchmark Adapters
//!
//! Exposes telemetry and observability operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for metrics collection
pub struct MetricsCollectionBenchmark {
    id: String,
    metric_count: usize,
    iterations: usize,
}

impl MetricsCollectionBenchmark {
    pub fn new() -> Self {
        Self {
            id: "observability::metrics::collection".to_string(),
            metric_count: 100,
            iterations: 50,
        }
    }
}

impl Default for MetricsCollectionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for MetricsCollectionBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks metrics collection and aggregation")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((10, 100))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let mut total_metrics_recorded = 0;
        let mut collection_times = Vec::new();

        for _ in 0..self.iterations {
            let iter_start = Instant::now();

            // Simulate recording various metric types
            for i in 0..self.metric_count {
                simulate_counter_increment(&format!("metric_{}", i));
                simulate_gauge_set(&format!("gauge_{}", i), i as f64);
                simulate_histogram_observe(&format!("histogram_{}", i), i as f64 * 0.1);
                total_metrics_recorded += 3;
            }

            collection_times.push(iter_start.elapsed().as_micros());
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "total_metrics_recorded": total_metrics_recorded,
                "iterations": self.iterations,
                "metrics_per_iteration": self.metric_count * 3,
                "avg_iteration_us": collection_times.iter().sum::<u128>() as f64 / collection_times.len() as f64,
                "metrics_per_second": total_metrics_recorded as f64 / total_duration.as_secs_f64()
            }),
        )
    }
}

/// Benchmark for distributed tracing
pub struct TracingBenchmark {
    id: String,
    span_depth: usize,
    traces_count: usize,
}

impl TracingBenchmark {
    pub fn new() -> Self {
        Self {
            id: "observability::tracing".to_string(),
            span_depth: 5,
            traces_count: 20,
        }
    }
}

impl Default for TracingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for TracingBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks distributed tracing span creation and propagation")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((20, 200))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let mut total_spans_created = 0;
        let mut trace_times = Vec::new();

        for trace_idx in 0..self.traces_count {
            let trace_start = Instant::now();

            // Simulate creating a trace with nested spans
            let spans = simulate_trace_creation(trace_idx, self.span_depth).await;
            total_spans_created += spans;

            trace_times.push(trace_start.elapsed().as_micros());
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "traces_created": self.traces_count,
                "total_spans": total_spans_created,
                "span_depth": self.span_depth,
                "avg_trace_time_us": trace_times.iter().sum::<u128>() as f64 / trace_times.len() as f64,
                "spans_per_second": total_spans_created as f64 / total_duration.as_secs_f64()
            }),
        )
    }
}

// Simulation functions

fn simulate_counter_increment(name: &str) {
    std::hint::black_box(name);
}

fn simulate_gauge_set(name: &str, value: f64) {
    std::hint::black_box(name);
    std::hint::black_box(value);
}

fn simulate_histogram_observe(name: &str, value: f64) {
    std::hint::black_box(name);
    std::hint::black_box(value);
}

async fn simulate_trace_creation(trace_idx: usize, depth: usize) -> usize {
    tokio::task::yield_now().await;

    let mut span_count = 0;

    // Simulate nested span creation
    fn create_nested_spans(current_depth: usize, max_depth: usize, count: &mut usize) {
        if current_depth >= max_depth {
            return;
        }

        *count += 1;
        std::hint::black_box(current_depth);

        // Each level can have 1-2 child spans
        let children = if current_depth < max_depth - 1 { 2 } else { 1 };
        for _ in 0..children {
            create_nested_spans(current_depth + 1, max_depth, count);
        }
    }

    create_nested_spans(0, depth, &mut span_count);
    std::hint::black_box(trace_idx);

    span_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collection_benchmark() {
        let benchmark = MetricsCollectionBenchmark::new();
        assert_eq!(benchmark.id(), "observability::metrics::collection");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_tracing_benchmark() {
        let benchmark = TracingBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
