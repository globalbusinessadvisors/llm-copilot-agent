//! Role-Based Access Control (RBAC)
//!
//! Provides role and permission management for authorization.

use crate::error::{Result, SecurityError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// System roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Super administrator with full access
    SuperAdmin,
    /// Organization administrator
    Admin,
    /// Regular user
    User,
    /// Read-only access
    Viewer,
    /// Service account for API access
    Service,
    /// Guest with limited access
    Guest,
}

impl Role {
    /// Get role from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "super_admin" | "superadmin" => Ok(Role::SuperAdmin),
            "admin" | "administrator" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            "viewer" | "readonly" => Ok(Role::Viewer),
            "service" | "service_account" => Ok(Role::Service),
            "guest" => Ok(Role::Guest),
            _ => Err(SecurityError::InvalidRole(s.to_string())),
        }
    }

    /// Get role as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::Admin => "admin",
            Role::User => "user",
            Role::Viewer => "viewer",
            Role::Service => "service",
            Role::Guest => "guest",
        }
    }

    /// Get role hierarchy level (higher = more privileges)
    pub fn level(&self) -> u8 {
        match self {
            Role::SuperAdmin => 100,
            Role::Admin => 80,
            Role::User => 50,
            Role::Service => 40,
            Role::Viewer => 20,
            Role::Guest => 10,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Permissions that can be granted to roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // User management
    UsersRead,
    UsersWrite,
    UsersDelete,
    UsersAdmin,

    // Conversation management
    ConversationsRead,
    ConversationsWrite,
    ConversationsDelete,

    // Workflow management
    WorkflowsRead,
    WorkflowsWrite,
    WorkflowsExecute,
    WorkflowsDelete,

    // Context management
    ContextRead,
    ContextWrite,
    ContextDelete,

    // Sandbox management
    SandboxExecute,
    SandboxAdmin,

    // API key management
    ApiKeysRead,
    ApiKeysWrite,
    ApiKeysDelete,

    // System administration
    SystemConfig,
    SystemLogs,
    SystemMetrics,
    SystemAdmin,

    // Tenant management (multi-tenant)
    TenantsRead,
    TenantsWrite,
    TenantsAdmin,
}

impl Permission {
    /// Get permission from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "users:read" | "users_read" => Ok(Permission::UsersRead),
            "users:write" | "users_write" => Ok(Permission::UsersWrite),
            "users:delete" | "users_delete" => Ok(Permission::UsersDelete),
            "users:admin" | "users_admin" => Ok(Permission::UsersAdmin),

            "conversations:read" | "conversations_read" => Ok(Permission::ConversationsRead),
            "conversations:write" | "conversations_write" => Ok(Permission::ConversationsWrite),
            "conversations:delete" | "conversations_delete" => Ok(Permission::ConversationsDelete),

            "workflows:read" | "workflows_read" => Ok(Permission::WorkflowsRead),
            "workflows:write" | "workflows_write" => Ok(Permission::WorkflowsWrite),
            "workflows:execute" | "workflows_execute" => Ok(Permission::WorkflowsExecute),
            "workflows:delete" | "workflows_delete" => Ok(Permission::WorkflowsDelete),

            "context:read" | "context_read" => Ok(Permission::ContextRead),
            "context:write" | "context_write" => Ok(Permission::ContextWrite),
            "context:delete" | "context_delete" => Ok(Permission::ContextDelete),

            "sandbox:execute" | "sandbox_execute" => Ok(Permission::SandboxExecute),
            "sandbox:admin" | "sandbox_admin" => Ok(Permission::SandboxAdmin),

            "api_keys:read" | "apikeys_read" => Ok(Permission::ApiKeysRead),
            "api_keys:write" | "apikeys_write" => Ok(Permission::ApiKeysWrite),
            "api_keys:delete" | "apikeys_delete" => Ok(Permission::ApiKeysDelete),

            "system:config" | "system_config" => Ok(Permission::SystemConfig),
            "system:logs" | "system_logs" => Ok(Permission::SystemLogs),
            "system:metrics" | "system_metrics" => Ok(Permission::SystemMetrics),
            "system:admin" | "system_admin" => Ok(Permission::SystemAdmin),

            "tenants:read" | "tenants_read" => Ok(Permission::TenantsRead),
            "tenants:write" | "tenants_write" => Ok(Permission::TenantsWrite),
            "tenants:admin" | "tenants_admin" => Ok(Permission::TenantsAdmin),

            _ => Err(SecurityError::AuthorizationFailed(format!(
                "Unknown permission: {}",
                s
            ))),
        }
    }

    /// Get permission as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::UsersRead => "users:read",
            Permission::UsersWrite => "users:write",
            Permission::UsersDelete => "users:delete",
            Permission::UsersAdmin => "users:admin",

            Permission::ConversationsRead => "conversations:read",
            Permission::ConversationsWrite => "conversations:write",
            Permission::ConversationsDelete => "conversations:delete",

            Permission::WorkflowsRead => "workflows:read",
            Permission::WorkflowsWrite => "workflows:write",
            Permission::WorkflowsExecute => "workflows:execute",
            Permission::WorkflowsDelete => "workflows:delete",

            Permission::ContextRead => "context:read",
            Permission::ContextWrite => "context:write",
            Permission::ContextDelete => "context:delete",

            Permission::SandboxExecute => "sandbox:execute",
            Permission::SandboxAdmin => "sandbox:admin",

            Permission::ApiKeysRead => "api_keys:read",
            Permission::ApiKeysWrite => "api_keys:write",
            Permission::ApiKeysDelete => "api_keys:delete",

            Permission::SystemConfig => "system:config",
            Permission::SystemLogs => "system:logs",
            Permission::SystemMetrics => "system:metrics",
            Permission::SystemAdmin => "system:admin",

            Permission::TenantsRead => "tenants:read",
            Permission::TenantsWrite => "tenants:write",
            Permission::TenantsAdmin => "tenants:admin",
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RBAC manager for role and permission management
#[derive(Debug, Clone)]
pub struct RbacManager {
    /// Role to permissions mapping
    role_permissions: HashMap<Role, HashSet<Permission>>,
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RbacManager {
    /// Create a new RBAC manager with default role-permission mappings
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // Super Admin - all permissions
        let mut super_admin_perms = HashSet::new();
        super_admin_perms.insert(Permission::UsersRead);
        super_admin_perms.insert(Permission::UsersWrite);
        super_admin_perms.insert(Permission::UsersDelete);
        super_admin_perms.insert(Permission::UsersAdmin);
        super_admin_perms.insert(Permission::ConversationsRead);
        super_admin_perms.insert(Permission::ConversationsWrite);
        super_admin_perms.insert(Permission::ConversationsDelete);
        super_admin_perms.insert(Permission::WorkflowsRead);
        super_admin_perms.insert(Permission::WorkflowsWrite);
        super_admin_perms.insert(Permission::WorkflowsExecute);
        super_admin_perms.insert(Permission::WorkflowsDelete);
        super_admin_perms.insert(Permission::ContextRead);
        super_admin_perms.insert(Permission::ContextWrite);
        super_admin_perms.insert(Permission::ContextDelete);
        super_admin_perms.insert(Permission::SandboxExecute);
        super_admin_perms.insert(Permission::SandboxAdmin);
        super_admin_perms.insert(Permission::ApiKeysRead);
        super_admin_perms.insert(Permission::ApiKeysWrite);
        super_admin_perms.insert(Permission::ApiKeysDelete);
        super_admin_perms.insert(Permission::SystemConfig);
        super_admin_perms.insert(Permission::SystemLogs);
        super_admin_perms.insert(Permission::SystemMetrics);
        super_admin_perms.insert(Permission::SystemAdmin);
        super_admin_perms.insert(Permission::TenantsRead);
        super_admin_perms.insert(Permission::TenantsWrite);
        super_admin_perms.insert(Permission::TenantsAdmin);
        role_permissions.insert(Role::SuperAdmin, super_admin_perms);

        // Admin - most permissions except super admin specific
        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::UsersRead);
        admin_perms.insert(Permission::UsersWrite);
        admin_perms.insert(Permission::UsersDelete);
        admin_perms.insert(Permission::UsersAdmin);
        admin_perms.insert(Permission::ConversationsRead);
        admin_perms.insert(Permission::ConversationsWrite);
        admin_perms.insert(Permission::ConversationsDelete);
        admin_perms.insert(Permission::WorkflowsRead);
        admin_perms.insert(Permission::WorkflowsWrite);
        admin_perms.insert(Permission::WorkflowsExecute);
        admin_perms.insert(Permission::WorkflowsDelete);
        admin_perms.insert(Permission::ContextRead);
        admin_perms.insert(Permission::ContextWrite);
        admin_perms.insert(Permission::ContextDelete);
        admin_perms.insert(Permission::SandboxExecute);
        admin_perms.insert(Permission::SandboxAdmin);
        admin_perms.insert(Permission::ApiKeysRead);
        admin_perms.insert(Permission::ApiKeysWrite);
        admin_perms.insert(Permission::ApiKeysDelete);
        admin_perms.insert(Permission::SystemLogs);
        admin_perms.insert(Permission::SystemMetrics);
        role_permissions.insert(Role::Admin, admin_perms);

