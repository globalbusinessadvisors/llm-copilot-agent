//! LLM-Test-Bench Runtime Adapter
//!
//! Runtime integration for LLM-Test-Bench service.
//! Provides benchmark querying and triggering capabilities.
//! This is NOT a compile-time dependency.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::{
    AdapterError, AdapterResult, HealthStatus, ModuleCapabilities, ModuleAdapter,
    circuit_breaker::CircuitBreaker,
    retry::with_retry,
};
use super::BenchmarkResult;

/// Benchmark suite definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub benchmarks: Vec<BenchmarkDefinition>,
    pub default_config: BenchmarkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkDefinition {
    pub id: String,
    pub name: String,
    pub benchmark_type: BenchmarkType,
    pub metrics: Vec<String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkType {
    Latency,
    Throughput,
    Quality,
    Cost,
    MemoryUsage,
    TokenEfficiency,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub warmup_iterations: u32,
    pub measurement_iterations: u32,
    pub timeout_seconds: u64,
    pub parallel_factor: u32,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 3,
            measurement_iterations: 10,
            timeout_seconds: 300,
            parallel_factor: 1,
        }
    }
}

/// Request to trigger a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBenchmarkRequest {
    pub suite_id: String,
    pub benchmark_ids: Option<Vec<String>>,
    pub config_overrides: Option<BenchmarkConfig>,
    pub target: BenchmarkTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTarget {
    pub target_type: TargetType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    Model { provider: String, model: String },
    Endpoint { url: String },
    Local { path: String },
}

/// Benchmark run status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub run_id: String,
    pub suite_id: String,
    pub status: RunStatus,
    pub progress: f32,
    pub results: Option<Vec<BenchmarkResult>>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
}

/// Query for benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuery {
    pub suite_id: Option<String>,
    pub benchmark_id: Option<String>,
    pub target_type: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: usize,
}

/// Trait defining TestBench runtime adapter operations
#[async_trait]
pub trait TestBenchRuntimeAdapter: Send + Sync {
    /// List available benchmark suites
    async fn list_suites(&self) -> AdapterResult<Vec<BenchmarkSuite>>;

    /// Get suite details
    async fn get_suite(&self, suite_id: &str) -> AdapterResult<BenchmarkSuite>;

    /// Trigger a benchmark run
    async fn trigger_benchmark(&self, request: TriggerBenchmarkRequest) -> AdapterResult<BenchmarkRun>;

    /// Get run status
    async fn get_run_status(&self, run_id: &str) -> AdapterResult<BenchmarkRun>;

    /// Query benchmark results
    async fn query_results(&self, query: BenchmarkQuery) -> AdapterResult<Vec<BenchmarkResult>>;

    /// Cancel a running benchmark
    async fn cancel_run(&self, run_id: &str) -> AdapterResult<BenchmarkRun>;

    /// Compare benchmark results
    async fn compare_results(&self, run_ids: Vec<String>) -> AdapterResult<ComparisonReport>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub runs: Vec<String>,
    pub metrics_comparison: HashMap<String, Vec<MetricComparison>>,
    pub winner: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub run_id: String,
    pub value: f64,
    pub relative_diff: f64,
}

/// HTTP client for LLM-Test-Bench service (runtime integration)
pub struct TestBenchRuntimeClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl TestBenchRuntimeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&T>,
    ) -> AdapterResult<R> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Sending {} request to {}", method, url);

        let response = with_retry(3, || async {
            self.circuit_breaker.call(|| async {
                let mut request = self.client.request(method.clone(), &url);
                if let Some(body) = body {
                    request = request.json(body);
                }
                request
                    .send()
                    .await
                    .map_err(|e| AdapterError::RequestFailed(e.to_string()))
            }).await
        }).await?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }
}

#[async_trait]
impl ModuleAdapter for TestBenchRuntimeClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Test-Bench is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let trigger_request: TriggerBenchmarkRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.trigger_benchmark(trigger_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Test-Bench".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "benchmark_suites".to_string(),
                "benchmark_execution".to_string(),
                "results_comparison".to_string(),
                "runtime_only".to_string(),
            ],
            endpoints: vec![
                "/suites".to_string(),
                "/runs".to_string(),
                "/results".to_string(),
                "/compare".to_string(),
            ],
        })
    }
}

#[async_trait]
impl TestBenchRuntimeAdapter for TestBenchRuntimeClient {
    async fn list_suites(&self) -> AdapterResult<Vec<BenchmarkSuite>> {
        debug!("Listing benchmark suites");
        self.send_request::<(), Vec<BenchmarkSuite>>(reqwest::Method::GET, "/suites", None).await
    }

    async fn get_suite(&self, suite_id: &str) -> AdapterResult<BenchmarkSuite> {
        debug!("Getting suite: {}", suite_id);
        let path = format!("/suites/{}", suite_id);
        self.send_request::<(), BenchmarkSuite>(reqwest::Method::GET, &path, None).await
    }

    async fn trigger_benchmark(&self, request: TriggerBenchmarkRequest) -> AdapterResult<BenchmarkRun> {
        info!("Triggering benchmark for suite: {}", request.suite_id);
        self.send_request(reqwest::Method::POST, "/runs", Some(&request)).await
    }

    async fn get_run_status(&self, run_id: &str) -> AdapterResult<BenchmarkRun> {
        debug!("Getting run status: {}", run_id);
        let path = format!("/runs/{}", run_id);
        self.send_request::<(), BenchmarkRun>(reqwest::Method::GET, &path, None).await
    }

    async fn query_results(&self, query: BenchmarkQuery) -> AdapterResult<Vec<BenchmarkResult>> {
        debug!("Querying benchmark results");
        self.send_request(reqwest::Method::POST, "/results/query", Some(&query)).await
    }

    async fn cancel_run(&self, run_id: &str) -> AdapterResult<BenchmarkRun> {
        info!("Cancelling run: {}", run_id);
        let path = format!("/runs/{}/cancel", run_id);
        self.send_request::<(), BenchmarkRun>(reqwest::Method::POST, &path, None).await
    }

    async fn compare_results(&self, run_ids: Vec<String>) -> AdapterResult<ComparisonReport> {
        debug!("Comparing {} runs", run_ids.len());
        let request = serde_json::json!({ "run_ids": run_ids });
        self.send_request(reqwest::Method::POST, "/compare", Some(&request)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = TestBenchRuntimeClient::new("http://localhost:8110");
        assert_eq!(client.base_url, "http://localhost:8110");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = TestBenchRuntimeClient::new("http://localhost:8110");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Test-Bench");
        assert!(caps.features.contains(&"runtime_only".to_string()));
    }
}
