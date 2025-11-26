//! Tenant isolation
//!
//! Provides data isolation mechanisms for multi-tenancy:
//! - Database schema isolation
//! - Vector store namespace isolation
//! - Cache key isolation

use crate::{Result, Tenant, TenantError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// Isolation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationStrategy {
    /// Shared database with tenant_id column
    SharedDatabase,
    /// Separate schema per tenant
    SeparateSchema,
    /// Separate database per tenant
    SeparateDatabase,
}

impl Default for IsolationStrategy {
    fn default() -> Self {
        Self::SeparateSchema
    }
}

/// Tenant context for request processing
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Tenant ID
    pub tenant_id: String,
    /// Tenant slug
    pub slug: String,
    /// Database schema name
    pub schema_name: String,
    /// Vector store namespace
    pub vector_namespace: String,
    /// Cache key prefix
    pub cache_prefix: String,
    /// User ID within tenant
    pub user_id: Option<String>,
    /// Request ID for tracing
    pub request_id: Option<String>,
}

impl TenantContext {
    /// Create from a tenant
    pub fn from_tenant(tenant: &Tenant) -> Self {
        Self {
            tenant_id: tenant.id.clone(),
            slug: tenant.slug.clone(),
            schema_name: tenant.schema_name.clone(),
            vector_namespace: tenant.vector_namespace.clone(),
            cache_prefix: format!("tenant:{}:", tenant.id),
            user_id: None,
            request_id: None,
        }
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }

    /// Get prefixed cache key
    pub fn cache_key(&self, key: &str) -> String {
        format!("{}{}", self.cache_prefix, key)
    }

    /// Get prefixed table name
    pub fn table_name(&self, table: &str) -> String {
        format!("{}.{}", self.schema_name, table)
    }
}

/// Database schema manager for tenant isolation
#[async_trait]
pub trait SchemaManager: Send + Sync {
    /// Create a new schema for a tenant
    async fn create_schema(&self, tenant: &Tenant) -> Result<()>;

    /// Drop a tenant's schema
    async fn drop_schema(&self, tenant: &Tenant) -> Result<()>;

    /// Check if schema exists
    async fn schema_exists(&self, schema_name: &str) -> Result<bool>;

    /// Run migrations on a tenant schema
    async fn run_migrations(&self, tenant: &Tenant) -> Result<()>;

    /// Get schema size in bytes
    async fn get_schema_size(&self, schema_name: &str) -> Result<u64>;
}

/// In-memory schema manager for testing
#[derive(Debug, Default)]
pub struct InMemorySchemaManager {
    schemas: Arc<RwLock<HashMap<String, SchemaInfo>>>,
}

#[derive(Debug, Clone)]
struct SchemaInfo {
    name: String,
    migrated: bool,
    size: u64,
}

impl InMemorySchemaManager {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SchemaManager for InMemorySchemaManager {
    async fn create_schema(&self, tenant: &Tenant) -> Result<()> {
        let mut schemas = self.schemas.write();

        if schemas.contains_key(&tenant.schema_name) {
            return Err(TenantError::AlreadyExists(format!(
                "Schema {} already exists",
                tenant.schema_name
            )));
        }

        schemas.insert(
            tenant.schema_name.clone(),
            SchemaInfo {
                name: tenant.schema_name.clone(),
                migrated: false,
                size: 0,
            },
        );

        info!(
            tenant_id = %tenant.id,
            schema = %tenant.schema_name,
            "Created tenant schema"
        );

        Ok(())
    }

    async fn drop_schema(&self, tenant: &Tenant) -> Result<()> {
        let mut schemas = self.schemas.write();

        if schemas.remove(&tenant.schema_name).is_none() {
            return Err(TenantError::NotFound(format!(
                "Schema {} not found",
                tenant.schema_name
            )));
        }

        info!(
            tenant_id = %tenant.id,
            schema = %tenant.schema_name,
            "Dropped tenant schema"
        );

        Ok(())
    }

    async fn schema_exists(&self, schema_name: &str) -> Result<bool> {
        Ok(self.schemas.read().contains_key(schema_name))
    }

    async fn run_migrations(&self, tenant: &Tenant) -> Result<()> {
        let mut schemas = self.schemas.write();

        if let Some(info) = schemas.get_mut(&tenant.schema_name) {
            info.migrated = true;
            debug!(
                tenant_id = %tenant.id,
                schema = %tenant.schema_name,
                "Ran migrations on tenant schema"
            );
            Ok(())
        } else {
            Err(TenantError::NotFound(format!(
                "Schema {} not found",
                tenant.schema_name
            )))
        }
    }