        // User - standard user permissions
        let mut user_perms = HashSet::new();
        user_perms.insert(Permission::ConversationsRead);
        user_perms.insert(Permission::ConversationsWrite);
        user_perms.insert(Permission::WorkflowsRead);
        user_perms.insert(Permission::WorkflowsExecute);
        user_perms.insert(Permission::ContextRead);
        user_perms.insert(Permission::ContextWrite);
        user_perms.insert(Permission::SandboxExecute);
        user_perms.insert(Permission::ApiKeysRead);
        user_perms.insert(Permission::ApiKeysWrite);
        role_permissions.insert(Role::User, user_perms);

        // Viewer - read-only permissions
        let mut viewer_perms = HashSet::new();
        viewer_perms.insert(Permission::ConversationsRead);
        viewer_perms.insert(Permission::WorkflowsRead);
        viewer_perms.insert(Permission::ContextRead);
        role_permissions.insert(Role::Viewer, viewer_perms);

        // Service - API access permissions
        let mut service_perms = HashSet::new();
        service_perms.insert(Permission::ConversationsRead);
        service_perms.insert(Permission::ConversationsWrite);
        service_perms.insert(Permission::WorkflowsRead);
        service_perms.insert(Permission::WorkflowsExecute);
        service_perms.insert(Permission::ContextRead);
        service_perms.insert(Permission::ContextWrite);
        service_perms.insert(Permission::SandboxExecute);
        role_permissions.insert(Role::Service, service_perms);

