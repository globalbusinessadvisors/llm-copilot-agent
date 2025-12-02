//! Workflow Benchmark Adapters
//!
//! Exposes workflow orchestration operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for workflow execution
pub struct WorkflowExecutionBenchmark {
    id: String,
    workflow_complexity: WorkflowComplexity,
}

#[derive(Clone, Copy)]
pub enum WorkflowComplexity {
    Simple,    // 3-5 steps
    Medium,    // 10-15 steps
    Complex,   // 20+ steps with branches
}

impl WorkflowExecutionBenchmark {
    pub fn new() -> Self {
        Self {
            id: "workflow::execution".to_string(),
            workflow_complexity: WorkflowComplexity::Medium,
        }
    }

    pub fn with_complexity(mut self, complexity: WorkflowComplexity) -> Self {
        self.workflow_complexity = complexity;
        self
    }
}

impl Default for WorkflowExecutionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for WorkflowExecutionBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks DAG-based workflow execution with varying complexity")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((100, 1000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let step_count = match self.workflow_complexity {
            WorkflowComplexity::Simple => 5,
            WorkflowComplexity::Medium => 12,
            WorkflowComplexity::Complex => 25,
        };

        // Simulate workflow execution
        let mut step_times = Vec::new();
        let mut parallel_groups = 0;

        for step in 0..step_count {
            let step_start = Instant::now();

            // Simulate step execution
            let is_parallel = step % 3 == 0 && step > 0;
            if is_parallel {
                parallel_groups += 1;
                simulate_parallel_step_execution(3).await;
            } else {
                simulate_step_execution(step).await;
            }

            step_times.push(step_start.elapsed().as_micros());
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "step_count": step_count,
                "parallel_groups": parallel_groups,
                "avg_step_time_us": step_times.iter().sum::<u128>() as f64 / step_times.len() as f64,
                "complexity": match self.workflow_complexity {
                    WorkflowComplexity::Simple => "simple",
                    WorkflowComplexity::Medium => "medium",
                    WorkflowComplexity::Complex => "complex",
                }
            }),
        )
    }
}

/// Benchmark for workflow validation
pub struct WorkflowValidationBenchmark {
    id: String,
}

impl WorkflowValidationBenchmark {
    pub fn new() -> Self {
        Self {
            id: "workflow::validation".to_string(),
        }
    }
}

impl Default for WorkflowValidationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for WorkflowValidationBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks workflow definition validation including cycle detection")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((5, 50))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        // Simulate validating various workflow definitions
        let workflow_sizes = vec![5, 10, 20, 50, 100];
        let mut validation_results = Vec::new();

        for size in &workflow_sizes {
            let validation_start = Instant::now();

            // Simulate workflow validation
            let _is_valid = simulate_workflow_validation(*size);
            let _has_cycles = simulate_cycle_detection(*size);

            validation_results.push(serde_json::json!({
                "step_count": size,
                "validation_time_us": validation_start.elapsed().as_micros()
            }));
        }

        let total_duration = start.elapsed();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "workflows_validated": workflow_sizes.len(),
                "validation_results": validation_results
            }),
        )
    }
}

// Simulation functions

async fn simulate_step_execution(step: usize) {
    tokio::task::yield_now().await;
    std::hint::black_box(step);
}

async fn simulate_parallel_step_execution(parallel_count: usize) {
    let handles: Vec<_> = (0..parallel_count)
        .map(|i| {
            tokio::spawn(async move {
                tokio::task::yield_now().await;
                std::hint::black_box(i);
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await;
    }
}

fn simulate_workflow_validation(step_count: usize) -> bool {
    std::hint::black_box(step_count);
    true
}

fn simulate_cycle_detection(step_count: usize) -> bool {
    std::hint::black_box(step_count);
    false // No cycles detected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution_benchmark() {
        let benchmark = WorkflowExecutionBenchmark::new();
        assert_eq!(benchmark.id(), "workflow::execution");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_workflow_validation_benchmark() {
        let benchmark = WorkflowValidationBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
