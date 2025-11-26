//! Workflow engine with state machine and execution control

use crate::approval::{ApprovalGate, ApprovalRequest, ApprovalStatus};
use crate::dag::WorkflowDag;
use crate::execution::{DefaultStepExecutor, ExecutionContext, StepExecutor};
use crate::step::{StepResult, StepState, WorkflowStep};
use crate::{Result, WorkflowError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Status of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending execution
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow is paused (e.g., waiting for approval)
    Paused,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
}

/// Complete state of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow ID
    pub workflow_id: String,
    /// Execution ID
    pub execution_id: String,
    /// Current status
    pub status: WorkflowStatus,
    /// Completed step IDs
    pub completed_steps: HashSet<String>,
    /// Currently running step IDs
    pub running_steps: HashSet<String>,
    /// Failed step IDs
    pub failed_steps: HashSet<String>,
    /// Skipped step IDs
    pub skipped_steps: HashSet<String>,
    /// Step results
    pub step_results: HashMap<String, StepResult>,
    /// Pending approval IDs
    pub pending_approvals: Vec<String>,
    /// Execution start time
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Execution end time
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

impl WorkflowState {
    /// Create a new workflow state
    pub fn new(workflow_id: impl Into<String>, execution_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            execution_id: execution_id.into(),
            status: WorkflowStatus::Pending,
            completed_steps: HashSet::new(),
            running_steps: HashSet::new(),
            failed_steps: HashSet::new(),
            skipped_steps: HashSet::new(),
            step_results: HashMap::new(),
            pending_approvals: Vec::new(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Get progress as a percentage
    pub fn progress_percent(&self, total_steps: usize) -> f64 {
        if total_steps == 0 {
            return 100.0;
        }

        let completed = self.completed_steps.len() + self.failed_steps.len() + self.skipped_steps.len();
        (completed as f64 / total_steps as f64) * 100.0
    }

    /// Check if workflow is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: String,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Workflow metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Maximum execution time in seconds
    pub timeout_secs: Option<u64>,
}

impl WorkflowDefinition {
    /// Create a new workflow definition
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            steps: Vec::new(),
            metadata: HashMap::new(),
            timeout_secs: None,
        }
    }

    /// Add a step to the workflow
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set workflow ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    /// Validate the workflow definition
    pub fn validate(&self) -> Result<()> {
        if self.steps.is_empty() {
            return Err(WorkflowError::InvalidDefinition(
                "Workflow has no steps".to_string(),
            ));
        }

        // Create DAG to validate structure
        WorkflowDag::new(self.steps.clone())?;

        Ok(())
    }
}

/// Workflow engine
#[derive(Clone)]
pub struct WorkflowEngine {
    /// Active workflow executions
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    /// Approval gate
    approval_gate: Arc<ApprovalGate>,
    /// Step executor
    executor: Arc<dyn StepExecutor>,
}

