//! E2B Agent for autonomous task execution

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    config::E2BConfig,
    execution::{CodeExecutor, ExecutionResult},
    sandbox::{Sandbox, SandboxManager, SandboxStatus},
    Result, SandboxTemplate,
};

/// Represents a task for the agent to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task identifier
    pub id: String,

    /// Task description
    pub description: String,

    /// Code to execute
    pub code: String,

    /// Runtime to use (python, nodejs, bash, etc.)
    pub runtime: String,

    /// Task timeout
    pub timeout: Duration,

    /// Dependencies (other task IDs that must complete first)
    pub dependencies: Vec<String>,

    /// Task metadata
    pub metadata: std::collections::HashMap<String, String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl AgentTask {
    /// Create a new task
    pub fn new(description: impl Into<String>, code: impl Into<String>, runtime: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            code: code.into(),
            runtime: runtime.into(),
            timeout: Duration::from_secs(60),
            dependencies: Vec::new(),
            metadata: std::collections::HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add a dependency
    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Result of agent task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Task ID
    pub task_id: String,

    /// Whether the task succeeded
    pub success: bool,

    /// Execution result
    pub execution: Option<ExecutionResult>,

    /// Error message if failed
    pub error: Option<String>,

    /// Start timestamp
    pub started_at: DateTime<Utc>,

    /// End timestamp
    pub ended_at: DateTime<Utc>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl AgentResult {
    /// Create a successful result
    pub fn success(task_id: String, execution: ExecutionResult) -> Self {
        let now = Utc::now();
        let duration_ms = execution.duration_ms;
        Self {
            task_id,
            success: true,
            execution: Some(execution),
            error: None,
            started_at: now - chrono::Duration::milliseconds(duration_ms as i64),
            ended_at: now,
            duration_ms,
        }
    }

    /// Create a failed result
    pub fn failure(task_id: String, error: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            success: false,
            execution: None,
            error: Some(error.into()),
            started_at: now,
            ended_at: now,
            duration_ms: 0,
        }
    }
}

/// E2B Agent for autonomous task execution
pub struct E2BAgent {
    config: E2BConfig,
    sandbox_manager: SandboxManager,
    executor: CodeExecutor,
    current_sandbox: Option<Sandbox>,
}

impl E2BAgent {
    /// Create a new E2B agent
    pub async fn new(config: E2BConfig) -> Result<Self> {
        info!("Initializing E2B Agent");

        let sandbox_manager = SandboxManager::new(config.clone());
        let executor = CodeExecutor::new(config.timeout);

        Ok(Self {
            config,
            sandbox_manager,
            executor,
            current_sandbox: None,
        })
    }

    /// Create agent from environment configuration
    pub async fn from_env() -> Result<Self> {
        let config = E2BConfig::from_env()?;
        Self::new(config).await
    }

    /// Get or create a sandbox for execution
    pub async fn ensure_sandbox(&mut self, template: Option<SandboxTemplate>) -> Result<()> {
        // Check if current sandbox is active
        let needs_new = match &self.current_sandbox {
            Some(sandbox) => !sandbox.is_active(),
            None => true,
        };

        if needs_new {
            // Create new sandbox
            let sandbox = self.sandbox_manager.create(template).await?;
            self.current_sandbox = Some(sandbox);
        }

        Ok(())
    }

    /// Get current sandbox reference
    pub fn current_sandbox(&self) -> Option<&Sandbox> {
        self.current_sandbox.as_ref()
    }

    /// Execute a task
    pub async fn execute_task(&mut self, task: AgentTask) -> Result<AgentResult> {
        info!("Executing task: {} - {}", task.id, task.description);

        // Ensure we have an active sandbox
        let template = match task.runtime.as_str() {
            "python" | "python3" => SandboxTemplate::Python,
            "node" | "nodejs" | "javascript" => SandboxTemplate::NodeJs,
            "bash" | "shell" | "sh" => SandboxTemplate::Bash,
            "rust" => SandboxTemplate::Rust,
            "go" => SandboxTemplate::Go,
            _ => self.config.default_template,
        };

        self.ensure_sandbox(Some(template)).await?;

        // Execute the code using the executor
        let execution_result = self.executor
            .execute(&task.code, &task.runtime)
            .await?;

        if execution_result.is_success() {
            info!("Task {} completed successfully", task.id);
            Ok(AgentResult::success(task.id, execution_result))
        } else {
            let error = execution_result.error
                .map(|e| e.message)
                .unwrap_or_else(|| execution_result.stderr.clone());
            warn!("Task {} failed: {}", task.id, error);
            Ok(AgentResult::failure(task.id, error))
        }
    }

    /// Execute code directly
    pub async fn execute_code(&mut self, code: &str) -> Result<ExecutionResult> {
        self.ensure_sandbox(None).await?;
        self.executor.execute_python(code).await
    }

    /// Execute code with a specific runtime
    pub async fn execute_code_with_runtime(&mut self, code: &str, runtime: &str) -> Result<ExecutionResult> {
        self.ensure_sandbox(None).await?;
        self.executor.execute(code, runtime).await
    }

    /// Execute multiple tasks in sequence
    pub async fn execute_tasks(&mut self, tasks: Vec<AgentTask>) -> Vec<AgentResult> {
        let mut results = Vec::new();

        for task in tasks {
            match self.execute_task(task).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Failed to execute task: {}", e);
                    // Create failure result for this task
                    results.push(AgentResult::failure(
                        Uuid::new_v4().to_string(),
                        e.to_string(),
                    ));
                }
            }
        }

        results
    }

    /// Clean up agent resources
    pub async fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up E2B Agent resources");

        if let Some(ref sandbox) = self.current_sandbox {
            self.sandbox_manager.destroy(&sandbox.id).await?;
            self.current_sandbox = None;
        }

        Ok(())
    }

    /// Get current sandbox status
    pub fn sandbox_status(&self) -> Option<SandboxStatus> {
        self.current_sandbox.as_ref().map(|s| s.status)
    }
}

