//! LLM-Shield Adapter
//!
//! Thin adapter for consuming LLM-Shield service.
//! Provides content filtering, PII protection, and safety guardrails.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::{
    AdapterError, AdapterResult, HealthStatus, ModuleCapabilities, ModuleAdapter,
    circuit_breaker::CircuitBreaker,
    retry::with_retry,
};

/// Request to screen content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningRequest {
    pub content: String,
    pub content_type: ContentType,
    pub screening_options: ScreeningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Prompt,
    Response,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningOptions {
    pub detect_pii: bool,
    pub detect_toxicity: bool,
    pub detect_bias: bool,
    pub detect_hallucination: bool,
    pub custom_filters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResponse {
    pub passed: bool,
    pub original_content: String,
    pub filtered_content: Option<String>,
    pub detections: Vec<Detection>,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub detection_type: DetectionType,
    pub severity: DetectionSeverity,
    pub description: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub redacted_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionType {
    PII(PIIType),
    Toxicity(ToxicityCategory),
    Bias(BiasType),
    Hallucination,
    CustomFilter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PIIType {
    Email,
    Phone,
    SSN,
    CreditCard,
    Address,
    Name,
    DateOfBirth,
    IPAddress,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToxicityCategory {
    Hate,
    Violence,
    Sexual,
    Harassment,
    SelfHarm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiasType {
    Gender,
    Racial,
    Age,
    Religious,
    Political,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Custom filter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFilter {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub filter_type: FilterType,
    pub action: FilterAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterType {
    Regex,
    Keyword,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    Block,
    Redact,
    Flag,
    Replace { replacement: String },
}

/// Trait defining Shield adapter operations
#[async_trait]
pub trait ShieldAdapter: Send + Sync {
    /// Screen content for safety
    async fn screen(&self, request: ScreeningRequest) -> AdapterResult<ScreeningResponse>;

    /// Create a custom filter
    async fn create_filter(&self, filter: CustomFilter) -> AdapterResult<CustomFilter>;

    /// List custom filters
    async fn list_filters(&self) -> AdapterResult<Vec<CustomFilter>>;

    /// Get safety statistics
    async fn get_stats(&self) -> AdapterResult<ShieldStats>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldStats {
    pub total_screened: u64,
    pub blocked_count: u64,
    pub pii_detections: u64,
    pub toxicity_detections: u64,
    pub avg_risk_score: f64,
}

/// HTTP client for LLM-Shield service
pub struct ShieldClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl ShieldClient {
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
impl ModuleAdapter for ShieldClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Shield is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let screen_request: ScreeningRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.screen(screen_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Shield".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "pii_detection".to_string(),
                "toxicity_detection".to_string(),
                "content_filtering".to_string(),
                "custom_filters".to_string(),
            ],
            endpoints: vec![
                "/screen".to_string(),
                "/filters".to_string(),
                "/stats".to_string(),
            ],
        })
    }
}

#[async_trait]
impl ShieldAdapter for ShieldClient {
    async fn screen(&self, request: ScreeningRequest) -> AdapterResult<ScreeningResponse> {
        debug!("Screening content");
        let response: ScreeningResponse = self.send_request(
            reqwest::Method::POST,
            "/screen",
            Some(&request)
        ).await?;

        if !response.passed {
            warn!("Content failed screening: risk_score={}", response.risk_score);
        }

        Ok(response)
    }

    async fn create_filter(&self, filter: CustomFilter) -> AdapterResult<CustomFilter> {
        info!("Creating custom filter: {}", filter.name);
        self.send_request(reqwest::Method::POST, "/filters", Some(&filter)).await
    }

    async fn list_filters(&self) -> AdapterResult<Vec<CustomFilter>> {
        debug!("Listing custom filters");
        self.send_request::<(), Vec<CustomFilter>>(reqwest::Method::GET, "/filters", None).await
    }

    async fn get_stats(&self) -> AdapterResult<ShieldStats> {
        debug!("Getting shield stats");
        self.send_request::<(), ShieldStats>(reqwest::Method::GET, "/stats", None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = ShieldClient::new("http://localhost:8097");
        assert_eq!(client.base_url, "http://localhost:8097");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = ShieldClient::new("http://localhost:8097");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Shield");
    }
}
