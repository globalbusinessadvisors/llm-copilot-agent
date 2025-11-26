//! Quota management for tenants
//!
//! Provides resource quota enforcement and tracking.

use crate::{Result, Tenant, TenantError, TenantTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, warn};

/// Quota type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaType {
    /// API calls per billing period
    ApiCalls,
    /// Tokens processed per billing period
    Tokens,
    /// Storage in bytes
    Storage,
    /// Number of users
    Users,
    /// Number of workflows
    Workflows,
    /// Number of concurrent workflows
    ConcurrentWorkflows,
    /// Number of conversations
    Conversations,
    /// Number of API keys
    ApiKeys,
    /// Number of context items
    ContextItems,
    /// Number of webhooks
    Webhooks,
}

impl QuotaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiCalls => "api_calls",
            Self::Tokens => "tokens",
            Self::Storage => "storage",
            Self::Users => "users",
            Self::Workflows => "workflows",
            Self::ConcurrentWorkflows => "concurrent_workflows",
            Self::Conversations => "conversations",
            Self::ApiKeys => "api_keys",
            Self::ContextItems => "context_items",
            Self::Webhooks => "webhooks",
        }
    }

    /// Check if this quota resets periodically
    pub fn is_periodic(&self) -> bool {
        matches!(self, Self::ApiCalls | Self::Tokens)
    }
}

/// Quota limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaLimit {
    /// Maximum allowed value
    pub limit: u64,
    /// Warning threshold (percentage)
    pub warning_threshold: f64,
    /// Whether to hard-enforce the limit
    pub hard_limit: bool,
}

impl QuotaLimit {
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            warning_threshold: 0.8,
            hard_limit: true,
        }
    }

    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_soft_limit(mut self) -> Self {
        self.hard_limit = false;
        self
    }
}

/// Quota usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsage {
    /// Quota type
    pub quota_type: QuotaType,
    /// Current usage
    pub current: u64,
    /// Limit
    pub limit: u64,
    /// Percentage used
    pub percentage: f64,
    /// Whether limit is exceeded
    pub exceeded: bool,
    /// Whether warning threshold is reached
    pub warning: bool,
    /// Period start (for periodic quotas)
    pub period_start: Option<DateTime<Utc>>,
    /// Period end (for periodic quotas)
    pub period_end: Option<DateTime<Utc>>,
}

impl QuotaUsage {
    pub fn new(quota_type: QuotaType, current: u64, limit: u64) -> Self {
        let percentage = if limit > 0 {
            (current as f64 / limit as f64) * 100.0
        } else {
            0.0
        };

        Self {
            quota_type,
            current,
            limit,
            percentage,
            exceeded: current > limit,
            warning: percentage >= 80.0,
            period_start: None,
            period_end: None,
        }
    }

    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.current)
    }
}

/// Tenant quota configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotas {
    /// Limits for each quota type
    pub limits: HashMap<QuotaType, QuotaLimit>,
}

impl TenantQuotas {
    /// Create default quotas for a tier
    pub fn for_tier(tier: TenantTier) -> Self {
        let mut limits = HashMap::new();

        limits.insert(QuotaType::ApiCalls, QuotaLimit::new(tier.default_api_calls_limit()));
        limits.insert(QuotaType::Tokens, QuotaLimit::new(tier.default_token_limit()));
        limits.insert(QuotaType::Storage, QuotaLimit::new(tier.default_storage_limit()));
        limits.insert(QuotaType::Users, QuotaLimit::new(tier.default_max_users() as u64));
        limits.insert(
            QuotaType::ConcurrentWorkflows,
            QuotaLimit::new(tier.default_max_concurrent_workflows() as u64),
        );

        // Add other default limits based on tier
        let (workflows, conversations, api_keys, context_items, webhooks) = match tier {
            TenantTier::Free => (5, 100, 2, 100, 1),
            TenantTier::Starter => (25, 1000, 5, 1000, 5),
            TenantTier::Professional => (100, 10000, 20, 10000, 20),
            TenantTier::Business => (500, 100000, 50, 100000, 50),
            TenantTier::Enterprise | TenantTier::Custom => (u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX),
        };

        limits.insert(QuotaType::Workflows, QuotaLimit::new(workflows));
        limits.insert(QuotaType::Conversations, QuotaLimit::new(conversations));
        limits.insert(QuotaType::ApiKeys, QuotaLimit::new(api_keys));
        limits.insert(QuotaType::ContextItems, QuotaLimit::new(context_items));
        limits.insert(QuotaType::Webhooks, QuotaLimit::new(webhooks));

        Self { limits }
    }

