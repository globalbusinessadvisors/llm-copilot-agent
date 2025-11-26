//! Workflow Engine for LLM CoPilot Agent
//!
//! This crate provides a comprehensive workflow execution engine with:
//! - DAG-based workflow definition and validation
//! - Parallel and sequential step execution
//! - Approval gates with timeout handling
//! - State management and persistence
//! - Retry logic with exponential backoff
//! - Real-time workflow status tracking
//! - Workflow versioning and rollback
//! - Scheduled workflow execution
//! - Event-driven workflow triggers
//! - Workflow templates library

pub mod approval;
pub mod dag;
pub mod engine;
pub mod execution;
pub mod step;
pub mod versioning;
pub mod scheduling;
pub mod triggers;
pub mod templates;

pub use approval::{ApprovalGate, ApprovalRequest, ApprovalStatus};
pub use dag::{WorkflowDag, DagValidationError};
pub use engine::{WorkflowEngine, WorkflowDefinition, WorkflowStatus, WorkflowState};
pub use execution::{ExecutionContext, StepExecutor, RetryConfig};
pub use step::{WorkflowStep, StepType, StepState, StepResult, StepAction};
pub use versioning::{WorkflowVersion, VersionManager, VersionBump, VersionRepository};
pub use scheduling::{Schedule, ScheduledWorkflow, WorkflowScheduler, ScheduleRepository};
pub use triggers::{TriggerEvent, TriggerCondition, WorkflowTrigger, TriggerManager, EventBus, EventSource};
pub use templates::{WorkflowTemplate, TemplateParameter, TemplateLibrary, TemplateBuilders};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),

    #[error("DAG validation error: {0}")]
    DagValidation(#[from] DagValidationError),

    #[error("Step execution failed: {step_id}: {reason}")]
    StepExecutionFailed {
        step_id: String,
        reason: String,
    },

    #[error("Workflow already running: {0}")]
    AlreadyRunning(String),

    #[error("Workflow not running: {0}")]
    NotRunning(String),

    #[error("Approval timeout: {0}")]
    ApprovalTimeout(String),

    #[error("Approval denied: {0}")]
    ApprovalDenied(String),

    #[error("Dependency failed: {0}")]
    DependencyFailed(String),

    #[error("Timeout exceeded: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    Core(#[from] copilot_core::AppError),
}

pub type Result<T> = std::result::Result<T, WorkflowError>;