    async fn get_schema_size(&self, schema_name: &str) -> Result<u64> {
        self.schemas
            .read()
            .get(schema_name)
            .map(|info| info.size)
            .ok_or_else(|| TenantError::NotFound(format!("Schema {} not found", schema_name)))
    }
}

/// Vector store namespace manager for tenant isolation
#[async_trait]
pub trait VectorNamespaceManager: Send + Sync {
    /// Create a namespace for a tenant
    async fn create_namespace(&self, tenant: &Tenant) -> Result<()>;

    /// Delete a tenant's namespace
    async fn delete_namespace(&self, tenant: &Tenant) -> Result<()>;

    /// Check if namespace exists
    async fn namespace_exists(&self, namespace: &str) -> Result<bool>;

    /// Get namespace statistics
    async fn get_namespace_stats(&self, namespace: &str) -> Result<NamespaceStats>;
}

/// Vector namespace statistics
#[derive(Debug, Clone, Default)]
pub struct NamespaceStats {
    /// Number of vectors
    pub vector_count: u64,
    /// Total storage size in bytes
    pub storage_bytes: u64,
    /// Index status
    pub indexed: bool,
}

/// In-memory vector namespace manager for testing
#[derive(Debug, Default)]
pub struct InMemoryVectorNamespaceManager {
    namespaces: Arc<RwLock<HashMap<String, NamespaceStats>>>,
}

impl InMemoryVectorNamespaceManager {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl VectorNamespaceManager for InMemoryVectorNamespaceManager {
    async fn create_namespace(&self, tenant: &Tenant) -> Result<()> {
        let mut namespaces = self.namespaces.write();

        if namespaces.contains_key(&tenant.vector_namespace) {
            return Err(TenantError::AlreadyExists(format!(
                "Namespace {} already exists",
                tenant.vector_namespace
            )));
        }

        namespaces.insert(tenant.vector_namespace.clone(), NamespaceStats::default());

        info!(
            tenant_id = %tenant.id,
            namespace = %tenant.vector_namespace,
            "Created vector namespace"
        );

        Ok(())
    }

    async fn delete_namespace(&self, tenant: &Tenant) -> Result<()> {
        let mut namespaces = self.namespaces.write();

        if namespaces.remove(&tenant.vector_namespace).is_none() {
            return Err(TenantError::NotFound(format!(
                "Namespace {} not found",
                tenant.vector_namespace
            )));
        }

        info!(
            tenant_id = %tenant.id,
            namespace = %tenant.vector_namespace,
            "Deleted vector namespace"
        );

        Ok(())
    }

    async fn namespace_exists(&self, namespace: &str) -> Result<bool> {
        Ok(self.namespaces.read().contains_key(namespace))
    }

    async fn get_namespace_stats(&self, namespace: &str) -> Result<NamespaceStats> {
        self.namespaces
            .read()
            .get(namespace)
            .cloned()
            .ok_or_else(|| TenantError::NotFound(format!("Namespace {} not found", namespace)))
    }
}

/// Isolation manager combining all isolation mechanisms
pub struct IsolationManager {
    schema_manager: Arc<dyn SchemaManager>,
    vector_manager: Arc<dyn VectorNamespaceManager>,
    strategy: IsolationStrategy,
}

impl IsolationManager {
    /// Create a new isolation manager
    pub fn new(
        schema_manager: Arc<dyn SchemaManager>,
        vector_manager: Arc<dyn VectorNamespaceManager>,
        strategy: IsolationStrategy,
    ) -> Self {
        Self {
            schema_manager,
            vector_manager,
            strategy,
        }
    }

    /// Create in-memory isolation manager for testing
    pub fn in_memory() -> Self {
        Self {
            schema_manager: Arc::new(InMemorySchemaManager::new()),
            vector_manager: Arc::new(InMemoryVectorNamespaceManager::new()),
            strategy: IsolationStrategy::SeparateSchema,
        }
    }

    /// Get isolation strategy
    pub fn strategy(&self) -> IsolationStrategy {
        self.strategy
    }

    /// Initialize isolation for a tenant
    pub async fn initialize_tenant(&self, tenant: &Tenant) -> Result<()> {
        info!(
            tenant_id = %tenant.id,
            strategy = ?self.strategy,
            "Initializing tenant isolation"
        );

        // Create database schema
        if matches!(
            self.strategy,
            IsolationStrategy::SeparateSchema | IsolationStrategy::SeparateDatabase
        ) {
            self.schema_manager.create_schema(tenant).await?;
            self.schema_manager.run_migrations(tenant).await?;
        }

        // Create vector namespace
        self.vector_manager.create_namespace(tenant).await?;

        info!(
            tenant_id = %tenant.id,
            "Tenant isolation initialized successfully"
        );

        Ok(())
    }