    /// Get limit for a quota type
    pub fn get_limit(&self, quota_type: &QuotaType) -> Option<u64> {
        self.limits.get(quota_type).map(|l| l.limit)
    }

    /// Set a custom limit
    pub fn set_limit(&mut self, quota_type: QuotaType, limit: QuotaLimit) {
        self.limits.insert(quota_type, limit);
    }
}

/// Quota manager for tracking and enforcing quotas
pub struct QuotaManager {
    /// Current usage per tenant
    usage: Arc<RwLock<HashMap<String, HashMap<QuotaType, u64>>>>,
    /// Quota configurations per tenant
    quotas: Arc<RwLock<HashMap<String, TenantQuotas>>>,
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            usage: Arc::new(RwLock::new(HashMap::new())),
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register quotas for a tenant
    pub fn register_tenant(&self, tenant: &Tenant) {
        let quotas = TenantQuotas::for_tier(tenant.tier);
        self.quotas.write().insert(tenant.id.clone(), quotas);
        self.usage.write().insert(tenant.id.clone(), HashMap::new());
        debug!(tenant_id = %tenant.id, tier = ?tenant.tier, "Registered tenant quotas");
    }

    /// Remove tenant quotas
    pub fn remove_tenant(&self, tenant_id: &str) {
        self.quotas.write().remove(tenant_id);
        self.usage.write().remove(tenant_id);
        debug!(tenant_id = %tenant_id, "Removed tenant quotas");
    }

    /// Get current usage for a quota
    pub fn get_usage(&self, tenant_id: &str, quota_type: QuotaType) -> Option<QuotaUsage> {
        let usage = self.usage.read();
        let quotas = self.quotas.read();

        let current = usage
            .get(tenant_id)?
            .get(&quota_type)
            .copied()
            .unwrap_or(0);

        let limit = quotas
            .get(tenant_id)?
            .get_limit(&quota_type)
            .unwrap_or(0);

        Some(QuotaUsage::new(quota_type, current, limit))
    }

    /// Get all usage for a tenant
    pub fn get_all_usage(&self, tenant_id: &str) -> HashMap<QuotaType, QuotaUsage> {
        let usage = self.usage.read();
        let quotas = self.quotas.read();

        let mut result = HashMap::new();

        if let Some(tenant_quotas) = quotas.get(tenant_id) {
            let tenant_usage = usage.get(tenant_id);

            for (quota_type, limit) in &tenant_quotas.limits {
                let current = tenant_usage
                    .and_then(|u| u.get(quota_type))
                    .copied()
                    .unwrap_or(0);

                result.insert(*quota_type, QuotaUsage::new(*quota_type, current, limit.limit));
            }
        }

        result
    }

