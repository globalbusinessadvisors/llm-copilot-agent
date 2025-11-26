//! Core tenant management
//!
//! Provides tenant data models and management functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Tenant status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    /// Tenant is active and operational
    Active,
    /// Tenant is pending activation
    Pending,
    /// Tenant is suspended (e.g., payment issues)
    Suspended,
    /// Tenant is disabled by admin
    Disabled,
    /// Tenant is marked for deletion
    Deleted,
    /// Tenant is in trial period
    Trial,
}

impl Default for TenantStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl TenantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Suspended => "suspended",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
            Self::Trial => "trial",
        }
    }

    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::Trial)
    }
}

/// Tenant tier/plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantTier {
    /// Free tier with limited resources
    Free,
    /// Starter tier for small teams
    Starter,
    /// Professional tier
    Professional,
    /// Business tier
    Business,
    /// Enterprise tier with custom limits
    Enterprise,
    /// Custom tier with negotiated limits
    Custom,
}

impl Default for TenantTier {
    fn default() -> Self {
        Self::Free
    }
}

impl TenantTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Starter => "starter",
            Self::Professional => "professional",
            Self::Business => "business",
            Self::Enterprise => "enterprise",
            Self::Custom => "custom",
        }
    }

    /// Get default monthly API call limit for this tier
    pub fn default_api_calls_limit(&self) -> u64 {
        match self {
            Self::Free => 1_000,
            Self::Starter => 10_000,
            Self::Professional => 100_000,
            Self::Business => 1_000_000,
            Self::Enterprise => 10_000_000,
            Self::Custom => u64::MAX,
        }
    }

    /// Get default monthly token limit for this tier
    pub fn default_token_limit(&self) -> u64 {
        match self {
            Self::Free => 100_000,
            Self::Starter => 1_000_000,
            Self::Professional => 10_000_000,
            Self::Business => 100_000_000,
            Self::Enterprise => 1_000_000_000,
            Self::Custom => u64::MAX,
        }
    }

    /// Get default storage limit in bytes
    pub fn default_storage_limit(&self) -> u64 {
        match self {
            Self::Free => 100 * 1024 * 1024,          // 100 MB
            Self::Starter => 1024 * 1024 * 1024,      // 1 GB
            Self::Professional => 10 * 1024 * 1024 * 1024, // 10 GB
            Self::Business => 100 * 1024 * 1024 * 1024,    // 100 GB
            Self::Enterprise => 1024 * 1024 * 1024 * 1024, // 1 TB
            Self::Custom => u64::MAX,
        }
    }

    /// Get default max users for this tier
    pub fn default_max_users(&self) -> u32 {
        match self {
            Self::Free => 1,
            Self::Starter => 5,
            Self::Professional => 25,
            Self::Business => 100,
            Self::Enterprise => 500,
            Self::Custom => u32::MAX,
        }
    }

    /// Get default max concurrent workflows
    pub fn default_max_concurrent_workflows(&self) -> u32 {
        match self {
            Self::Free => 1,
            Self::Starter => 3,
            Self::Professional => 10,
            Self::Business => 50,
            Self::Enterprise => 200,
            Self::Custom => u32::MAX,
        }
    }
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Custom branding
    pub branding: Option<TenantBranding>,
    /// Custom domain
    pub custom_domain: Option<String>,
    /// Allowed IP ranges
    pub allowed_ips: Vec<String>,
    /// Feature flags
    pub features: HashMap<String, bool>,
    /// Custom settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Webhook URL for events
    pub webhook_url: Option<String>,
    /// Webhook secret for signing
    pub webhook_secret: Option<String>,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            branding: None,
            custom_domain: None,
            allowed_ips: Vec::new(),
            features: HashMap::new(),
            settings: HashMap::new(),
            webhook_url: None,
            webhook_secret: None,
        }
    }
}

/// Tenant branding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBranding {
    /// Company name
    pub company_name: Option<String>,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary color (hex)
    pub primary_color: Option<String>,
    /// Secondary color (hex)
    pub secondary_color: Option<String>,
    /// Custom CSS
    pub custom_css: Option<String>,
}

