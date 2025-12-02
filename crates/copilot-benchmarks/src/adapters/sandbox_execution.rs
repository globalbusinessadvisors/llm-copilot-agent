//! Sandbox Execution Benchmark Adapters
//!
//! Exposes E2B sandbox execution operations as benchmark targets.

use async_trait::async_trait;
use std::time::Instant;
use crate::result::BenchmarkResult;
use crate::traits::BenchTarget;

/// Benchmark for Python code execution in sandbox
pub struct PythonExecutionBenchmark {
    id: String,
    iterations: usize,
}

impl PythonExecutionBenchmark {
    pub fn new() -> Self {
        Self {
            id: "sandbox::python::execution".to_string(),
            iterations: 10,
        }
    }
}

impl Default for PythonExecutionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for PythonExecutionBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks Python code execution in E2B sandbox")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((100, 1000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let python_snippets = vec![
            "print('Hello, World!')",
            "import json; data = {'key': 'value'}; print(json.dumps(data))",
            "result = sum(range(1000)); print(result)",
            "import math; print([math.sqrt(i) for i in range(10)])",
        ];

        let mut execution_times = Vec::new();
        let mut total_stdout_bytes = 0;

        for _ in 0..self.iterations {
            for code in &python_snippets {
                let exec_start = Instant::now();
                let (stdout, _stderr, exit_code) = simulate_sandbox_execution(code, "python").await;
                execution_times.push(exec_start.elapsed().as_millis());

                total_stdout_bytes += stdout.len();
                std::hint::black_box(exit_code);
            }
        }

        let total_duration = start.elapsed();
        let total_executions = self.iterations * python_snippets.len();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "total_executions": total_executions,
                "avg_execution_ms": execution_times.iter().sum::<u128>() as f64 / execution_times.len() as f64,
                "total_stdout_bytes": total_stdout_bytes,
                "runtime": "python"
            }),
        )
    }
}

/// Benchmark for Node.js code execution in sandbox
pub struct NodeExecutionBenchmark {
    id: String,
    iterations: usize,
}

impl NodeExecutionBenchmark {
    pub fn new() -> Self {
        Self {
            id: "sandbox::nodejs::execution".to_string(),
            iterations: 10,
        }
    }
}

impl Default for NodeExecutionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for NodeExecutionBenchmark {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some("Benchmarks Node.js code execution in E2B sandbox")
    }

    fn expected_duration_ms(&self) -> Option<(u64, u64)> {
        Some((100, 1000))
    }

    async fn run(&self) -> BenchmarkResult {
        let start = Instant::now();

        let node_snippets = vec![
            "console.log('Hello, World!')",
            "const data = {key: 'value'}; console.log(JSON.stringify(data))",
            "const sum = Array.from({length: 1000}, (_, i) => i).reduce((a, b) => a + b); console.log(sum)",
            "console.log([...Array(10).keys()].map(Math.sqrt))",
        ];

        let mut execution_times = Vec::new();
        let mut total_stdout_bytes = 0;

        for _ in 0..self.iterations {
            for code in &node_snippets {
                let exec_start = Instant::now();
                let (stdout, _stderr, exit_code) = simulate_sandbox_execution(code, "nodejs").await;
                execution_times.push(exec_start.elapsed().as_millis());

                total_stdout_bytes += stdout.len();
                std::hint::black_box(exit_code);
            }
        }

        let total_duration = start.elapsed();
        let total_executions = self.iterations * node_snippets.len();

        BenchmarkResult::new(
            &self.id,
            serde_json::json!({
                "success": true,
                "duration_ms": total_duration.as_millis() as u64,
                "total_executions": total_executions,
                "avg_execution_ms": execution_times.iter().sum::<u128>() as f64 / execution_times.len() as f64,
                "total_stdout_bytes": total_stdout_bytes,
                "runtime": "nodejs"
            }),
        )
    }
}

// Simulation function

async fn simulate_sandbox_execution(code: &str, runtime: &str) -> (String, String, i32) {
    // Simulate async sandbox execution
    tokio::task::yield_now().await;

    std::hint::black_box(code.len());
    std::hint::black_box(runtime);

    // Return mock output
    let stdout = format!("Output from {} execution of {} bytes of code", runtime, code.len());
    let stderr = String::new();
    let exit_code = 0;

    (stdout, stderr, exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_execution_benchmark() {
        let benchmark = PythonExecutionBenchmark::new();
        assert_eq!(benchmark.id(), "sandbox::python::execution");

        let result = benchmark.run().await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_node_execution_benchmark() {
        let benchmark = NodeExecutionBenchmark::new();
        let result = benchmark.run().await;
        assert!(result.is_success());
    }
}
