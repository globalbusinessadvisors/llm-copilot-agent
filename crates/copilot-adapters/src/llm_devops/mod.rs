//! LLM-Dev-Ops Ecosystem Adapter Modules
//!
//! This module provides thin adapter implementations for consuming
//! LLM-Dev-Ops upstream services. These are additive integrations
//! that do not modify existing public APIs.
//!
//! Phase 2B: Runtime consumption layer

pub mod simulator;
pub mod router;
pub mod cost_ops;
pub mod memory_graph;
pub mod orchestrator;
pub mod observatory;
pub mod sentinel;
pub mod shield;
pub mod connector_hub;
pub mod data_vault;
pub mod policy_engine;
pub mod governance_dashboard;
pub mod auto_optimizer;
pub mod analytics_hub;
pub mod registry;
pub mod marketplace;
pub mod research_lab;

// Re-export all adapter traits and clients
pub use simulator::{SimulatorAdapter, SimulatorClient};
pub use router::{RouterAdapter, RouterClient};
pub use cost_ops::{CostOpsAdapter, CostOpsClient};
pub use memory_graph::{MemoryGraphAdapter, MemoryGraphClient};
pub use orchestrator::{LLMOrchestratorAdapter, LLMOrchestratorClient};
pub use observatory::{LLMObservatoryAdapter, LLMObservatoryClient};
pub use sentinel::{SentinelAdapter, SentinelClient};
pub use shield::{ShieldAdapter, ShieldClient};
pub use connector_hub::{ConnectorHubAdapter, ConnectorHubClient};
pub use data_vault::{DataVaultAdapter, DataVaultClient};
pub use policy_engine::{PolicyEngineAdapter, PolicyEngineClient};
pub use governance_dashboard::{GovernanceDashboardAdapter, GovernanceDashboardClient};
pub use auto_optimizer::{AutoOptimizerAdapter, AutoOptimizerClient};
pub use analytics_hub::{AnalyticsHubAdapter, AnalyticsHubClient};
pub use registry::{RegistryAdapter, RegistryClient};
pub use marketplace::{MarketplaceAdapter, MarketplaceClient};
pub use research_lab::{ResearchLabAdapter, ResearchLabClient};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common configuration for LLM-Dev-Ops service connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMDevOpsConfig {
    /// Base URLs for each service
    pub service_urls: HashMap<String, String>,
    /// Global timeout in seconds
    pub timeout_seconds: u64,
    /// Enable circuit breaker pattern
    pub circuit_breaker_enabled: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for LLMDevOpsConfig {
    fn default() -> Self {
        let mut service_urls = HashMap::new();
        service_urls.insert("simulator".to_string(), "http://localhost:8090".to_string());
        service_urls.insert("router".to_string(), "http://localhost:8091".to_string());
        service_urls.insert("cost_ops".to_string(), "http://localhost:8092".to_string());
        service_urls.insert("memory_graph".to_string(), "http://localhost:8093".to_string());
        service_urls.insert("orchestrator".to_string(), "http://localhost:8094".to_string());
        service_urls.insert("observatory".to_string(), "http://localhost:8095".to_string());
        service_urls.insert("sentinel".to_string(), "http://localhost:8096".to_string());
        service_urls.insert("shield".to_string(), "http://localhost:8097".to_string());
        service_urls.insert("connector_hub".to_string(), "http://localhost:8098".to_string());
        service_urls.insert("data_vault".to_string(), "http://localhost:8099".to_string());
        service_urls.insert("policy_engine".to_string(), "http://localhost:8100".to_string());
        service_urls.insert("governance_dashboard".to_string(), "http://localhost:8101".to_string());
        service_urls.insert("auto_optimizer".to_string(), "http://localhost:8102".to_string());
        service_urls.insert("analytics_hub".to_string(), "http://localhost:8103".to_string());
        service_urls.insert("registry".to_string(), "http://localhost:8104".to_string());
        service_urls.insert("marketplace".to_string(), "http://localhost:8105".to_string());
        service_urls.insert("research_lab".to_string(), "http://localhost:8106".to_string());

        Self {
            service_urls,
            timeout_seconds: 30,
            circuit_breaker_enabled: true,
            max_retries: 3,
        }
    }
}

/// Aggregated client for all LLM-Dev-Ops services
pub struct LLMDevOpsHub {
    pub simulator: SimulatorClient,
    pub router: RouterClient,
    pub cost_ops: CostOpsClient,
    pub memory_graph: MemoryGraphClient,
    pub orchestrator: LLMOrchestratorClient,
    pub observatory: LLMObservatoryClient,
    pub sentinel: SentinelClient,
    pub shield: ShieldClient,
    pub connector_hub: ConnectorHubClient,
    pub data_vault: DataVaultClient,
    pub policy_engine: PolicyEngineClient,
    pub governance_dashboard: GovernanceDashboardClient,
    pub auto_optimizer: AutoOptimizerClient,
    pub analytics_hub: AnalyticsHubClient,
    pub registry: RegistryClient,
    pub marketplace: MarketplaceClient,
    pub research_lab: ResearchLabClient,
}

impl LLMDevOpsHub {
    pub fn new(config: LLMDevOpsConfig) -> Self {
        Self {
            simulator: SimulatorClient::new(
                config.service_urls.get("simulator").cloned().unwrap_or_default()
            ),
            router: RouterClient::new(
                config.service_urls.get("router").cloned().unwrap_or_default()
            ),
            cost_ops: CostOpsClient::new(
                config.service_urls.get("cost_ops").cloned().unwrap_or_default()
            ),
            memory_graph: MemoryGraphClient::new(
                config.service_urls.get("memory_graph").cloned().unwrap_or_default()
            ),
            orchestrator: LLMOrchestratorClient::new(
                config.service_urls.get("orchestrator").cloned().unwrap_or_default()
            ),
            observatory: LLMObservatoryClient::new(
                config.service_urls.get("observatory").cloned().unwrap_or_default()
            ),
            sentinel: SentinelClient::new(
                config.service_urls.get("sentinel").cloned().unwrap_or_default()
            ),
            shield: ShieldClient::new(
                config.service_urls.get("shield").cloned().unwrap_or_default()
            ),
            connector_hub: ConnectorHubClient::new(
                config.service_urls.get("connector_hub").cloned().unwrap_or_default()
            ),
            data_vault: DataVaultClient::new(
                config.service_urls.get("data_vault").cloned().unwrap_or_default()
            ),
            policy_engine: PolicyEngineClient::new(
                config.service_urls.get("policy_engine").cloned().unwrap_or_default()
            ),
            governance_dashboard: GovernanceDashboardClient::new(
                config.service_urls.get("governance_dashboard").cloned().unwrap_or_default()
            ),
            auto_optimizer: AutoOptimizerClient::new(
                config.service_urls.get("auto_optimizer").cloned().unwrap_or_default()
            ),
            analytics_hub: AnalyticsHubClient::new(
                config.service_urls.get("analytics_hub").cloned().unwrap_or_default()
            ),
            registry: RegistryClient::new(
                config.service_urls.get("registry").cloned().unwrap_or_default()
            ),
            marketplace: MarketplaceClient::new(
                config.service_urls.get("marketplace").cloned().unwrap_or_default()
            ),
            research_lab: ResearchLabClient::new(
                config.service_urls.get("research_lab").cloned().unwrap_or_default()
            ),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LLMDevOpsConfig::default())
    }
}