/// Tenant entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant ID
    pub id: String,
    /// Tenant name
    pub name: String,
    /// Tenant slug (URL-friendly identifier)
    pub slug: String,
    /// Tenant status
    pub status: TenantStatus,
    /// Tenant tier/plan
    pub tier: TenantTier,
    /// Owner user ID
    pub owner_id: String,
    /// Tenant configuration
    pub config: TenantConfig,
    /// Database schema name (for isolation)
    pub schema_name: String,
    /// Vector store namespace
    pub vector_namespace: String,
    /// Trial end date (if on trial)
    pub trial_ends_at: Option<DateTime<Utc>>,
    /// Subscription end date
    pub subscription_ends_at: Option<DateTime<Utc>>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Create a new tenant
    pub fn new(name: &str, slug: &str, owner_id: &str, tier: TenantTier) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        Self {
            id: id.clone(),
            name: name.to_string(),
            slug: slug.to_string(),
            status: TenantStatus::Pending,
            tier,
            owner_id: owner_id.to_string(),
            config: TenantConfig::default(),
            schema_name: format!("tenant_{}", slug.replace('-', "_")),
            vector_namespace: format!("tenant:{}", id),
            trial_ends_at: None,
            subscription_ends_at: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a trial tenant
    pub fn new_trial(name: &str, slug: &str, owner_id: &str, trial_days: u32) -> Self {
        let mut tenant = Self::new(name, slug, owner_id, TenantTier::Free);
        tenant.status = TenantStatus::Trial;
        tenant.trial_ends_at = Some(Utc::now() + chrono::Duration::days(trial_days as i64));
        tenant
    }

    /// Check if tenant is operational
    pub fn is_operational(&self) -> bool {
        if !self.status.is_operational() {
            return false;
        }

        // Check trial expiration
        if self.status == TenantStatus::Trial {
            if let Some(trial_ends) = self.trial_ends_at {
                if Utc::now() > trial_ends {
                    return false;
                }
            }
        }

        // Check subscription expiration
        if let Some(subscription_ends) = self.subscription_ends_at {
            if Utc::now() > subscription_ends {
                return false;
            }
        }

        true
    }

    /// Activate the tenant
    pub fn activate(&mut self) {
        self.status = TenantStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Suspend the tenant
    pub fn suspend(&mut self) {
        self.status = TenantStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Disable the tenant
    pub fn disable(&mut self) {
        self.status = TenantStatus::Disabled;
        self.updated_at = Utc::now();
    }

    /// Mark tenant for deletion
    pub fn mark_for_deletion(&mut self) {
        self.status = TenantStatus::Deleted;
        self.updated_at = Utc::now();
    }

    /// Upgrade tenant tier
    pub fn upgrade_tier(&mut self, new_tier: TenantTier) {
        self.tier = new_tier;
        self.updated_at = Utc::now();
    }

    /// Set subscription end date
    pub fn set_subscription(&mut self, ends_at: DateTime<Utc>) {
        self.subscription_ends_at = Some(ends_at);
        self.updated_at = Utc::now();
    }

    /// Check if feature is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.config.features.get(feature).copied().unwrap_or(false)
    }

    /// Enable a feature
    pub fn enable_feature(&mut self, feature: &str) {
        self.config.features.insert(feature.to_string(), true);
        self.updated_at = Utc::now();
    }

    /// Disable a feature
    pub fn disable_feature(&mut self, feature: &str) {
        self.config.features.insert(feature.to_string(), false);
        self.updated_at = Utc::now();
    }
}

/// Tenant member/user association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMember {
    /// Tenant ID
    pub tenant_id: String,
    /// User ID
    pub user_id: String,
    /// Role within the tenant
    pub role: TenantRole,
    /// Joined timestamp
    pub joined_at: DateTime<Utc>,
    /// Invited by user ID
    pub invited_by: Option<String>,
}

