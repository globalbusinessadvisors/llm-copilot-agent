//! LLM-Research-Lab Adapter
//!
//! Thin adapter for consuming LLM-Research-Lab service.
//! Provides research experimentation and analysis capabilities.

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

/// Experiment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: String,
    pub experiment_type: ExperimentType,
    pub config: ExperimentConfig,
    pub status: ExperimentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperimentType {
    ABTest,
    PromptComparison,
    ModelEvaluation,
    ParameterSweep,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub variants: Vec<Variant>,
    pub metrics: Vec<String>,
    pub sample_size: u64,
    pub duration_hours: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub name: String,
    pub config: serde_json::Value,
    pub traffic_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperimentStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

/// Experiment results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: String,
    pub variant_results: Vec<VariantResult>,
    pub winner: Option<String>,
    pub statistical_significance: f64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResult {
    pub variant_id: String,
    pub sample_count: u64,
    pub metrics: HashMap<String, MetricResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricResult {
    pub mean: f64,
    pub std_dev: f64,
    pub confidence_interval: (f64, f64),
}

/// Dataset for research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub size: u64,
    pub schema: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait defining ResearchLab adapter operations
#[async_trait]
pub trait ResearchLabAdapter: Send + Sync {
    /// Create an experiment
    async fn create_experiment(&self, experiment: Experiment) -> AdapterResult<Experiment>;

    /// Start an experiment
    async fn start_experiment(&self, experiment_id: &str) -> AdapterResult<Experiment>;

    /// Get experiment results
    async fn get_results(&self, experiment_id: &str) -> AdapterResult<ExperimentResults>;

    /// List experiments
    async fn list_experiments(&self, status: Option<ExperimentStatus>) -> AdapterResult<Vec<Experiment>>;

    /// Upload a dataset
    async fn upload_dataset(&self, dataset: Dataset) -> AdapterResult<Dataset>;

    /// List datasets
    async fn list_datasets(&self) -> AdapterResult<Vec<Dataset>>;
}

/// HTTP client for LLM-Research-Lab service
pub struct ResearchLabClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl ResearchLabClient {
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
impl ModuleAdapter for ResearchLabClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Research-Lab is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let experiment: Experiment = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.create_experiment(experiment).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Research-Lab".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "experiments".to_string(),
                "ab_testing".to_string(),
                "datasets".to_string(),
                "statistical_analysis".to_string(),
            ],
            endpoints: vec![
                "/experiments".to_string(),
                "/experiments/{id}".to_string(),
                "/experiments/{id}/results".to_string(),
                "/datasets".to_string(),
            ],
        })
    }
}

#[async_trait]
impl ResearchLabAdapter for ResearchLabClient {
    async fn create_experiment(&self, experiment: Experiment) -> AdapterResult<Experiment> {
        info!("Creating experiment: {}", experiment.name);
        self.send_request(reqwest::Method::POST, "/experiments", Some(&experiment)).await
    }

    async fn start_experiment(&self, experiment_id: &str) -> AdapterResult<Experiment> {
        info!("Starting experiment: {}", experiment_id);
        let path = format!("/experiments/{}/start", experiment_id);
        self.send_request::<(), Experiment>(reqwest::Method::POST, &path, None).await
    }

    async fn get_results(&self, experiment_id: &str) -> AdapterResult<ExperimentResults> {
        debug!("Getting results for experiment: {}", experiment_id);
        let path = format!("/experiments/{}/results", experiment_id);
        self.send_request::<(), ExperimentResults>(reqwest::Method::GET, &path, None).await
    }

    async fn list_experiments(&self, status: Option<ExperimentStatus>) -> AdapterResult<Vec<Experiment>> {
        debug!("Listing experiments");
        let path = match status {
            Some(s) => format!("/experiments?status={:?}", s),
            None => "/experiments".to_string(),
        };
        self.send_request::<(), Vec<Experiment>>(reqwest::Method::GET, &path, None).await
    }

    async fn upload_dataset(&self, dataset: Dataset) -> AdapterResult<Dataset> {
        info!("Uploading dataset: {}", dataset.name);
        self.send_request(reqwest::Method::POST, "/datasets", Some(&dataset)).await
    }

    async fn list_datasets(&self) -> AdapterResult<Vec<Dataset>> {
        debug!("Listing datasets");
        self.send_request::<(), Vec<Dataset>>(reqwest::Method::GET, "/datasets", None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = ResearchLabClient::new("http://localhost:8106");
        assert_eq!(client.base_url, "http://localhost:8106");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = ResearchLabClient::new("http://localhost:8106");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Research-Lab");
    }
}
