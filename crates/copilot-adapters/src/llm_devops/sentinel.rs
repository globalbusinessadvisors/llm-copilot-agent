//! LLM-Sentinel Adapter
//!
//! Thin adapter for consuming LLM-Sentinel service.
//! Provides anomaly detection and security monitoring.

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

/// Request to analyze for anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAnalysisRequest {
    pub request_id: String,
    pub model: String,
    pub prompt: String,
    pub response: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAnalysisResponse {
    pub request_id: String,
    pub is_anomaly: bool,
    pub anomaly_score: f64,
    pub detected_anomalies: Vec<DetectedAnomaly>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAnomaly {
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub confidence: f64,
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    PromptInjection,
    DataExfiltration,
    ModelManipulation,
    UnusualPattern,
    CostAnomaly,
    LatencyAnomaly,
    ContentViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    pub id: String,
    pub name: String,
    pub rule_type: RuleType,
    pub condition: String,
    pub action: RuleAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    Pattern,
    Threshold,
    ML,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    Block,
    Alert,
    Log,
    Quarantine,
}

/// Trait defining Sentinel adapter operations
#[async_trait]
pub trait SentinelAdapter: Send + Sync {
    /// Analyze request for anomalies
    async fn analyze(&self, request: AnomalyAnalysisRequest) -> AdapterResult<AnomalyAnalysisResponse>;

    /// Create a security rule
    async fn create_rule(&self, rule: SecurityRule) -> AdapterResult<SecurityRule>;

    /// List security rules
    async fn list_rules(&self) -> AdapterResult<Vec<SecurityRule>>;

    /// Get threat intelligence
    async fn get_threat_intel(&self) -> AdapterResult<ThreatIntelligence>;

    /// Report a security incident
    async fn report_incident(&self, incident: SecurityIncident) -> AdapterResult<SecurityIncident>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelligence {
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub known_threats: Vec<ThreatIndicator>,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIndicator {
    pub indicator_type: String,
    pub value: String,
    pub severity: AnomalySeverity,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub id: String,
    pub incident_type: String,
    pub severity: AnomalySeverity,
    pub description: String,
    pub affected_resources: Vec<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub status: IncidentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentStatus {
    Open,
    Investigating,
    Contained,
    Resolved,
}

/// HTTP client for LLM-Sentinel service
pub struct SentinelClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl SentinelClient {
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
impl ModuleAdapter for SentinelClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Sentinel is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let analysis_request: AnomalyAnalysisRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.analyze(analysis_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Sentinel".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "anomaly_detection".to_string(),
                "security_rules".to_string(),
                "threat_intelligence".to_string(),
                "incident_response".to_string(),
            ],
            endpoints: vec![
                "/analyze".to_string(),
                "/rules".to_string(),
                "/threat-intel".to_string(),
                "/incidents".to_string(),
            ],
        })
    }
}

#[async_trait]
impl SentinelAdapter for SentinelClient {
    async fn analyze(&self, request: AnomalyAnalysisRequest) -> AdapterResult<AnomalyAnalysisResponse> {
        debug!("Analyzing request: {}", request.request_id);
        let response: AnomalyAnalysisResponse = self.send_request(
            reqwest::Method::POST,
            "/analyze",
            Some(&request)
        ).await?;

        if response.is_anomaly {
            warn!("Anomaly detected in request {}: score={}", request.request_id, response.anomaly_score);
        }

        Ok(response)
    }

    async fn create_rule(&self, rule: SecurityRule) -> AdapterResult<SecurityRule> {
        info!("Creating security rule: {}", rule.name);
        self.send_request(reqwest::Method::POST, "/rules", Some(&rule)).await
    }

    async fn list_rules(&self) -> AdapterResult<Vec<SecurityRule>> {
        debug!("Listing security rules");
        self.send_request::<(), Vec<SecurityRule>>(reqwest::Method::GET, "/rules", None).await
    }

    async fn get_threat_intel(&self) -> AdapterResult<ThreatIntelligence> {
        debug!("Getting threat intelligence");
        self.send_request::<(), ThreatIntelligence>(reqwest::Method::GET, "/threat-intel", None).await
    }

    async fn report_incident(&self, incident: SecurityIncident) -> AdapterResult<SecurityIncident> {
        warn!("Reporting security incident: {} ({})", incident.incident_type, incident.severity);
        self.send_request(reqwest::Method::POST, "/incidents", Some(&incident)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SentinelClient::new("http://localhost:8096");
        assert_eq!(client.base_url, "http://localhost:8096");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = SentinelClient::new("http://localhost:8096");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Sentinel");
    }
}