    /// Check if a quota would be exceeded
    pub fn check_quota(&self, tenant_id: &str, quota_type: QuotaType, amount: u64) -> Result<()> {
        let usage = self.usage.read();
        let quotas = self.quotas.read();

        let tenant_quotas = quotas
            .get(tenant_id)
            .ok_or_else(|| TenantError::NotFound(tenant_id.to_string()))?;

        let limit = tenant_quotas
            .limits
            .get(&quota_type)
            .ok_or_else(|| TenantError::InvalidConfiguration(format!(
                "No limit configured for {:?}",
                quota_type
            )))?;

        let current = usage
            .get(tenant_id)
            .and_then(|u| u.get(&quota_type))
            .copied()
            .unwrap_or(0);

        let new_total = current + amount;

        if new_total > limit.limit && limit.hard_limit {
            return Err(TenantError::QuotaExceeded(format!(
                "{:?} quota exceeded: {} + {} > {}",
                quota_type, current, amount, limit.limit
            )));
        }

        if new_total as f64 / limit.limit as f64 >= limit.warning_threshold {
            warn!(
                tenant_id = %tenant_id,
                quota_type = ?quota_type,
                current = current,
                amount = amount,
                limit = limit.limit,
                "Quota warning threshold reached"
            );
        }

        Ok(())
    }

    /// Increment usage for a quota
    pub fn increment(&self, tenant_id: &str, quota_type: QuotaType, amount: u64) -> Result<u64> {
        self.check_quota(tenant_id, quota_type, amount)?;

        let mut usage = self.usage.write();
        let tenant_usage = usage.entry(tenant_id.to_string()).or_default();
        let current = tenant_usage.entry(quota_type).or_insert(0);
        *current += amount;

        debug!(
            tenant_id = %tenant_id,
            quota_type = ?quota_type,
            amount = amount,
            new_total = *current,
            "Incremented quota usage"
        );

        Ok(*current)
    }

    /// Decrement usage for a quota
    pub fn decrement(&self, tenant_id: &str, quota_type: QuotaType, amount: u64) -> u64 {
        let mut usage = self.usage.write();
        let tenant_usage = usage.entry(tenant_id.to_string()).or_default();
        let current = tenant_usage.entry(quota_type).or_insert(0);
        *current = current.saturating_sub(amount);

        debug!(
            tenant_id = %tenant_id,
            quota_type = ?quota_type,
            amount = amount,
            new_total = *current,
            "Decremented quota usage"
        );

        *current
    }

    /// Set usage directly
    pub fn set_usage(&self, tenant_id: &str, quota_type: QuotaType, amount: u64) {
        let mut usage = self.usage.write();
        let tenant_usage = usage.entry(tenant_id.to_string()).or_default();
        tenant_usage.insert(quota_type, amount);
    }

    /// Reset periodic quotas for a tenant
    pub fn reset_periodic(&self, tenant_id: &str) {
        let mut usage = self.usage.write();
        if let Some(tenant_usage) = usage.get_mut(tenant_id) {
            for quota_type in [QuotaType::ApiCalls, QuotaType::Tokens] {
                tenant_usage.insert(quota_type, 0);
            }
        }
        debug!(tenant_id = %tenant_id, "Reset periodic quotas");
    }

    /// Update tenant tier quotas
    pub fn update_tier(&self, tenant_id: &str, tier: TenantTier) {
        let quotas = TenantQuotas::for_tier(tier);
        self.quotas.write().insert(tenant_id.to_string(), quotas);
        debug!(tenant_id = %tenant_id, tier = ?tier, "Updated tenant tier quotas");
    }

