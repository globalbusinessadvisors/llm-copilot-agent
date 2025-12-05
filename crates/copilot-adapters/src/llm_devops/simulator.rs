//! LLM-Simulator Adapter
//!
//! Thin adapter for consuming LLM-Simulator service.
//! Provides offline LLM API simulation for testing and development.

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

/// Request to simulate an LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub simulation_mode: SimulationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationMode {
    Deterministic,
    Stochastic,
    Replay { recording_id: String },
    LatencyInjection { min_ms: u64, max_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub tokens_used: u32,
    pub latency_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingRequest {
    pub name: String,
    pub model: String,
    pub requests: Vec<SimulationRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub name: String,
    pub model: String,
    pub request_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait defining LLM-Simulator adapter operations
#[async_trait]
pub trait SimulatorAdapter: Send + Sync {
    /// Simulate an LLM completion request
    async fn simulate(&self, request: SimulationRequest) -> AdapterResult<SimulationResponse>;

    /// Create a recording for replay mode
    async fn create_recording(&self, request: RecordingRequest) -> AdapterResult<Recording>;

    /// List available recordings
    async fn list_recordings(&self) -> AdapterResult<Vec<Recording>>;

    /// Get simulator statistics
    async fn get_stats(&self) -> AdapterResult<SimulatorStats>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorStats {
    pub total_requests: u64,
    pub avg_latency_ms: f64,
    pub active_recordings: usize,
    pub uptime_seconds: u64,
}

/// HTTP client for LLM-Simulator service
pub struct SimulatorClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl SimulatorClient {
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
impl ModuleAdapter for SimulatorClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Simulator is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let sim_request: SimulationRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.simulate(sim_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Simulator".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "offline_simulation".to_string(),
                "deterministic_mode".to_string(),
                "replay_recordings".to_string(),
                "latency_injection".to_string(),
            ],
            endpoints: vec![
                "/simulate".to_string(),
                "/recordings".to_string(),
                "/stats".to_string(),
            ],
        })
    }
}

#[async_trait]
impl SimulatorAdapter for SimulatorClient {
    async fn simulate(&self, request: SimulationRequest) -> AdapterResult<SimulationResponse> {
        info!("Simulating LLM request for model: {}", request.model);
        self.send_request(reqwest::Method::POST, "/simulate", Some(&request)).await
    }

    async fn create_recording(&self, request: RecordingRequest) -> AdapterResult<Recording> {
        info!("Creating recording: {}", request.name);
        self.send_request(reqwest::Method::POST, "/recordings", Some(&request)).await
    }

    async fn list_recordings(&self) -> AdapterResult<Vec<Recording>> {
        debug!("Listing recordings");
        self.send_request::<(), Vec<Recording>>(reqwest::Method::GET, "/recordings", None).await
    }

    async fn get_stats(&self) -> AdapterResult<SimulatorStats> {
        debug!("Getting simulator stats");
        self.send_request::<(), SimulatorStats>(reqwest::Method::GET, "/stats", None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SimulatorClient::new("http://localhost:8090");
        assert_eq!(client.base_url, "http://localhost:8090");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = SimulatorClient::new("http://localhost:8090");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Simulator");
    }
}
