//! Sandbox lifecycle management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{E2BConfig, E2BError, Result, SandboxTemplate};

/// Represents an E2B sandbox instance
#[derive(Debug, Clone)]
pub struct Sandbox {
    /// Unique sandbox identifier
    pub id: String,

    /// Sandbox template used
    pub template: SandboxTemplate,

    /// Current status
    pub status: SandboxStatus,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Environment variables
    pub env_vars: HashMap<String, String>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Sandbox {
    /// Create a new sandbox representation
    pub fn new(template: SandboxTemplate) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            template,
            status: SandboxStatus::Pending,
            created_at: now,
            last_activity: now,
            env_vars: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Check if the sandbox is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, SandboxStatus::Running)
    }

    /// Check if the sandbox can execute code
    pub fn can_execute(&self) -> bool {
        matches!(self.status, SandboxStatus::Running | SandboxStatus::Paused)
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Set an environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env_vars.insert(key.into(), value.into());
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Sandbox status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    /// Sandbox is being created
    Pending,
    /// Sandbox is running and ready
    Running,
    /// Sandbox is paused
    Paused,
    /// Sandbox is stopping
    Stopping,
    /// Sandbox has stopped
    Stopped,
    /// Sandbox encountered an error
    Error,
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxStatus::Pending => write!(f, "pending"),
            SandboxStatus::Running => write!(f, "running"),
            SandboxStatus::Paused => write!(f, "paused"),
            SandboxStatus::Stopping => write!(f, "stopping"),
            SandboxStatus::Stopped => write!(f, "stopped"),
            SandboxStatus::Error => write!(f, "error"),
        }
    }
}

/// Manages sandbox lifecycle
pub struct SandboxManager {
    config: E2BConfig,
    sandboxes: Arc<RwLock<HashMap<String, Sandbox>>>,
}

impl SandboxManager {
    /// Create a new sandbox manager
    pub fn new(config: E2BConfig) -> Self {
        Self {
            config,
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new sandbox
    pub async fn create(&self, template: Option<SandboxTemplate>) -> Result<Sandbox> {
        let template = template.unwrap_or(self.config.default_template);

        // Check sandbox limit
        let sandboxes = self.sandboxes.read().await;
        let active_count = sandboxes.values()
            .filter(|s| s.is_active())
            .count();

        if active_count >= self.config.max_sandboxes {
            return Err(E2BError::ResourceLimit(format!(
                "Maximum sandbox limit ({}) reached",
                self.config.max_sandboxes
            )));
        }
        drop(sandboxes);

        info!("Creating new {} sandbox", template);

        // Create sandbox representation
        let mut sandbox = Sandbox::new(template);

        // Add configured environment variables
        for (key, value) in &self.config.env_vars {
            sandbox.set_env(key.clone(), value.clone());
        }

        // Simulate sandbox creation (in production, this would call E2B API)
        sandbox.status = SandboxStatus::Running;

        // Store sandbox
        let sandbox_id = sandbox.id.clone();
        self.sandboxes.write().await.insert(sandbox_id.clone(), sandbox.clone());

        info!("Created sandbox {} with template {}", sandbox_id, template);

        Ok(sandbox)
    }

    /// Get a sandbox by ID
    pub async fn get(&self, sandbox_id: &str) -> Result<Sandbox> {
        self.sandboxes.read().await
            .get(sandbox_id)
            .cloned()
            .ok_or_else(|| E2BError::sandbox(format!("Sandbox {} not found", sandbox_id)))
    }

    /// List all sandboxes
    pub async fn list(&self) -> Vec<Sandbox> {
        self.sandboxes.read().await.values().cloned().collect()
    }

    /// Pause a sandbox
    pub async fn pause(&self, sandbox_id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write().await;
        let sandbox = sandboxes.get_mut(sandbox_id)
            .ok_or_else(|| E2BError::sandbox(format!("Sandbox {} not found", sandbox_id)))?;

        if sandbox.status != SandboxStatus::Running {
            return Err(E2BError::sandbox(format!(
                "Cannot pause sandbox in {} state",
                sandbox.status
            )));
        }

        debug!("Pausing sandbox {}", sandbox_id);
        sandbox.status = SandboxStatus::Paused;
        sandbox.touch();

        Ok(())
    }

    /// Resume a paused sandbox
    pub async fn resume(&self, sandbox_id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write().await;
        let sandbox = sandboxes.get_mut(sandbox_id)
            .ok_or_else(|| E2BError::sandbox(format!("Sandbox {} not found", sandbox_id)))?;

        if sandbox.status != SandboxStatus::Paused {
            return Err(E2BError::sandbox(format!(
                "Cannot resume sandbox in {} state",
                sandbox.status
            )));
        }

        debug!("Resuming sandbox {}", sandbox_id);
        sandbox.status = SandboxStatus::Running;
        sandbox.touch();

        Ok(())
    }

    /// Destroy a sandbox
    pub async fn destroy(&self, sandbox_id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write().await;
        let sandbox = sandboxes.get_mut(sandbox_id)
            .ok_or_else(|| E2BError::sandbox(format!("Sandbox {} not found", sandbox_id)))?;

        info!("Destroying sandbox {}", sandbox_id);
        sandbox.status = SandboxStatus::Stopping;

        // In production, this would call E2B API to destroy the sandbox
        sandbox.status = SandboxStatus::Stopped;

        // Remove from active sandboxes
        sandboxes.remove(sandbox_id);

        Ok(())
    }

    /// Clean up inactive sandboxes
    pub async fn cleanup_inactive(&self, max_idle_duration: std::time::Duration) -> usize {
        let now = Utc::now();
        let mut sandboxes = self.sandboxes.write().await;
        let mut removed = 0;

        let to_remove: Vec<String> = sandboxes.iter()
            .filter(|(_, sandbox)| {
                let idle_duration = now.signed_duration_since(sandbox.last_activity);
                idle_duration.num_seconds() > max_idle_duration.as_secs() as i64
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            warn!("Removing inactive sandbox {}", id);
            sandboxes.remove(&id);
            removed += 1;
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = Sandbox::new(SandboxTemplate::Python);
        assert!(!sandbox.id.is_empty());
        assert_eq!(sandbox.template, SandboxTemplate::Python);
        assert_eq!(sandbox.status, SandboxStatus::Pending);
    }

    #[test]
    fn test_sandbox_status_display() {
        assert_eq!(SandboxStatus::Running.to_string(), "running");
        assert_eq!(SandboxStatus::Stopped.to_string(), "stopped");
    }

    #[tokio::test]
    async fn test_sandbox_manager() {
        let config = E2BConfig::with_api_key("test-key")
            .max_sandboxes(2);
        let manager = SandboxManager::new(config);

        // Create first sandbox
        let sandbox1 = manager.create(Some(SandboxTemplate::Python)).await.unwrap();
        assert_eq!(sandbox1.status, SandboxStatus::Running);

        // Create second sandbox
        let sandbox2 = manager.create(Some(SandboxTemplate::NodeJs)).await.unwrap();
        assert_eq!(sandbox2.status, SandboxStatus::Running);

        // Third sandbox should fail (limit reached)
        let result = manager.create(Some(SandboxTemplate::Go)).await;
        assert!(result.is_err());

        // Destroy one sandbox
        manager.destroy(&sandbox1.id).await.unwrap();

        // Now we can create another
        let sandbox3 = manager.create(Some(SandboxTemplate::Go)).await.unwrap();
        assert_eq!(sandbox3.status, SandboxStatus::Running);
    }
}