    /// Set custom quota limit
    pub fn set_custom_limit(&self, tenant_id: &str, quota_type: QuotaType, limit: u64) {
        let mut quotas = self.quotas.write();
        if let Some(tenant_quotas) = quotas.get_mut(tenant_id) {
            tenant_quotas.set_limit(quota_type, QuotaLimit::new(limit));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tenant() -> Tenant {
        Tenant::new("Test", "test", "owner", TenantTier::Professional)
    }

    #[test]
    fn test_quota_type_periodic() {
        assert!(QuotaType::ApiCalls.is_periodic());
        assert!(QuotaType::Tokens.is_periodic());
        assert!(!QuotaType::Storage.is_periodic());
        assert!(!QuotaType::Users.is_periodic());
    }

    #[test]
    fn test_quota_usage() {
        let usage = QuotaUsage::new(QuotaType::ApiCalls, 800, 1000);

        assert_eq!(usage.current, 800);
        assert_eq!(usage.limit, 1000);
        assert_eq!(usage.percentage, 80.0);
        assert!(!usage.exceeded);
        assert!(usage.warning);
        assert_eq!(usage.remaining(), 200);
    }

    #[test]
    fn test_quota_exceeded() {
        let usage = QuotaUsage::new(QuotaType::ApiCalls, 1200, 1000);

        assert!(usage.exceeded);
        assert_eq!(usage.remaining(), 0);
    }

    #[test]
    fn test_tenant_quotas_for_tier() {
        let free_quotas = TenantQuotas::for_tier(TenantTier::Free);
        let enterprise_quotas = TenantQuotas::for_tier(TenantTier::Enterprise);

        assert!(free_quotas.get_limit(&QuotaType::ApiCalls).unwrap() < enterprise_quotas.get_limit(&QuotaType::ApiCalls).unwrap());
        assert!(free_quotas.get_limit(&QuotaType::Users).unwrap() < enterprise_quotas.get_limit(&QuotaType::Users).unwrap());
    }

    #[test]
    fn test_quota_manager_basic() {
        let manager = QuotaManager::new();
        let tenant = create_test_tenant();

        manager.register_tenant(&tenant);

        // Check initial usage
        let usage = manager.get_usage(&tenant.id, QuotaType::ApiCalls).unwrap();
        assert_eq!(usage.current, 0);

        // Increment usage
        manager.increment(&tenant.id, QuotaType::ApiCalls, 100).unwrap();
        let usage = manager.get_usage(&tenant.id, QuotaType::ApiCalls).unwrap();
        assert_eq!(usage.current, 100);

        // Decrement usage
        manager.decrement(&tenant.id, QuotaType::ApiCalls, 50);
        let usage = manager.get_usage(&tenant.id, QuotaType::ApiCalls).unwrap();
        assert_eq!(usage.current, 50);
    }

    #[test]
    fn test_quota_enforcement() {
        let manager = QuotaManager::new();
        let tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);

        manager.register_tenant(&tenant);

        // Set small limit for testing
        manager.set_custom_limit(&tenant.id, QuotaType::ApiCalls, 100);

        // Should succeed
        assert!(manager.check_quota(&tenant.id, QuotaType::ApiCalls, 50).is_ok());

        // Increment to near limit
        manager.set_usage(&tenant.id, QuotaType::ApiCalls, 90);

        // Should fail - would exceed
        assert!(manager.check_quota(&tenant.id, QuotaType::ApiCalls, 20).is_err());
    }

    #[test]
    fn test_reset_periodic() {
        let manager = QuotaManager::new();
        let tenant = create_test_tenant();

        manager.register_tenant(&tenant);
        manager.increment(&tenant.id, QuotaType::ApiCalls, 500).unwrap();
        manager.increment(&tenant.id, QuotaType::Tokens, 10000).unwrap();
        manager.increment(&tenant.id, QuotaType::Users, 5).unwrap();

        manager.reset_periodic(&tenant.id);

        // Periodic quotas should be reset
        assert_eq!(manager.get_usage(&tenant.id, QuotaType::ApiCalls).unwrap().current, 0);
        assert_eq!(manager.get_usage(&tenant.id, QuotaType::Tokens).unwrap().current, 0);
        // Non-periodic should remain
        assert_eq!(manager.get_usage(&tenant.id, QuotaType::Users).unwrap().current, 5);
    }

    #[test]
    fn test_get_all_usage() {
        let manager = QuotaManager::new();
        let tenant = create_test_tenant();

        manager.register_tenant(&tenant);
        manager.increment(&tenant.id, QuotaType::ApiCalls, 100).unwrap();
        manager.increment(&tenant.id, QuotaType::Users, 3).unwrap();

        let all_usage = manager.get_all_usage(&tenant.id);

        assert!(all_usage.contains_key(&QuotaType::ApiCalls));
        assert!(all_usage.contains_key(&QuotaType::Users));
        assert!(all_usage.contains_key(&QuotaType::Storage));
    }
}