/// Internal workflow execution state
struct WorkflowExecution {
    definition: WorkflowDefinition,
    dag: WorkflowDag,
    state: WorkflowState,
    context: ExecutionContext,
    cancel_flag: Arc<RwLock<bool>>,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            approval_gate: Arc::new(ApprovalGate::new()),
            executor: Arc::new(DefaultStepExecutor::new()),
        }
    }

    /// Create a new workflow engine with custom executor
    pub fn with_executor(executor: Arc<dyn StepExecutor>) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            approval_gate: Arc::new(ApprovalGate::new()),
            executor,
        }
    }

    /// Create and validate a workflow
    pub async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<String> {
        // Validate the definition
        definition.validate()?;

        tracing::info!(
            workflow_id = %definition.id,
            name = %definition.name,
            step_count = definition.steps.len(),
            "Workflow created"
        );

        Ok(definition.id.clone())
    }

    /// Execute a workflow
    pub async fn execute_workflow(&self, definition: WorkflowDefinition) -> Result<String> {
        let workflow_id = definition.id.clone();

        // Validate
        definition.validate()?;

        // Create DAG
        let dag = WorkflowDag::new(definition.steps.clone())?;

        // Create execution
        let execution_id = Uuid::new_v4().to_string();
        let mut state = WorkflowState::new(&workflow_id, &execution_id);
        state.status = WorkflowStatus::Running;
        state.started_at = Some(chrono::Utc::now());

        let context = ExecutionContext::new(&workflow_id, &execution_id);
        let cancel_flag = Arc::new(RwLock::new(false));

        let execution = WorkflowExecution {
            definition,
            dag,
            state,
            context,
            cancel_flag: cancel_flag.clone(),
        };

        // Store execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), execution);
        }

        tracing::info!(
            workflow_id = %workflow_id,
            execution_id = %execution_id,
            "Workflow execution started"
        );

        // Spawn execution task
        let engine = self.clone();
        let exec_id = execution_id.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.run_workflow_loop(&exec_id).await {
                tracing::error!(
                    execution_id = %exec_id,
                    error = %e,
                    "Workflow execution failed"
                );
            }
        });

        Ok(execution_id)
    }

    /// Main workflow execution loop
    async fn run_workflow_loop(&self, execution_id: &str) -> Result<()> {
        loop {
            // Check if cancelled
            let cancelled = {
                let executions = self.executions.read().await;
                let execution = executions.get(execution_id)
                    .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;
                let flag = *execution.cancel_flag.read().await;
                flag
            };

            if cancelled {
                self.mark_workflow_cancelled(execution_id).await?;
                break;
            }

            // Get ready steps
            let ready_steps = {
                let executions = self.executions.read().await;
                let execution = executions.get(execution_id)
                    .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

                execution.dag.get_ready_steps(&execution.state.completed_steps)
            };

            // Filter out already running or completed steps
            let steps_to_run: Vec<_> = {
                let executions = self.executions.read().await;
                let execution = executions.get(execution_id)
                    .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

                ready_steps
                    .into_iter()
                    .filter(|id| {
                        !execution.state.running_steps.contains(id)
                            && !execution.state.completed_steps.contains(id)
                            && !execution.state.failed_steps.contains(id)
                            && !execution.state.skipped_steps.contains(id)
                    })
                    .collect()
            };

            if steps_to_run.is_empty() {
                // Check if workflow is complete
                let is_complete = {
                    let executions = self.executions.read().await;
                    let execution = executions.get(execution_id)
                        .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

                    execution.state.running_steps.is_empty()
                        && execution.dag.get_ready_steps(&execution.state.completed_steps).is_empty()
                };

                if is_complete {
                    self.mark_workflow_complete(execution_id).await?;
                    break;
                }

                // Wait a bit before checking again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            // Execute ready steps
            for step_id in steps_to_run {
                let engine = self.clone();
                let exec_id = execution_id.to_string();

                tokio::spawn(async move {
                    if let Err(e) = engine.execute_step(&exec_id, &step_id).await {
                        tracing::error!(
                            execution_id = %exec_id,
                            step_id = %step_id,
                            error = %e,
                            "Step execution failed"
                        );
                    }
                });
            }

            // Small delay to avoid busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Execute a single step
    async fn execute_step(&self, execution_id: &str, step_id: &str) -> Result<()> {
        // Mark step as running
        {
            let mut executions = self.executions.write().await;
            let execution = executions.get_mut(execution_id)
                .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;
            execution.state.running_steps.insert(step_id.to_string());
        }

        // Get step and context
        let (step, context) = {
            let executions = self.executions.read().await;
            let execution = executions.get(execution_id)
                .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

            let step = execution.dag.get_step(step_id)
                .ok_or_else(|| WorkflowError::InvalidDefinition(
                    format!("Step not found: {}", step_id)
                ))?
                .clone();

            (step, execution.context.clone())
        };

        // Execute step
        let result = self.executor.execute_step(&step, &context).await?;

        // Update state
        {
            let mut executions = self.executions.write().await;
            let execution = executions.get_mut(execution_id)
                .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

            execution.state.running_steps.remove(step_id);

            match result.state {
                StepState::Completed => {
                    execution.state.completed_steps.insert(step_id.to_string());
                }
                StepState::Failed => {
                    execution.state.failed_steps.insert(step_id.to_string());

                    if step.fail_on_error {
                        execution.state.status = WorkflowStatus::Failed;
                        execution.state.error = result.error.clone();
                        execution.state.completed_at = Some(chrono::Utc::now());
                    }
                }
                StepState::Skipped => {
                    execution.state.skipped_steps.insert(step_id.to_string());
                }
                _ => {}
            }

            execution.state.step_results.insert(step_id.to_string(), result);
        }

        Ok(())
    }

    /// Mark workflow as complete
    async fn mark_workflow_complete(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        execution.state.status = WorkflowStatus::Completed;
        execution.state.completed_at = Some(chrono::Utc::now());

        tracing::info!(
            execution_id = %execution_id,
            "Workflow completed successfully"
        );

        Ok(())
    }

    /// Mark workflow as cancelled
    async fn mark_workflow_cancelled(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        execution.state.status = WorkflowStatus::Cancelled;
        execution.state.completed_at = Some(chrono::Utc::now());

        tracing::info!(
            execution_id = %execution_id,
            "Workflow cancelled"
        );

        Ok(())
    }

    /// Pause a workflow (typically for approval)
    pub async fn pause_workflow(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        if execution.state.status != WorkflowStatus::Running {
            return Err(WorkflowError::NotRunning(execution_id.to_string()));
        }

        execution.state.status = WorkflowStatus::Paused;

        tracing::info!(
            execution_id = %execution_id,
            "Workflow paused"
        );

        Ok(())
    }

    /// Resume a paused workflow
    pub async fn resume_workflow(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        if execution.state.status != WorkflowStatus::Paused {
            return Err(WorkflowError::InvalidDefinition(
                format!("Workflow is not paused: {:?}", execution.state.status)
            ));
        }

        execution.state.status = WorkflowStatus::Running;

        tracing::info!(
            execution_id = %execution_id,
            "Workflow resumed"
        );

        Ok(())
    }

    /// Cancel a workflow
    pub async fn cancel_workflow(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        *execution.cancel_flag.write().await = true;

        tracing::info!(
            execution_id = %execution_id,
            "Workflow cancel requested"
        );

        Ok(())
    }

    /// Get workflow execution status
    pub async fn get_status(&self, execution_id: &str) -> Result<WorkflowState> {
        let executions = self.executions.read().await;
        let execution = executions.get(execution_id)
            .ok_or_else(|| WorkflowError::NotFound(execution_id.to_string()))?;

        Ok(execution.state.clone())
    }

    /// Get approval gate
    pub fn approval_gate(&self) -> &ApprovalGate {
        &self.approval_gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{StepAction, StepType};

    #[tokio::test]
    async fn test_workflow_definition() {
        let workflow = WorkflowDefinition::new("Test Workflow", "A test workflow")
            .add_step(WorkflowStep::new(
                "step1",
                StepType::Action,
                StepAction::Wait { duration_secs: 1 },
            ));

        assert!(workflow.validate().is_ok());
    }

    #[tokio::test]
    async fn test_workflow_engine() {
        let engine = WorkflowEngine::new();

        let workflow = WorkflowDefinition::new("Test Workflow", "A test workflow")
            .add_step(
                WorkflowStep::new(
                    "step1",
                    StepType::Action,
                    StepAction::Wait { duration_secs: 0 },
                )
                .with_id("step1")
            );

        let execution_id = engine.execute_workflow(workflow).await.unwrap();

        // Wait a bit for execution
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let status = engine.get_status(&execution_id).await.unwrap();
        assert!(matches!(
            status.status,
            WorkflowStatus::Running | WorkflowStatus::Completed
        ));
    }
}