    /// Clean up isolation for a tenant
    pub async fn cleanup_tenant(&self, tenant: &Tenant) -> Result<()> {
        warn!(
            tenant_id = %tenant.id,
            "Cleaning up tenant isolation (destructive operation)"
        );

        // Delete vector namespace first
        if let Err(e) = self.vector_manager.delete_namespace(tenant).await {
            warn!(
                tenant_id = %tenant.id,
                error = %e,
                "Failed to delete vector namespace"
            );
        }

        // Drop database schema
        if matches!(
            self.strategy,
            IsolationStrategy::SeparateSchema | IsolationStrategy::SeparateDatabase
        ) {
            if let Err(e) = self.schema_manager.drop_schema(tenant).await {
                warn!(
                    tenant_id = %tenant.id,
                    error = %e,
                    "Failed to drop schema"
                );
            }
        }

        info!(
            tenant_id = %tenant.id,
            "Tenant isolation cleaned up"
        );

        Ok(())
    }

    /// Get tenant resource usage
    pub async fn get_resource_usage(&self, tenant: &Tenant) -> Result<TenantResourceUsage> {
        let schema_size = self
            .schema_manager
            .get_schema_size(&tenant.schema_name)
            .await
            .unwrap_or(0);

        let vector_stats = self
            .vector_manager
            .get_namespace_stats(&tenant.vector_namespace)
            .await
            .unwrap_or_default();

        Ok(TenantResourceUsage {
            database_bytes: schema_size,
            vector_count: vector_stats.vector_count,
            vector_bytes: vector_stats.storage_bytes,
            total_bytes: schema_size + vector_stats.storage_bytes,
        })
    }
}

/// Tenant resource usage
#[derive(Debug, Clone, Default)]
pub struct TenantResourceUsage {
    /// Database storage in bytes
    pub database_bytes: u64,
    /// Number of vectors
    pub vector_count: u64,
    /// Vector storage in bytes
    pub vector_bytes: u64,
    /// Total storage in bytes
    pub total_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TenantTier;

    #[test]
    fn test_tenant_context() {
        let tenant = Tenant::new("Test", "test-co", "owner", TenantTier::Professional);
        let ctx = TenantContext::from_tenant(&tenant)
            .with_user("user-123")
            .with_request_id("req-456");

        assert_eq!(ctx.tenant_id, tenant.id);
        assert_eq!(ctx.schema_name, "tenant_test_co");
        assert!(ctx.cache_key("sessions").starts_with("tenant:"));
        assert_eq!(ctx.table_name("users"), "tenant_test_co.users");
        assert_eq!(ctx.user_id, Some("user-123".to_string()));
        assert_eq!(ctx.request_id, Some("req-456".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_schema_manager() {
        let manager = InMemorySchemaManager::new();
        let tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);

        // Create schema
        manager.create_schema(&tenant).await.unwrap();
        assert!(manager.schema_exists(&tenant.schema_name).await.unwrap());

        // Run migrations
        manager.run_migrations(&tenant).await.unwrap();

        // Drop schema
        manager.drop_schema(&tenant).await.unwrap();
        assert!(!manager.schema_exists(&tenant.schema_name).await.unwrap());
    }

    #[tokio::test]
    async fn test_in_memory_vector_namespace_manager() {
        let manager = InMemoryVectorNamespaceManager::new();
        let tenant = Tenant::new("Test", "test", "owner", TenantTier::Free);

        // Create namespace
        manager.create_namespace(&tenant).await.unwrap();
        assert!(manager.namespace_exists(&tenant.vector_namespace).await.unwrap());

        // Get stats
        let stats = manager.get_namespace_stats(&tenant.vector_namespace).await.unwrap();
        assert_eq!(stats.vector_count, 0);

        // Delete namespace
        manager.delete_namespace(&tenant).await.unwrap();
        assert!(!manager.namespace_exists(&tenant.vector_namespace).await.unwrap());
    }

    #[tokio::test]
    async fn test_isolation_manager() {
        let manager = IsolationManager::in_memory();
        let tenant = Tenant::new("Test", "test", "owner", TenantTier::Professional);

        // Initialize tenant
        manager.initialize_tenant(&tenant).await.unwrap();

        // Get resource usage
        let usage = manager.get_resource_usage(&tenant).await.unwrap();
        assert_eq!(usage.vector_count, 0);

        // Cleanup tenant
        manager.cleanup_tenant(&tenant).await.unwrap();
    }
}
