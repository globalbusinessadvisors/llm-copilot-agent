//! LLM-Auto-Optimizer Adapter
//!
//! Thin adapter for consuming LLM-Auto-Optimizer service.
//! Provides automated optimization for prompts, models, and costs.

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

/// Request to optimize a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizePromptRequest {
    pub original_prompt: String,
    pub optimization_goals: Vec<OptimizationGoal>,
    pub constraints: OptimizationConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationGoal {
    ReduceTokens,
    ImproveClarify,
    ReduceCost,
    ImproveLatency,
    ImproveQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConstraints {
    pub max_token_reduction_percent: Option<f32>,
    pub preserve_intent: bool,
    pub target_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizePromptResponse {
    pub optimized_prompt: String,
    pub original_tokens: u32,
    pub optimized_tokens: u32,
    pub improvements: Vec<Improvement>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub improvement_type: String,
    pub description: String,
    pub impact_estimate: f32,
}

/// Request for model recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendationRequest {
    pub task_description: String,
    pub sample_inputs: Vec<String>,
    pub requirements: ModelRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequirements {
    pub max_cost_per_request: Option<f64>,
    pub max_latency_ms: Option<u64>,
    pub min_quality_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub model: String,
    pub provider: String,
    pub estimated_cost: f64,
    pub estimated_latency_ms: u64,
    pub quality_score: f32,
    pub reasoning: String,
}

/// Trait defining AutoOptimizer adapter operations
#[async_trait]
pub trait AutoOptimizerAdapter: Send + Sync {
    /// Optimize a prompt
    async fn optimize_prompt(&self, request: OptimizePromptRequest) -> AdapterResult<OptimizePromptResponse>;

    /// Get model recommendations
    async fn recommend_model(&self, request: ModelRecommendationRequest) -> AdapterResult<Vec<ModelRecommendation>>;

    /// Analyze usage patterns
    async fn analyze_patterns(&self, tenant_id: &str) -> AdapterResult<UsagePatternAnalysis>;

    /// Get optimization suggestions
    async fn get_suggestions(&self, tenant_id: &str) -> AdapterResult<Vec<OptimizationSuggestion>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatternAnalysis {
    pub tenant_id: String,
    pub analysis_period_days: u32,
    pub patterns: Vec<UsagePattern>,
    pub anomalies: Vec<UsageAnomaly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: f32,
    pub impact: PatternImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternImpact {
    Positive,
    Negative,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnomaly {
    pub anomaly_type: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub estimated_savings: f64,
    pub effort: EffortLevel,
    pub auto_applicable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    PromptOptimization,
    ModelSwitch,
    CachingStrategy,
    BatchingOpportunity,
    CostReduction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

/// HTTP client for LLM-Auto-Optimizer service
pub struct AutoOptimizerClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl AutoOptimizerClient {
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
impl ModuleAdapter for AutoOptimizerClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Auto-Optimizer is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let opt_request: OptimizePromptRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.optimize_prompt(opt_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Auto-Optimizer".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "prompt_optimization".to_string(),
                "model_recommendation".to_string(),
                "pattern_analysis".to_string(),
                "cost_optimization".to_string(),
            ],
            endpoints: vec![
                "/optimize/prompt".to_string(),
                "/recommend/model".to_string(),
                "/analyze/{tenant_id}".to_string(),
                "/suggestions/{tenant_id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl AutoOptimizerAdapter for AutoOptimizerClient {
    async fn optimize_prompt(&self, request: OptimizePromptRequest) -> AdapterResult<OptimizePromptResponse> {
        info!("Optimizing prompt with {} goals", request.optimization_goals.len());
        self.send_request(reqwest::Method::POST, "/optimize/prompt", Some(&request)).await
    }

    async fn recommend_model(&self, request: ModelRecommendationRequest) -> AdapterResult<Vec<ModelRecommendation>> {
        info!("Getting model recommendations");
        self.send_request(reqwest::Method::POST, "/recommend/model", Some(&request)).await
    }

    async fn analyze_patterns(&self, tenant_id: &str) -> AdapterResult<UsagePatternAnalysis> {
        debug!("Analyzing patterns for tenant: {}", tenant_id);
        let path = format!("/analyze/{}", tenant_id);
        self.send_request::<(), UsagePatternAnalysis>(reqwest::Method::GET, &path, None).await
    }

    async fn get_suggestions(&self, tenant_id: &str) -> AdapterResult<Vec<OptimizationSuggestion>> {
        debug!("Getting suggestions for tenant: {}", tenant_id);
        let path = format!("/suggestions/{}", tenant_id);
        self.send_request::<(), Vec<OptimizationSuggestion>>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = AutoOptimizerClient::new("http://localhost:8102");
        assert_eq!(client.base_url, "http://localhost:8102");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = AutoOptimizerClient::new("http://localhost:8102");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Auto-Optimizer");
    }
}
