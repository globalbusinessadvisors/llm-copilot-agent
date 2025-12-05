//! LLM-Governance-Dashboard Adapter
//!
//! Thin adapter for consuming LLM-Governance-Dashboard service.
//! Provides cost tracking, usage analytics, and governance reporting.

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

/// Dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub data_source: String,
    pub refresh_interval_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    CostOverTime,
    UsageByModel,
    TopUsers,
    ComplianceScore,
    AlertsSummary,
    CustomMetric,
}

/// Governance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceReport {
    pub id: String,
    pub report_type: ReportType,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub summary: ReportSummary,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportType {
    CostReport,
    UsageReport,
    ComplianceReport,
    SecurityReport,
    Executive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_cost_usd: f64,
    pub total_requests: u64,
    pub compliance_score: f32,
    pub top_models: Vec<ModelUsage>,
    pub alerts_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub request_count: u64,
    pub cost_usd: f64,
    pub avg_latency_ms: u64,
}

/// Usage quota configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuota {
    pub id: String,
    pub name: String,
    pub scope: QuotaScope,
    pub limit_type: LimitType,
    pub limit_value: u64,
    pub current_usage: u64,
    pub reset_period: ResetPeriod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuotaScope {
    Tenant { tenant_id: String },
    User { user_id: String },
    Team { team_id: String },
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LimitType {
    Requests,
    Tokens,
    Cost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResetPeriod {
    Daily,
    Weekly,
    Monthly,
    Never,
}

/// Trait defining GovernanceDashboard adapter operations
#[async_trait]
pub trait GovernanceDashboardAdapter: Send + Sync {
    /// Get dashboard data
    async fn get_dashboard(&self, dashboard_id: &str) -> AdapterResult<Vec<DashboardWidget>>;

    /// Generate a report
    async fn generate_report(&self, report_type: ReportType, days: u32) -> AdapterResult<GovernanceReport>;

    /// Get usage quotas
    async fn get_quotas(&self) -> AdapterResult<Vec<UsageQuota>>;

    /// Set usage quota
    async fn set_quota(&self, quota: UsageQuota) -> AdapterResult<UsageQuota>;

    /// Get compliance status
    async fn get_compliance_status(&self) -> AdapterResult<ComplianceStatus>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub overall_score: f32,
    pub checks: Vec<ComplianceCheck>,
    pub last_audit: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub check_id: String,
    pub name: String,
    pub status: CheckStatus,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckStatus {
    Passed,
    Failed,
    Warning,
    NotApplicable,
}

/// HTTP client for LLM-Governance-Dashboard service
pub struct GovernanceDashboardClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl GovernanceDashboardClient {
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
impl ModuleAdapter for GovernanceDashboardClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Governance-Dashboard is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let report_type: ReportType = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.generate_report(report_type, 30).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Governance-Dashboard".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "cost_tracking".to_string(),
                "usage_analytics".to_string(),
                "compliance_reporting".to_string(),
                "quota_management".to_string(),
            ],
            endpoints: vec![
                "/dashboards/{id}".to_string(),
                "/reports".to_string(),
                "/quotas".to_string(),
                "/compliance".to_string(),
            ],
        })
    }
}

#[async_trait]
impl GovernanceDashboardAdapter for GovernanceDashboardClient {
    async fn get_dashboard(&self, dashboard_id: &str) -> AdapterResult<Vec<DashboardWidget>> {
        debug!("Getting dashboard: {}", dashboard_id);
        let path = format!("/dashboards/{}", dashboard_id);
        self.send_request::<(), Vec<DashboardWidget>>(reqwest::Method::GET, &path, None).await
    }

    async fn generate_report(&self, report_type: ReportType, days: u32) -> AdapterResult<GovernanceReport> {
        info!("Generating {:?} report for {} days", report_type, days);
        let request = serde_json::json!({
            "report_type": report_type,
            "period_days": days
        });
        self.send_request(reqwest::Method::POST, "/reports", Some(&request)).await
    }

    async fn get_quotas(&self) -> AdapterResult<Vec<UsageQuota>> {
        debug!("Getting usage quotas");
        self.send_request::<(), Vec<UsageQuota>>(reqwest::Method::GET, "/quotas", None).await
    }

    async fn set_quota(&self, quota: UsageQuota) -> AdapterResult<UsageQuota> {
        info!("Setting quota: {}", quota.name);
        self.send_request(reqwest::Method::POST, "/quotas", Some(&quota)).await
    }

    async fn get_compliance_status(&self) -> AdapterResult<ComplianceStatus> {
        debug!("Getting compliance status");
        self.send_request::<(), ComplianceStatus>(reqwest::Method::GET, "/compliance", None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = GovernanceDashboardClient::new("http://localhost:8101");
        assert_eq!(client.base_url, "http://localhost:8101");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = GovernanceDashboardClient::new("http://localhost:8101");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Governance-Dashboard");
    }
}
