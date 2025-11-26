//! Tenant onboarding automation
//!
//! Provides automated tenant setup and self-service administration.

use crate::{
    billing::{BillingService, Subscription},
    isolation::IsolationManager,
    quota::QuotaManager,
    metering::MeteringService,
    Result, Tenant, TenantConfig, TenantError, TenantInvitation, TenantMember, TenantRole,
    TenantStatus, TenantTier,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Onboarding step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStepStatus {
    /// Step is pending
    Pending,
    /// Step is in progress
    InProgress,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
}

/// Onboarding step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Status
    pub status: OnboardingStepStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl OnboardingStep {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            status: OnboardingStepStatus::Pending,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn start(&mut self) {
        self.status = OnboardingStepStatus::InProgress;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = OnboardingStepStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: &str) {
        self.status = OnboardingStepStatus::Failed;
        self.error = Some(error.to_string());
        self.completed_at = Some(Utc::now());
    }

    pub fn skip(&mut self) {
        self.status = OnboardingStepStatus::Skipped;
        self.completed_at = Some(Utc::now());
    }
}

/// Onboarding progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingProgress {
    /// Tenant ID
    pub tenant_id: String,
    /// Steps
    pub steps: Vec<OnboardingStep>,
    /// Overall status
    pub status: OnboardingStepStatus,
    /// Started timestamp
    pub started_at: DateTime<Utc>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl OnboardingProgress {
    pub fn new(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            steps: vec![
                OnboardingStep::new("create_tenant", "Create Tenant", "Creating tenant record"),
                OnboardingStep::new("setup_isolation", "Setup Isolation", "Setting up database schema and vector namespace"),
                OnboardingStep::new("configure_quotas", "Configure Quotas", "Setting up resource quotas"),
                OnboardingStep::new("setup_billing", "Setup Billing", "Creating billing account and subscription"),
                OnboardingStep::new("initialize_metering", "Initialize Metering", "Setting up usage metering"),
                OnboardingStep::new("send_welcome", "Send Welcome", "Sending welcome notification"),
            ],
            status: OnboardingStepStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn get_step_mut(&mut self, id: &str) -> Option<&mut OnboardingStep> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    pub fn update_status(&mut self) {
        let all_completed = self
            .steps
            .iter()
            .all(|s| matches!(s.status, OnboardingStepStatus::Completed | OnboardingStepStatus::Skipped));

        let any_failed = self
            .steps
            .iter()
            .any(|s| s.status == OnboardingStepStatus::Failed);

        let any_in_progress = self
            .steps
            .iter()
            .any(|s| s.status == OnboardingStepStatus::InProgress);

        self.status = if any_failed {
            OnboardingStepStatus::Failed
        } else if all_completed {
            self.completed_at = Some(Utc::now());
            OnboardingStepStatus::Completed
        } else if any_in_progress {
            OnboardingStepStatus::InProgress
        } else {
            OnboardingStepStatus::Pending
        };
    }

    pub fn completion_percentage(&self) -> f64 {
        let completed = self
            .steps
            .iter()
            .filter(|s| matches!(s.status, OnboardingStepStatus::Completed | OnboardingStepStatus::Skipped))
            .count();
        (completed as f64 / self.steps.len() as f64) * 100.0
    }
}

/// Tenant registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRegistration {
    /// Company/Organization name
    pub name: String,
    /// URL-friendly slug
    pub slug: Option<String>,
    /// Owner's user ID
    pub owner_id: String,
    /// Owner's email
    pub owner_email: String,
    /// Requested tier
    pub tier: TenantTier,
    /// Start as trial
    pub start_trial: bool,
    /// Trial duration in days
    pub trial_days: Option<u32>,
    /// Custom configuration
    pub config: Option<TenantConfig>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TenantRegistration {
    pub fn new(name: &str, owner_id: &str, owner_email: &str) -> Self {
        Self {
            name: name.to_string(),
            slug: None,
            owner_id: owner_id.to_string(),
            owner_email: owner_email.to_string(),
            tier: TenantTier::Free,
            start_trial: false,
            trial_days: None,
            config: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_tier(mut self, tier: TenantTier) -> Self {
        self.tier = tier;
        self
    }

    pub fn with_trial(mut self, days: u32) -> Self {
        self.start_trial = true;
        self.trial_days = Some(days);
        self
    }

    /// Generate slug from name if not provided
    pub fn effective_slug(&self) -> String {
        self.slug.clone().unwrap_or_else(|| {
            self.name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        })
    }
}

/// Onboarding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingResult {
    /// Created tenant
    pub tenant: Tenant,
    /// Onboarding progress
    pub progress: OnboardingProgress,
    /// Subscription (if billing enabled)
    pub subscription: Option<Subscription>,
}

/// Tenant repository trait
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// Create a tenant
    async fn create(&self, tenant: &Tenant) -> Result<()>;

    /// Get tenant by ID
    async fn get(&self, id: &str) -> Result<Option<Tenant>>;

    /// Get tenant by slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;

    /// Update tenant
    async fn update(&self, tenant: &Tenant) -> Result<()>;

    /// Delete tenant
    async fn delete(&self, id: &str) -> Result<()>;

    /// List tenants
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>>;

    /// Add member to tenant
    async fn add_member(&self, member: &TenantMember) -> Result<()>;

    /// Remove member from tenant
    async fn remove_member(&self, tenant_id: &str, user_id: &str) -> Result<()>;

    /// Get tenant members
    async fn get_members(&self, tenant_id: &str) -> Result<Vec<TenantMember>>;

    /// Create invitation
    async fn create_invitation(&self, invitation: &TenantInvitation) -> Result<()>;

    /// Get invitation by token
    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<TenantInvitation>>;

    /// Update invitation
    async fn update_invitation(&self, invitation: &TenantInvitation) -> Result<()>;
}

/// In-memory tenant repository for testing
#[derive(Debug, Default)]
pub struct InMemoryTenantRepository {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
    members: Arc<RwLock<HashMap<String, Vec<TenantMember>>>>,
    invitations: Arc<RwLock<HashMap<String, TenantInvitation>>>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TenantRepository for InMemoryTenantRepository {
    async fn create(&self, tenant: &Tenant) -> Result<()> {
        let mut tenants = self.tenants.write();
        if tenants.contains_key(&tenant.id) {
            return Err(TenantError::AlreadyExists(tenant.id.clone()));
        }
        tenants.insert(tenant.id.clone(), tenant.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Tenant>> {
        Ok(self.tenants.read().get(id).cloned())
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        Ok(self.tenants.read().values().find(|t| t.slug == slug).cloned())
    }

    async fn update(&self, tenant: &Tenant) -> Result<()> {
        let mut tenants = self.tenants.write();
        if !tenants.contains_key(&tenant.id) {
            return Err(TenantError::NotFound(tenant.id.clone()));
        }
        tenants.insert(tenant.id.clone(), tenant.clone());
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        if self.tenants.write().remove(id).is_none() {
            return Err(TenantError::NotFound(id.to_string()));
        }
        self.members.write().remove(id);
        Ok(())
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        Ok(self
            .tenants
            .read()
            .values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn add_member(&self, member: &TenantMember) -> Result<()> {
        self.members
            .write()
            .entry(member.tenant_id.clone())
            .or_default()
            .push(member.clone());
        Ok(())
    }

    async fn remove_member(&self, tenant_id: &str, user_id: &str) -> Result<()> {
        let mut members = self.members.write();
        if let Some(tenant_members) = members.get_mut(tenant_id) {
            tenant_members.retain(|m| m.user_id != user_id);
        }
        Ok(())
    }

    async fn get_members(&self, tenant_id: &str) -> Result<Vec<TenantMember>> {
        Ok(self
            .members
            .read()
            .get(tenant_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn create_invitation(&self, invitation: &TenantInvitation) -> Result<()> {
        self.invitations
            .write()
            .insert(invitation.token.clone(), invitation.clone());
        Ok(())
    }

    async fn get_invitation_by_token(&self, token: &str) -> Result<Option<TenantInvitation>> {
        Ok(self.invitations.read().get(token).cloned())
    }

    async fn update_invitation(&self, invitation: &TenantInvitation) -> Result<()> {
        self.invitations
            .write()
            .insert(invitation.token.clone(), invitation.clone());
        Ok(())
    }
}

/// Onboarding service for automating tenant setup
pub struct OnboardingService {
    repository: Arc<dyn TenantRepository>,
    isolation: Arc<IsolationManager>,
    quotas: Arc<QuotaManager>,
    billing: Arc<BillingService>,
    metering: Arc<MeteringService>,
    progress: Arc<RwLock<HashMap<String, OnboardingProgress>>>,
}

impl OnboardingService {
    pub fn new(
        repository: Arc<dyn TenantRepository>,
        isolation: Arc<IsolationManager>,
        quotas: Arc<QuotaManager>,
        billing: Arc<BillingService>,
        metering: Arc<MeteringService>,
    ) -> Self {
        Self {
            repository,
            isolation,
            quotas,
            billing,
            metering,
            progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create in-memory onboarding service for testing
    pub fn in_memory() -> Self {
        Self::new(
            Arc::new(InMemoryTenantRepository::new()),
            Arc::new(IsolationManager::in_memory()),
            Arc::new(QuotaManager::new()),
            Arc::new(BillingService::in_memory()),
            Arc::new(MeteringService::default()),
        )
    }

    /// Register a new tenant
    pub async fn register(&self, registration: TenantRegistration) -> Result<OnboardingResult> {
        let slug = registration.effective_slug();

        // Check if slug is already taken
        if self.repository.get_by_slug(&slug).await?.is_some() {
            return Err(TenantError::AlreadyExists(format!(
                "Tenant with slug '{}' already exists",
                slug
            )));
        }

        // Create tenant
        let mut tenant = if registration.start_trial {
            let trial_days = registration.trial_days.unwrap_or(14);
            Tenant::new_trial(&registration.name, &slug, &registration.owner_id, trial_days)
        } else {
            Tenant::new(&registration.name, &slug, &registration.owner_id, registration.tier)
        };

        if let Some(config) = registration.config {
            tenant.config = config;
        }
        tenant.metadata = registration.metadata;

        // Initialize progress tracking
        let mut progress = OnboardingProgress::new(&tenant.id);
        self.progress.write().insert(tenant.id.clone(), progress.clone());

        info!(
            tenant_id = %tenant.id,
            name = %tenant.name,
            slug = %slug,
            tier = ?tenant.tier,
            "Starting tenant onboarding"
        );

        // Step 1: Create tenant
        self.run_step(&mut progress, "create_tenant", async {
            self.repository.create(&tenant).await
        })
        .await?;

        // Step 2: Setup isolation
        self.run_step(&mut progress, "setup_isolation", async {
            self.isolation.initialize_tenant(&tenant).await
        })
        .await?;

        // Step 3: Configure quotas
        self.run_step(&mut progress, "configure_quotas", async {
            self.quotas.register_tenant(&tenant);
            Ok(())
        })
        .await?;

        // Step 4: Setup billing
        let subscription = self
            .run_step(&mut progress, "setup_billing", async {
                self.billing.setup_tenant(&tenant).await
            })
            .await
            .ok();

        // Step 5: Initialize metering
        self.run_step(&mut progress, "initialize_metering", async {
            // Metering is already initialized via the service
            Ok(())
        })
        .await?;

        // Step 6: Send welcome (placeholder)
        self.run_step(&mut progress, "send_welcome", async {
            debug!(tenant_id = %tenant.id, "Would send welcome notification here");
            Ok(())
        })
        .await?;

        // Add owner as member
        let owner_member = TenantMember {
            tenant_id: tenant.id.clone(),
            user_id: tenant.owner_id.clone(),
            role: TenantRole::Owner,
            joined_at: Utc::now(),
            invited_by: None,
        };
        self.repository.add_member(&owner_member).await?;

        // Activate tenant
        tenant.activate();
        self.repository.update(&tenant).await?;

        progress.update_status();
        self.progress.write().insert(tenant.id.clone(), progress.clone());

        info!(
            tenant_id = %tenant.id,
            completion = progress.completion_percentage(),
            "Tenant onboarding completed"
        );

        Ok(OnboardingResult {
            tenant,
            progress,
            subscription,
        })
    }

    /// Run an onboarding step
    async fn run_step<F, T>(&self, progress: &mut OnboardingProgress, step_id: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        if let Some(step) = progress.get_step_mut(step_id) {
            step.start();
        }
        self.progress.write().insert(progress.tenant_id.clone(), progress.clone());

        match f.await {
            Ok(result) => {
                if let Some(step) = progress.get_step_mut(step_id) {
                    step.complete();
                }
                self.progress.write().insert(progress.tenant_id.clone(), progress.clone());
                Ok(result)
            }
            Err(e) => {
                if let Some(step) = progress.get_step_mut(step_id) {
                    step.fail(&e.to_string());
                }
                self.progress.write().insert(progress.tenant_id.clone(), progress.clone());
                warn!(step = step_id, error = %e, "Onboarding step failed");
                Err(e)
            }
        }
    }

    /// Get onboarding progress
    pub fn get_progress(&self, tenant_id: &str) -> Option<OnboardingProgress> {
        self.progress.read().get(tenant_id).cloned()
    }

    /// Get tenant
    pub async fn get_tenant(&self, tenant_id: &str) -> Result<Option<Tenant>> {
        self.repository.get(tenant_id).await
    }

    /// Get tenant by slug
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        self.repository.get_by_slug(slug).await
    }

    /// Update tenant
    pub async fn update_tenant(&self, tenant: &Tenant) -> Result<()> {
        self.repository.update(tenant).await
    }

    /// Delete tenant (with cleanup)
    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<()> {
        let tenant = self
            .repository
            .get(tenant_id)
            .await?
            .ok_or_else(|| TenantError::NotFound(tenant_id.to_string()))?;

        warn!(tenant_id = %tenant_id, "Deleting tenant (destructive operation)");

        // Clean up isolation
        self.isolation.cleanup_tenant(&tenant).await?;

        // Remove quotas
        self.quotas.remove_tenant(tenant_id);

        // Delete from repository
        self.repository.delete(tenant_id).await?;

        info!(tenant_id = %tenant_id, "Tenant deleted");
        Ok(())
    }

    /// Invite user to tenant
    pub async fn invite_user(
        &self,
        tenant_id: &str,
        email: &str,
        role: TenantRole,
        invited_by: &str,
    ) -> Result<TenantInvitation> {
        let invitation = TenantInvitation::new(tenant_id, email, role, invited_by);
        self.repository.create_invitation(&invitation).await?;

        info!(
            tenant_id = %tenant_id,
            email = %email,
            role = ?role,
            "Created tenant invitation"
        );

        Ok(invitation)
    }

    /// Accept invitation
    pub async fn accept_invitation(&self, token: &str, user_id: &str) -> Result<TenantMember> {
        let mut invitation = self
            .repository
            .get_invitation_by_token(token)
            .await?
            .ok_or_else(|| TenantError::NotFound("Invitation not found".to_string()))?;

        if !invitation.is_valid() {
            return Err(TenantError::InvalidConfiguration(
                "Invitation is no longer valid".to_string(),
            ));
        }

        invitation.accept();
        self.repository.update_invitation(&invitation).await?;

        let member = TenantMember {
            tenant_id: invitation.tenant_id.clone(),
            user_id: user_id.to_string(),
            role: invitation.role,
            joined_at: Utc::now(),
            invited_by: Some(invitation.invited_by.clone()),
        };
        self.repository.add_member(&member).await?;

        info!(
            tenant_id = %invitation.tenant_id,
            user_id = %user_id,
            role = ?invitation.role,
            "User accepted invitation and joined tenant"
        );

        Ok(member)
    }

    /// List tenant members
    pub async fn list_members(&self, tenant_id: &str) -> Result<Vec<TenantMember>> {
        self.repository.get_members(tenant_id).await
    }

    /// Remove member from tenant
    pub async fn remove_member(&self, tenant_id: &str, user_id: &str) -> Result<()> {
        self.repository.remove_member(tenant_id, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_registration() {
        let service = OnboardingService::in_memory();

        let registration = TenantRegistration::new("Test Company", "owner-123", "owner@test.com")
            .with_tier(TenantTier::Professional);

        let result = service.register(registration).await.unwrap();

        assert_eq!(result.tenant.name, "Test Company");
        assert_eq!(result.tenant.status, TenantStatus::Active);
        assert_eq!(result.progress.status, OnboardingStepStatus::Completed);
        assert_eq!(result.progress.completion_percentage(), 100.0);
    }

    #[tokio::test]
    async fn test_trial_registration() {
        let service = OnboardingService::in_memory();

        let registration = TenantRegistration::new("Trial Co", "owner-456", "owner@trial.com")
            .with_trial(14);

        let result = service.register(registration).await.unwrap();

        assert_eq!(result.tenant.status, TenantStatus::Active); // Activated after onboarding
        assert!(result.tenant.trial_ends_at.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_slug_rejected() {
        let service = OnboardingService::in_memory();

        let reg1 = TenantRegistration::new("Test Company", "owner-1", "owner1@test.com");
        service.register(reg1).await.unwrap();

        let reg2 = TenantRegistration::new("Test Company", "owner-2", "owner2@test.com");
        let result = service.register(reg2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invitation_flow() {
        let service = OnboardingService::in_memory();

        // Create tenant
        let registration = TenantRegistration::new("Test Co", "owner-123", "owner@test.com");
        let result = service.register(registration).await.unwrap();

        // Invite user
        let invitation = service
            .invite_user(&result.tenant.id, "user@test.com", TenantRole::Member, "owner-123")
            .await
            .unwrap();

        assert!(invitation.is_valid());

        // Accept invitation
        let member = service
            .accept_invitation(&invitation.token, "user-456")
            .await
            .unwrap();

        assert_eq!(member.role, TenantRole::Member);
        assert_eq!(member.tenant_id, result.tenant.id);

        // List members
        let members = service.list_members(&result.tenant.id).await.unwrap();
        assert_eq!(members.len(), 2); // Owner + invited user
    }

    #[tokio::test]
    async fn test_tenant_deletion() {
        let service = OnboardingService::in_memory();

        let registration = TenantRegistration::new("Test Co", "owner-123", "owner@test.com");
        let result = service.register(registration).await.unwrap();

        // Delete tenant
        service.delete_tenant(&result.tenant.id).await.unwrap();

        // Verify deletion
        let tenant = service.get_tenant(&result.tenant.id).await.unwrap();
        assert!(tenant.is_none());
    }

    #[test]
    fn test_onboarding_progress() {
        let mut progress = OnboardingProgress::new("tenant-1");

        assert_eq!(progress.status, OnboardingStepStatus::Pending);
        assert_eq!(progress.completion_percentage(), 0.0);

        // Complete first step
        progress.get_step_mut("create_tenant").unwrap().complete();
        progress.update_status();

        assert!(progress.completion_percentage() > 0.0);
        assert!(progress.completion_percentage() < 100.0);
    }
}
