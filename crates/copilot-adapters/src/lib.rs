pub mod traits;
pub mod testbench;
pub mod observatory;
pub mod incident;
pub mod orchestrator;
pub mod circuit_breaker;
pub mod retry;

// Phase 2B: LLM-Dev-Ops Ecosystem Adapters
pub mod llm_devops;

// Phase 2B: Benchmark Runtime Integrations (NOT compile-time dependencies)
pub mod benchmarks;

pub use traits::{
    ModuleAdapter,
    TestBenchAdapter,
    ObservatoryAdapter,
    IncidentAdapter,
    OrchestratorAdapter,
};

pub use testbench::TestBenchClient;
pub use observatory::ObservatoryClient;
pub use incident::IncidentClient;
pub use orchestrator::OrchestratorClient;
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use retry::{RetryPolicy, with_retry};

// Re-export LLM-Dev-Ops adapters
pub use llm_devops::{
    LLMDevOpsConfig, LLMDevOpsHub,
    SimulatorAdapter, SimulatorClient,
    RouterAdapter, RouterClient,
    CostOpsAdapter, CostOpsClient,
    MemoryGraphAdapter, MemoryGraphClient,
    LLMOrchestratorAdapter, LLMOrchestratorClient,
    LLMObservatoryAdapter, LLMObservatoryClient,
    SentinelAdapter, SentinelClient,
    ShieldAdapter, ShieldClient,
    ConnectorHubAdapter, ConnectorHubClient,
    DataVaultAdapter, DataVaultClient,
    PolicyEngineAdapter, PolicyEngineClient,
    GovernanceDashboardAdapter, GovernanceDashboardClient,
    AutoOptimizerAdapter, AutoOptimizerClient,
    AnalyticsHubAdapter, AnalyticsHubClient,
    RegistryAdapter, RegistryClient,
    MarketplaceAdapter, MarketplaceClient,
    ResearchLabAdapter, ResearchLabClient,
};

// Re-export benchmark runtime adapters
pub use benchmarks::{
    BenchmarkRuntimeConfig, BenchmarkRuntimeHub,
    BenchmarkResult, BenchmarkCorpus, CorpusSource,
    TestBenchRuntimeAdapter, TestBenchRuntimeClient,
    BenchmarkExchangeAdapter, BenchmarkExchangeClient,
};

use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type AdapterResult<T> = Result<T, AdapterError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleCapabilities {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<serde_json::Value>,
}

impl HealthStatus {
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            healthy: true,
            message: message.into(),
            timestamp: chrono::Utc::now(),
            details: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
            timestamp: chrono::Utc::now(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}