impl Drop for E2BAgent {
    fn drop(&mut self) {
        // Note: Cannot call async cleanup in Drop
        // Users should call cleanup() explicitly
        if self.current_sandbox.is_some() {
            warn!("E2B Agent dropped without cleanup - call cleanup() explicitly");
        }
    }
}

/// Trait for agent capabilities
#[async_trait]
pub trait AgentCapability: Send + Sync {
    /// Get capability name
    fn name(&self) -> &str;

    /// Check if this capability can handle the task
    fn can_handle(&self, task: &AgentTask) -> bool;

    /// Execute the task
    async fn execute(&self, task: &AgentTask) -> Result<AgentResult>;
}

/// Python code execution capability
pub struct PythonCapability {
    executor: CodeExecutor,
}

impl PythonCapability {
    pub fn new(timeout: Duration) -> Self {
        Self {
            executor: CodeExecutor::new(timeout),
        }
    }
}

#[async_trait]
impl AgentCapability for PythonCapability {
    fn name(&self) -> &str {
        "python"
    }

    fn can_handle(&self, task: &AgentTask) -> bool {
        matches!(task.runtime.as_str(), "python" | "python3")
    }

    async fn execute(&self, task: &AgentTask) -> Result<AgentResult> {
        let result = self.executor.execute_python(&task.code).await?;
        Ok(AgentResult::success(task.id.clone(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = AgentTask::new(
            "Test task",
            "print('hello')",
            "python",
        )
        .with_timeout(Duration::from_secs(30))
        .with_metadata("key", "value");

        assert!(!task.id.is_empty());
        assert_eq!(task.description, "Test task");
        assert_eq!(task.runtime, "python");
        assert_eq!(task.timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let config = E2BConfig::with_api_key("test-key");
        let agent = E2BAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_agent_execute_code() {
        let config = E2BConfig::with_api_key("test-key");
        let mut agent = E2BAgent::new(config).await.unwrap();

        let result = agent.execute_code("print('hello')").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_success());

        agent.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_agent_execute_task() {
        let config = E2BConfig::with_api_key("test-key");
        let mut agent = E2BAgent::new(config).await.unwrap();

        let task = AgentTask::new(
            "Test Python execution",
            "print('Hello from E2B!')",
            "python",
        );

        let result = agent.execute_task(task).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);

        agent.cleanup().await.unwrap();
    }
}
