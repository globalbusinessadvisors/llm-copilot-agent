//! LLM-Orchestrator Adapter
//!
//! Thin adapter for consuming LLM-Orchestrator service.
//! Provides workflow orchestration and multi-agent coordination.

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

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub triggers: Vec<WorkflowTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub config: serde_json::Value,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    LLMCall,
    ApiCall,
    Transform,
    Conditional,
    Parallel,
    Human,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub trigger_type: TriggerType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    Manual,
    Schedule,
    Webhook,
    Event,
}

/// Request to execute a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWorkflowRequest {
    pub workflow_id: String,
    pub input: serde_json::Value,
    pub context: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionResult {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub step_results: HashMap<String, StepResult>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    WaitingForHuman,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: ExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Trait defining LLMOrchestrator adapter operations
#[async_trait]
pub trait LLMOrchestratorAdapter: Send + Sync {
    /// Create a new workflow definition
    async fn create_workflow(&self, definition: WorkflowDefinition) -> AdapterResult<WorkflowDefinition>;

    /// Execute a workflow
    async fn execute_workflow(&self, request: ExecuteWorkflowRequest) -> AdapterResult<WorkflowExecutionResult>;

    /// Get workflow execution status
    async fn get_execution(&self, execution_id: &str) -> AdapterResult<WorkflowExecutionResult>;

    /// List workflows
    async fn list_workflows(&self) -> AdapterResult<Vec<WorkflowDefinition>>;

    /// Cancel a workflow execution
    async fn cancel_execution(&self, execution_id: &str) -> AdapterResult<WorkflowExecutionResult>;
}

/// HTTP client for LLM-Orchestrator service
pub struct LLMOrchestratorClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl LLMOrchestratorClient {
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
impl ModuleAdapter for LLMOrchestratorClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Orchestrator is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let exec_request: ExecuteWorkflowRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.execute_workflow(exec_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Orchestrator".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "workflow_execution".to_string(),
                "multi_agent".to_string(),
                "human_in_loop".to_string(),
                "parallel_execution".to_string(),
            ],
            endpoints: vec![
                "/workflows".to_string(),
                "/workflows/execute".to_string(),
                "/executions/{id}".to_string(),
                "/executions/{id}/cancel".to_string(),
            ],
        })
    }
}

#[async_trait]
impl LLMOrchestratorAdapter for LLMOrchestratorClient {
    async fn create_workflow(&self, definition: WorkflowDefinition) -> AdapterResult<WorkflowDefinition> {
        info!("Creating workflow: {}", definition.name);
        self.send_request(reqwest::Method::POST, "/workflows", Some(&definition)).await
    }

    async fn execute_workflow(&self, request: ExecuteWorkflowRequest) -> AdapterResult<WorkflowExecutionResult> {
        info!("Executing workflow: {}", request.workflow_id);
        self.send_request(reqwest::Method::POST, "/workflows/execute", Some(&request)).await
    }

    async fn get_execution(&self, execution_id: &str) -> AdapterResult<WorkflowExecutionResult> {
        debug!("Getting execution: {}", execution_id);
        let path = format!("/executions/{}", execution_id);
        self.send_request::<(), WorkflowExecutionResult>(reqwest::Method::GET, &path, None).await
    }

    async fn list_workflows(&self) -> AdapterResult<Vec<WorkflowDefinition>> {
        debug!("Listing workflows");
        self.send_request::<(), Vec<WorkflowDefinition>>(reqwest::Method::GET, "/workflows", None).await
    }

    async fn cancel_execution(&self, execution_id: &str) -> AdapterResult<WorkflowExecutionResult> {
        info!("Cancelling execution: {}", execution_id);
        let path = format!("/executions/{}/cancel", execution_id);
        self.send_request::<(), WorkflowExecutionResult>(reqwest::Method::POST, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = LLMOrchestratorClient::new("http://localhost:8094");
        assert_eq!(client.base_url, "http://localhost:8094");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = LLMOrchestratorClient::new("http://localhost:8094");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Orchestrator");
    }
}