        // Guest - minimal permissions
        let mut guest_perms = HashSet::new();
        guest_perms.insert(Permission::ConversationsRead);
        role_permissions.insert(Role::Guest, guest_perms);

        Self { role_permissions }
    }

    /// Check if a role has a specific permission
    pub fn has_permission(&self, role: &Role, permission: &Permission) -> bool {
        self.role_permissions
            .get(role)
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }

    /// Check if any of the roles has a specific permission
    pub fn any_role_has_permission(&self, roles: &[Role], permission: &Permission) -> bool {
        roles.iter().any(|r| self.has_permission(r, permission))
    }

    /// Check if any of the role strings has a specific permission
    pub fn check_permission(&self, role_strings: &[String], permission: &Permission) -> Result<()> {
        let roles: Vec<Role> = role_strings
            .iter()
            .filter_map(|s| Role::from_str(s).ok())
            .collect();

        if self.any_role_has_permission(&roles, permission) {
            Ok(())
        } else {
            Err(SecurityError::AuthorizationFailed(format!(
                "Missing permission: {}",
                permission
            )))
        }
    }

    /// Get all permissions for a role
    pub fn get_permissions(&self, role: &Role) -> HashSet<Permission> {
        self.role_permissions
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all permissions for multiple roles (union)
    pub fn get_all_permissions(&self, roles: &[Role]) -> HashSet<Permission> {
        roles
            .iter()
            .flat_map(|r| self.get_permissions(r))
            .collect()
    }

    /// Get highest role from a list of roles
    pub fn get_highest_role(&self, roles: &[Role]) -> Option<Role> {
        roles.iter().max_by_key(|r| r.level()).copied()
    }

    /// Check if a role can manage another role (based on hierarchy)
    pub fn can_manage_role(&self, manager_role: &Role, target_role: &Role) -> bool {
        manager_role.level() > target_role.level()
    }

    /// Add a custom permission to a role
    pub fn add_permission(&mut self, role: Role, permission: Permission) {
        self.role_permissions
            .entry(role)
            .or_default()
            .insert(permission);
    }

    /// Remove a permission from a role
    pub fn remove_permission(&mut self, role: &Role, permission: &Permission) {
        if let Some(perms) = self.role_permissions.get_mut(role) {
            perms.remove(permission);
        }
    }
}

/// Authorization context for a request
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: String,
    /// User's roles
    pub roles: Vec<Role>,
    /// User's direct permissions (in addition to role permissions)
    pub permissions: HashSet<Permission>,
    /// Tenant ID (for multi-tenant)
    pub tenant_id: Option<String>,
}