/// Role within a tenant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantRole {
    /// Owner with full access
    Owner,
    /// Admin with management access
    Admin,
    /// Member with standard access
    Member,
    /// Viewer with read-only access
    Viewer,
}

impl TenantRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
        }
    }

    /// Check if role can manage other users
    pub fn can_manage_users(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// Check if role can modify settings
    pub fn can_modify_settings(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// Check if role can delete tenant
    pub fn can_delete_tenant(&self) -> bool {
        matches!(self, Self::Owner)
    }
}

/// Tenant invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantInvitation {
    /// Unique invitation ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Invited email address
    pub email: String,
    /// Assigned role
    pub role: TenantRole,
    /// Invitation token
    pub token: String,
    /// Invited by user ID
    pub invited_by: String,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Accepted timestamp
    pub accepted_at: Option<DateTime<Utc>>,
}

impl TenantInvitation {
    /// Create a new invitation
    pub fn new(tenant_id: &str, email: &str, role: TenantRole, invited_by: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            email: email.to_string(),
            role,
            token: Uuid::new_v4().to_string(),
            invited_by: invited_by.to_string(),
            expires_at: now + chrono::Duration::days(7),
            created_at: now,
            accepted_at: None,
        }
    }

    /// Check if invitation is valid
    pub fn is_valid(&self) -> bool {
        self.accepted_at.is_none() && Utc::now() < self.expires_at
    }

    /// Accept the invitation
    pub fn accept(&mut self) {
        self.accepted_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Test Company", "test-company", "user-123", TenantTier::Professional);

        assert!(!tenant.id.is_empty());
        assert_eq!(tenant.name, "Test Company");
        assert_eq!(tenant.slug, "test-company");
        assert_eq!(tenant.status, TenantStatus::Pending);
        assert_eq!(tenant.tier, TenantTier::Professional);
        assert_eq!(tenant.schema_name, "tenant_test_company");
    }

    #[test]
    fn test_trial_tenant() {
        let tenant = Tenant::new_trial("Trial Co", "trial-co", "user-456", 14);

        assert_eq!(tenant.status, TenantStatus::Trial);
        assert!(tenant.trial_ends_at.is_some());
        assert!(tenant.is_operational());
    }

    #[test]
    fn test_tenant_activation() {
        let mut tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);
        assert_eq!(tenant.status, TenantStatus::Pending);

        tenant.activate();
        assert_eq!(tenant.status, TenantStatus::Active);
        assert!(tenant.is_operational());
    }

    #[test]
    fn test_tenant_suspension() {
        let mut tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);
        tenant.activate();
        tenant.suspend();

        assert_eq!(tenant.status, TenantStatus::Suspended);
        assert!(!tenant.is_operational());
    }

    #[test]
    fn test_tier_limits() {
        assert_eq!(TenantTier::Free.default_api_calls_limit(), 1_000);
        assert_eq!(TenantTier::Enterprise.default_api_calls_limit(), 10_000_000);
        assert_eq!(TenantTier::Free.default_max_users(), 1);
        assert_eq!(TenantTier::Business.default_max_users(), 100);
    }

    #[test]
    fn test_feature_flags() {
        let mut tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);

        assert!(!tenant.has_feature("advanced_analytics"));
        tenant.enable_feature("advanced_analytics");
        assert!(tenant.has_feature("advanced_analytics"));
        tenant.disable_feature("advanced_analytics");
        assert!(!tenant.has_feature("advanced_analytics"));
    }

    #[test]
    fn test_tenant_role_permissions() {
        assert!(TenantRole::Owner.can_manage_users());
        assert!(TenantRole::Admin.can_manage_users());
        assert!(!TenantRole::Member.can_manage_users());
        assert!(!TenantRole::Viewer.can_manage_users());

        assert!(TenantRole::Owner.can_delete_tenant());
        assert!(!TenantRole::Admin.can_delete_tenant());
    }

    #[test]
    fn test_invitation() {
        let invitation = TenantInvitation::new("tenant-1", "user@example.com", TenantRole::Member, "owner-1");

        assert!(invitation.is_valid());
        assert!(invitation.accepted_at.is_none());
    }
}
