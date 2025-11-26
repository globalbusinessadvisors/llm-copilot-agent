//! Workflow versioning and rollback
//!
//! Provides version control for workflow definitions with rollback capabilities.

use crate::{engine::WorkflowDefinition, Result, WorkflowError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Workflow version metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVersion {
    /// Version ID
    pub id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Version number (semantic versioning)
    pub version: String,
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
    /// Workflow definition at this version
    pub definition: WorkflowDefinition,
    /// Version description/changelog
    pub description: Option<String>,
    /// Author who created this version
    pub author: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Whether this is the active version
    pub is_active: bool,
    /// Whether this version is deprecated
    pub is_deprecated: bool,
    /// Parent version ID (for lineage tracking)
    pub parent_version_id: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl WorkflowVersion {
    /// Create a new workflow version
    pub fn new(workflow_id: &str, definition: WorkflowDefinition) -> Self {
        Self {
            id: format!(
                "wfv_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "")
            ),
            workflow_id: workflow_id.to_string(),
            version: "1.0.0".to_string(),
            major: 1,
            minor: 0,
            patch: 0,
            definition,
            description: None,
            author: None,
            created_at: Utc::now(),
            is_active: true,
            is_deprecated: false,
            parent_version_id: None,
            tags: Vec::new(),
        }
    }

    /// Create next major version
    pub fn next_major(&self, definition: WorkflowDefinition) -> Self {
        let mut version = Self::new(&self.workflow_id, definition);
        version.major = self.major + 1;
        version.minor = 0;
        version.patch = 0;
        version.version = format!("{}.{}.{}", version.major, version.minor, version.patch);
        version.parent_version_id = Some(self.id.clone());
        version
    }

    /// Create next minor version
    pub fn next_minor(&self, definition: WorkflowDefinition) -> Self {
        let mut version = Self::new(&self.workflow_id, definition);
        version.major = self.major;
        version.minor = self.minor + 1;
        version.patch = 0;
        version.version = format!("{}.{}.{}", version.major, version.minor, version.patch);
        version.parent_version_id = Some(self.id.clone());
        version
    }

    /// Create next patch version
    pub fn next_patch(&self, definition: WorkflowDefinition) -> Self {
        let mut version = Self::new(&self.workflow_id, definition);
        version.major = self.major;
        version.minor = self.minor;
        version.patch = self.patch + 1;
        version.version = format!("{}.{}.{}", version.major, version.minor, version.patch);
        version.parent_version_id = Some(self.id.clone());
        version
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Version comparison result
#[derive(Debug, Clone)]
pub struct VersionDiff {
    /// Old version
    pub old_version: String,
    /// New version
    pub new_version: String,
    /// Added steps
    pub steps_added: Vec<String>,
    /// Removed steps
    pub steps_removed: Vec<String>,
    /// Modified steps
    pub steps_modified: Vec<String>,
    /// Connection changes
    pub connection_changes: Vec<String>,
}

/// Version repository trait
#[async_trait]
pub trait VersionRepository: Send + Sync {
    /// Save a version
    async fn save(&self, version: &WorkflowVersion) -> Result<()>;

    /// Get version by ID
    async fn get(&self, version_id: &str) -> Result<Option<WorkflowVersion>>;

    /// Get active version for a workflow
    async fn get_active(&self, workflow_id: &str) -> Result<Option<WorkflowVersion>>;

    /// Get all versions for a workflow
    async fn list_versions(&self, workflow_id: &str) -> Result<Vec<WorkflowVersion>>;

    /// Get specific version by workflow ID and version string
    async fn get_by_version(
        &self,
        workflow_id: &str,
        version: &str,
    ) -> Result<Option<WorkflowVersion>>;

    /// Delete a version
    async fn delete(&self, version_id: &str) -> Result<()>;

    /// Update version
    async fn update(&self, version: &WorkflowVersion) -> Result<()>;
}

/// In-memory version repository
pub struct InMemoryVersionRepository {
    versions: RwLock<HashMap<String, WorkflowVersion>>,
    by_workflow: RwLock<HashMap<String, Vec<String>>>,
}

impl InMemoryVersionRepository {
    pub fn new() -> Self {
        Self {
            versions: RwLock::new(HashMap::new()),
            by_workflow: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryVersionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VersionRepository for InMemoryVersionRepository {
    async fn save(&self, version: &WorkflowVersion) -> Result<()> {
        let mut versions = self.versions.write().await;
        let mut by_workflow = self.by_workflow.write().await;

        versions.insert(version.id.clone(), version.clone());

        by_workflow
            .entry(version.workflow_id.clone())
            .or_default()
            .push(version.id.clone());

        Ok(())
    }

    async fn get(&self, version_id: &str) -> Result<Option<WorkflowVersion>> {
        let versions = self.versions.read().await;
        Ok(versions.get(version_id).cloned())
    }

    async fn get_active(&self, workflow_id: &str) -> Result<Option<WorkflowVersion>> {
        let versions = self.versions.read().await;
        let by_workflow = self.by_workflow.read().await;

        if let Some(version_ids) = by_workflow.get(workflow_id) {
            for version_id in version_ids {
                if let Some(version) = versions.get(version_id) {
                    if version.is_active {
                        return Ok(Some(version.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn list_versions(&self, workflow_id: &str) -> Result<Vec<WorkflowVersion>> {
        let versions = self.versions.read().await;
        let by_workflow = self.by_workflow.read().await;

        let mut result = Vec::new();
        if let Some(version_ids) = by_workflow.get(workflow_id) {
            for version_id in version_ids {
                if let Some(version) = versions.get(version_id) {
                    result.push(version.clone());
                }
            }
        }

        // Sort by version number descending
        result.sort_by(|a, b| {
            let a_v = (a.major, a.minor, a.patch);
            let b_v = (b.major, b.minor, b.patch);
            b_v.cmp(&a_v)
        });

        Ok(result)
    }

    async fn get_by_version(
        &self,
        workflow_id: &str,
        version: &str,
    ) -> Result<Option<WorkflowVersion>> {
        let versions = self.versions.read().await;
        let by_workflow = self.by_workflow.read().await;

        if let Some(version_ids) = by_workflow.get(workflow_id) {
            for version_id in version_ids {
                if let Some(v) = versions.get(version_id) {
                    if v.version == version {
                        return Ok(Some(v.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn delete(&self, version_id: &str) -> Result<()> {
        let mut versions = self.versions.write().await;
        let mut by_workflow = self.by_workflow.write().await;

        if let Some(version) = versions.remove(version_id) {
            if let Some(ids) = by_workflow.get_mut(&version.workflow_id) {
                ids.retain(|id| id != version_id);
            }
        }

        Ok(())
    }

    async fn update(&self, version: &WorkflowVersion) -> Result<()> {
        let mut versions = self.versions.write().await;
        if versions.contains_key(&version.id) {
            versions.insert(version.id.clone(), version.clone());
            Ok(())
        } else {
            Err(WorkflowError::NotFound(version.id.clone()))
        }
    }
}

/// Workflow version manager
pub struct VersionManager {
    repository: Arc<dyn VersionRepository>,
}

impl VersionManager {
    pub fn new(repository: Arc<dyn VersionRepository>) -> Self {
        Self { repository }
    }

    /// Create initial version for a workflow
    pub async fn create_initial(
        &self,
        workflow_id: &str,
        definition: WorkflowDefinition,
    ) -> Result<WorkflowVersion> {
        let version = WorkflowVersion::new(workflow_id, definition);
        self.repository.save(&version).await?;

        info!(
            workflow_id = %workflow_id,
            version = %version.version,
            "Created initial workflow version"
        );

        Ok(version)
    }

    /// Create a new version
    pub async fn create_version(
        &self,
        workflow_id: &str,
        definition: WorkflowDefinition,
        version_type: VersionBump,
    ) -> Result<WorkflowVersion> {
        // Get current active version
        let current = self.repository.get_active(workflow_id).await?;

        let new_version = if let Some(mut current) = current {
            // Deactivate current version
            current.is_active = false;
            self.repository.update(&current).await?;

            match version_type {
                VersionBump::Major => current.next_major(definition),
                VersionBump::Minor => current.next_minor(definition),
                VersionBump::Patch => current.next_patch(definition),
            }
        } else {
            WorkflowVersion::new(workflow_id, definition)
        };

        self.repository.save(&new_version).await?;

        info!(
            workflow_id = %workflow_id,
            version = %new_version.version,
            bump = ?version_type,
            "Created new workflow version"
        );

        Ok(new_version)
    }

    /// Activate a specific version
    pub async fn activate(&self, workflow_id: &str, version_id: &str) -> Result<()> {
        // Deactivate current active version
        if let Some(mut current) = self.repository.get_active(workflow_id).await? {
            current.is_active = false;
            self.repository.update(&current).await?;
        }

        // Activate new version
        if let Some(mut version) = self.repository.get(version_id).await? {
            version.is_active = true;
            self.repository.update(&version).await?;

            info!(
                workflow_id = %workflow_id,
                version = %version.version,
                "Activated workflow version"
            );

            Ok(())
        } else {
            Err(WorkflowError::NotFound(version_id.to_string()))
        }
    }

    /// Rollback to a previous version
    pub async fn rollback(&self, workflow_id: &str, version: &str) -> Result<WorkflowVersion> {
        let target = self
            .repository
            .get_by_version(workflow_id, version)
            .await?
            .ok_or_else(|| WorkflowError::NotFound(format!("Version {} not found", version)))?;

        self.activate(workflow_id, &target.id).await?;

        warn!(
            workflow_id = %workflow_id,
            version = %version,
            "Rolled back to previous version"
        );

        Ok(target)
    }

    /// Deprecate a version
    pub async fn deprecate(&self, version_id: &str) -> Result<()> {
        if let Some(mut version) = self.repository.get(version_id).await? {
            version.is_deprecated = true;
            version.is_active = false;
            self.repository.update(&version).await?;

            info!(
                version_id = %version_id,
                version = %version.version,
                "Deprecated workflow version"
            );

            Ok(())
        } else {
            Err(WorkflowError::NotFound(version_id.to_string()))
        }
    }

    /// Get version history
    pub async fn get_history(&self, workflow_id: &str) -> Result<Vec<WorkflowVersion>> {
        self.repository.list_versions(workflow_id).await
    }

    /// Compare two versions
    pub async fn compare(&self, version_id_a: &str, version_id_b: &str) -> Result<VersionDiff> {
        let version_a = self
            .repository
            .get(version_id_a)
            .await?
            .ok_or_else(|| WorkflowError::NotFound(version_id_a.to_string()))?;

        let version_b = self
            .repository
            .get(version_id_b)
            .await?
            .ok_or_else(|| WorkflowError::NotFound(version_id_b.to_string()))?;

        let steps_a: std::collections::HashSet<_> =
            version_a.definition.steps.iter().map(|s| &s.id).collect();
        let steps_b: std::collections::HashSet<_> =
            version_b.definition.steps.iter().map(|s| &s.id).collect();

        let steps_added: Vec<_> = steps_b.difference(&steps_a).map(|s| s.to_string()).collect();
        let steps_removed: Vec<_> = steps_a.difference(&steps_b).map(|s| s.to_string()).collect();

        // Find modified steps (same ID but different content)
        let mut steps_modified = Vec::new();
        for step_a in &version_a.definition.steps {
            for step_b in &version_b.definition.steps {
                if step_a.id == step_b.id && step_a.name != step_b.name {
                    steps_modified.push(step_a.id.clone());
                }
            }
        }

        debug!(
            version_a = %version_a.version,
            version_b = %version_b.version,
            added = steps_added.len(),
            removed = steps_removed.len(),
            modified = steps_modified.len(),
            "Compared workflow versions"
        );

        Ok(VersionDiff {
            old_version: version_a.version,
            new_version: version_b.version,
            steps_added,
            steps_removed,
            steps_modified,
            connection_changes: Vec::new(), // Simplified
        })
    }
}

/// Version bump type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionBump {
    Major,
    Minor,
    Patch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{WorkflowStep, StepType, StepAction};

    fn create_test_definition() -> WorkflowDefinition {
        WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "Test workflow description".to_string(),
            steps: vec![WorkflowStep::new(
                "Test Step",
                StepType::Action,
                StepAction::Wait { duration_secs: 1 },
            ).with_id("step-1")],
            metadata: Default::default(),
            timeout_secs: None,
        }
    }

    #[tokio::test]
    async fn test_version_creation() {
        let repo = Arc::new(InMemoryVersionRepository::new());
        let manager = VersionManager::new(repo);

        let definition = create_test_definition();
        let version = manager.create_initial("wf-1", definition).await.unwrap();

        assert_eq!(version.version, "1.0.0");
        assert!(version.is_active);
    }

    #[tokio::test]
    async fn test_version_bumps() {
        let repo = Arc::new(InMemoryVersionRepository::new());
        let manager = VersionManager::new(repo);

        let definition = create_test_definition();
        manager.create_initial("wf-1", definition.clone()).await.unwrap();

        // Minor bump
        let v2 = manager
            .create_version("wf-1", definition.clone(), VersionBump::Minor)
            .await
            .unwrap();
        assert_eq!(v2.version, "1.1.0");

        // Patch bump
        let v3 = manager
            .create_version("wf-1", definition.clone(), VersionBump::Patch)
            .await
            .unwrap();
        assert_eq!(v3.version, "1.1.1");

        // Major bump
        let v4 = manager
            .create_version("wf-1", definition, VersionBump::Major)
            .await
            .unwrap();
        assert_eq!(v4.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_rollback() {
        let repo = Arc::new(InMemoryVersionRepository::new());
        let manager = VersionManager::new(repo);

        let definition = create_test_definition();
        manager.create_initial("wf-1", definition.clone()).await.unwrap();

        manager
            .create_version("wf-1", definition, VersionBump::Minor)
            .await
            .unwrap();

        // Rollback to 1.0.0
        let rolled_back = manager.rollback("wf-1", "1.0.0").await.unwrap();
        assert_eq!(rolled_back.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_version_history() {
        let repo = Arc::new(InMemoryVersionRepository::new());
        let manager = VersionManager::new(repo);

        let definition = create_test_definition();
        manager.create_initial("wf-1", definition.clone()).await.unwrap();
        manager
            .create_version("wf-1", definition.clone(), VersionBump::Minor)
            .await
            .unwrap();
        manager
            .create_version("wf-1", definition, VersionBump::Minor)
            .await
            .unwrap();

        let history = manager.get_history("wf-1").await.unwrap();
        assert_eq!(history.len(), 3);
        // Should be sorted descending
        assert_eq!(history[0].version, "1.2.0");
        assert_eq!(history[1].version, "1.1.0");
        assert_eq!(history[2].version, "1.0.0");
    }
}