impl AuthContext {
    /// Create a new auth context
    pub fn new(user_id: String, roles: Vec<Role>) -> Self {
        Self {
            user_id,
            roles,
            permissions: HashSet::new(),
            tenant_id: None,
        }
    }

    /// Create from role strings
    pub fn from_role_strings(user_id: String, role_strings: &[String]) -> Self {
        let roles: Vec<Role> = role_strings
            .iter()
            .filter_map(|s| Role::from_str(s).ok())
            .collect();

        Self::new(user_id, roles)
    }

    /// Add a tenant ID
    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Add a direct permission
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self
    }

    /// Check if context has a permission (including direct permissions)
    pub fn has_permission(&self, rbac: &RbacManager, permission: &Permission) -> bool {
        // Check direct permissions first
        if self.permissions.contains(permission) {
            return true;
        }

        // Check role permissions
        rbac.any_role_has_permission(&self.roles, permission)
    }

    /// Require a permission (returns error if not granted)
    pub fn require_permission(
        &self,
        rbac: &RbacManager,
        permission: &Permission,
    ) -> Result<()> {
        if self.has_permission(rbac, permission) {
            Ok(())
        } else {
            Err(SecurityError::AuthorizationFailed(format!(
                "Missing required permission: {}",
                permission
            )))
        }
    }

    /// Check if this context can access a resource owned by another user
    pub fn can_access_user_resource(&self, rbac: &RbacManager, owner_id: &str) -> bool {
        // User can always access their own resources
        if self.user_id == owner_id {
            return true;
        }

        // Admins can access any user's resources
        self.has_permission(rbac, &Permission::UsersAdmin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_from_str() {
        assert_eq!(Role::from_str("admin").unwrap(), Role::Admin);
        assert_eq!(Role::from_str("super_admin").unwrap(), Role::SuperAdmin);
        assert_eq!(Role::from_str("user").unwrap(), Role::User);
        assert!(Role::from_str("invalid").is_err());
    }

    #[test]
    fn test_super_admin_has_all_permissions() {
        let rbac = RbacManager::new();
        let permissions = rbac.get_permissions(&Role::SuperAdmin);

        assert!(permissions.contains(&Permission::UsersAdmin));
        assert!(permissions.contains(&Permission::SystemAdmin));
        assert!(permissions.contains(&Permission::TenantsAdmin));
    }

    #[test]
    fn test_user_has_limited_permissions() {
        let rbac = RbacManager::new();
        let permissions = rbac.get_permissions(&Role::User);

        assert!(permissions.contains(&Permission::ConversationsWrite));
        assert!(!permissions.contains(&Permission::UsersAdmin));
        assert!(!permissions.contains(&Permission::SystemAdmin));
    }

    #[test]
    fn test_check_permission() {
        let rbac = RbacManager::new();

        assert!(rbac
            .check_permission(&["user".to_string()], &Permission::ConversationsWrite)
            .is_ok());

        assert!(rbac
            .check_permission(&["user".to_string()], &Permission::UsersAdmin)
            .is_err());
    }

    #[test]
    fn test_auth_context() {
        let rbac = RbacManager::new();
        let ctx = AuthContext::new("user-123".to_string(), vec![Role::User]);

        assert!(ctx.has_permission(&rbac, &Permission::ConversationsWrite));
        assert!(!ctx.has_permission(&rbac, &Permission::SystemAdmin));
    }

    #[test]
    fn test_can_manage_role() {
        let rbac = RbacManager::new();

        assert!(rbac.can_manage_role(&Role::Admin, &Role::User));
        assert!(rbac.can_manage_role(&Role::SuperAdmin, &Role::Admin));
        assert!(!rbac.can_manage_role(&Role::User, &Role::Admin));
    }

    #[test]
    fn test_can_access_user_resource() {
        let rbac = RbacManager::new();

        let user_ctx = AuthContext::new("user-123".to_string(), vec![Role::User]);
        let admin_ctx = AuthContext::new("admin-456".to_string(), vec![Role::Admin]);

        // User can access own resource
        assert!(user_ctx.can_access_user_resource(&rbac, "user-123"));
        // User cannot access other's resource
        assert!(!user_ctx.can_access_user_resource(&rbac, "other-user"));
        // Admin can access any resource
        assert!(admin_ctx.can_access_user_resource(&rbac, "user-123"));
    }
}
