//! LLM-Policy-Engine Adapter
//!
//! Thin adapter for consuming LLM-Policy-Engine service.
//! Provides policy definition, evaluation, and enforcement.

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

/// Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub rules: Vec<PolicyRule>,
    pub effect: PolicyEffect,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub rule_id: String,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    RequireApproval,
    Log,
    Transform { transformation: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Request to evaluate policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRequest {
    pub resource: String,
    pub action: String,
    pub context: HashMap<String, serde_json::Value>,
    pub subject: Subject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub subject_type: String,
    pub id: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResponse {
    pub decision: Decision,
    pub matched_policies: Vec<String>,
    pub reasons: Vec<String>,
    pub obligations: Vec<Obligation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny,
    NotApplicable,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub obligation_id: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Trait defining PolicyEngine adapter operations
#[async_trait]
pub trait PolicyEngineAdapter: Send + Sync {
    /// Create a new policy
    async fn create_policy(&self, policy: Policy) -> AdapterResult<Policy>;

    /// Evaluate policies for a request
    async fn evaluate(&self, request: EvaluationRequest) -> AdapterResult<EvaluationResponse>;

    /// List policies
    async fn list_policies(&self) -> AdapterResult<Vec<Policy>>;

    /// Update a policy
    async fn update_policy(&self, policy: Policy) -> AdapterResult<Policy>;

    /// Delete a policy
    async fn delete_policy(&self, policy_id: &str) -> AdapterResult<()>;
}

/// HTTP client for LLM-Policy-Engine service
pub struct PolicyEngineClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl PolicyEngineClient {
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
impl ModuleAdapter for PolicyEngineClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Policy-Engine is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let eval_request: EvaluationRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.evaluate(eval_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Policy-Engine".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "policy_evaluation".to_string(),
                "rbac".to_string(),
                "abac".to_string(),
                "obligations".to_string(),
            ],
            endpoints: vec![
                "/policies".to_string(),
                "/evaluate".to_string(),
                "/policies/{id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl PolicyEngineAdapter for PolicyEngineClient {
    async fn create_policy(&self, policy: Policy) -> AdapterResult<Policy> {
        info!("Creating policy: {}", policy.name);
        self.send_request(reqwest::Method::POST, "/policies", Some(&policy)).await
    }

    async fn evaluate(&self, request: EvaluationRequest) -> AdapterResult<EvaluationResponse> {
        debug!("Evaluating policies for resource: {}", request.resource);
        let response: EvaluationResponse = self.send_request(
            reqwest::Method::POST,
            "/evaluate",
            Some(&request)
        ).await?;

        if matches!(response.decision, Decision::Deny) {
            warn!("Policy evaluation denied for resource: {}", request.resource);
        }

        Ok(response)
    }

    async fn list_policies(&self) -> AdapterResult<Vec<Policy>> {
        debug!("Listing policies");
        self.send_request::<(), Vec<Policy>>(reqwest::Method::GET, "/policies", None).await
    }

    async fn update_policy(&self, policy: Policy) -> AdapterResult<Policy> {
        info!("Updating policy: {}", policy.id);
        let path = format!("/policies/{}", policy.id);
        self.send_request(reqwest::Method::PUT, &path, Some(&policy)).await
    }

    async fn delete_policy(&self, policy_id: &str) -> AdapterResult<()> {
        info!("Deleting policy: {}", policy_id);
        let path = format!("/policies/{}", policy_id);
        self.send_request::<(), ()>(reqwest::Method::DELETE, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = PolicyEngineClient::new("http://localhost:8100");
        assert_eq!(client.base_url, "http://localhost:8100");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = PolicyEngineClient::new("http://localhost:8100");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Policy-Engine");
    }
}
