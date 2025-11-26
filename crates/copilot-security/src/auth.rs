//! Authentication service
//!
//! Provides user authentication, registration, and token management.

use crate::audit::{AuditEvent, AuditEventType, AuditLogger, AuditOutcome};
use crate::error::{Result, SecurityError};
use crate::jwt::{Claims, JwtConfig, JwtManager, TokenPair};
use crate::password::{PasswordConfig, PasswordManager};
use crate::rbac::{AuthContext, RbacManager, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID
    pub id: String,
    /// Username (unique)
    pub username: String,
    /// Email address (unique)
    pub email: String,
    /// Password hash (Argon2)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// User's roles
    pub roles: Vec<String>,
    /// Whether the user is active
    pub is_active: bool,
    /// Whether email is verified
    pub email_verified: bool,
    /// Tenant ID (for multi-tenant)
    pub tenant_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Last login timestamp
    pub last_login_at: Option<DateTime<Utc>>,
    /// Failed login attempts
    pub failed_login_attempts: u32,
    /// Account locked until
    pub locked_until: Option<DateTime<Utc>>,
    /// User metadata
    pub metadata: serde_json::Value,
}

impl User {
    /// Create a new user
    pub fn new(username: &str, email: &str, password_hash: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            roles: vec!["user".to_string()],
            is_active: true,
            email_verified: false,
            tenant_id: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Check if the account is locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    /// Get roles as Role enums
    pub fn get_roles(&self) -> Vec<Role> {
        self.roles
            .iter()
            .filter_map(|s| Role::from_str(s).ok())
            .collect()
    }

    /// Create an auth context for this user
    pub fn to_auth_context(&self) -> AuthContext {
        AuthContext::from_role_strings(self.id.clone(), &self.roles)
    }
}

/// Login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Username or email
    pub username_or_email: String,
    /// Password
    pub password: String,
}

/// Registration request
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// Tenant ID (optional)
    pub tenant_id: Option<String>,
}

/// Login response
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    /// Token pair
    #[serde(flatten)]
    pub tokens: TokenPair,
    /// User info
    pub user: UserInfo,
}

/// User info (public subset of User)
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub tenant_id: Option<String>,
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            tenant_id: user.tenant_id.clone(),
        }
    }
}

/// User storage trait
#[async_trait::async_trait]
pub trait UserStore: Send + Sync {
    /// Find user by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<User>>;
    /// Find user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<User>>;
    /// Find user by email
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    /// Create a new user
    async fn create(&self, user: User) -> Result<User>;
    /// Update a user
    async fn update(&self, user: User) -> Result<User>;
    /// Delete a user
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Token blacklist trait (for logout/revocation)
#[async_trait::async_trait]
pub trait TokenBlacklist: Send + Sync {
    /// Check if a token ID is blacklisted
    async fn is_blacklisted(&self, token_id: &str) -> bool;
    /// Blacklist a token ID
    async fn blacklist(&self, token_id: &str, expires_at: DateTime<Utc>) -> Result<()>;
    /// Clean up expired blacklist entries
    async fn cleanup_expired(&self) -> Result<()>;
}

/// In-memory user store (for testing/development)
#[derive(Debug, Default)]
pub struct InMemoryUserStore {
    users: Arc<RwLock<Vec<User>>>,
}

impl InMemoryUserStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl UserStore for InMemoryUserStore {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users
            .iter()
            .find(|u| u.username.to_lowercase() == username.to_lowercase())
            .cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users
            .iter()
            .find(|u| u.email.to_lowercase() == email.to_lowercase())
            .cloned())
    }

    async fn create(&self, user: User) -> Result<User> {
        let mut users = self.users.write().await;

        // Check for duplicates
        if users.iter().any(|u| u.username.to_lowercase() == user.username.to_lowercase()) {
            return Err(SecurityError::UserAlreadyExists(user.username.clone()));
        }
        if users.iter().any(|u| u.email.to_lowercase() == user.email.to_lowercase()) {
            return Err(SecurityError::UserAlreadyExists(user.email.clone()));
        }

        users.push(user.clone());
        Ok(user)
    }

    async fn update(&self, user: User) -> Result<User> {
        let mut users = self.users.write().await;

        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            *existing = user.clone();
            Ok(user)
        } else {
            Err(SecurityError::UserNotFound)
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut users = self.users.write().await;
        let len_before = users.len();
        users.retain(|u| u.id != id);

        if users.len() < len_before {
            Ok(())
        } else {
            Err(SecurityError::UserNotFound)
        }
    }
}

/// In-memory token blacklist
#[derive(Debug, Default)]
pub struct InMemoryTokenBlacklist {
    blacklist: Arc<RwLock<HashSet<String>>>,
}

impl InMemoryTokenBlacklist {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl TokenBlacklist for InMemoryTokenBlacklist {
    async fn is_blacklisted(&self, token_id: &str) -> bool {
        self.blacklist.read().await.contains(token_id)
    }

    async fn blacklist(&self, token_id: &str, _expires_at: DateTime<Utc>) -> Result<()> {
        self.blacklist.write().await.insert(token_id.to_string());
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<()> {
        // In-memory version doesn't track expiration
        Ok(())
    }
}

/// Authentication service configuration
#[derive(Debug, Clone)]
pub struct AuthServiceConfig {
    /// JWT configuration
    pub jwt_config: JwtConfig,
    /// Password configuration
    pub password_config: PasswordConfig,
    /// Maximum failed login attempts before lockout
    pub max_failed_attempts: u32,
    /// Account lockout duration in seconds
    pub lockout_duration_secs: i64,
    /// Require email verification
    pub require_email_verification: bool,
}

impl Default for AuthServiceConfig {
    fn default() -> Self {
        Self {
            jwt_config: JwtConfig::default(),
            password_config: PasswordConfig::default(),
            max_failed_attempts: 5,
            lockout_duration_secs: 900, // 15 minutes
            require_email_verification: false,
        }
    }
}

/// Authentication service
pub struct AuthService {
    config: AuthServiceConfig,
    jwt_manager: JwtManager,
    password_manager: PasswordManager,
    rbac_manager: RbacManager,
    user_store: Arc<dyn UserStore>,
    token_blacklist: Arc<dyn TokenBlacklist>,
    audit_logger: Arc<dyn AuditLogger>,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(
        config: AuthServiceConfig,
        user_store: Arc<dyn UserStore>,
        token_blacklist: Arc<dyn TokenBlacklist>,
        audit_logger: Arc<dyn AuditLogger>,
    ) -> Result<Self> {
        let jwt_manager = JwtManager::new(config.jwt_config.clone());
        let password_manager = PasswordManager::new(config.password_config.clone())?;
        let rbac_manager = RbacManager::new();

        Ok(Self {
            config,
            jwt_manager,
            password_manager,
            rbac_manager,
            user_store,
            token_blacklist,
            audit_logger,
        })
    }

    /// Create with default in-memory stores (for testing)
    pub fn in_memory(config: AuthServiceConfig) -> Result<Self> {
        use crate::audit::TracingAuditLogger;

        Self::new(
            config,
            Arc::new(InMemoryUserStore::new()),
            Arc::new(InMemoryTokenBlacklist::new()),
            Arc::new(TracingAuditLogger),
        )
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest, ip: Option<&str>) -> Result<User> {
        // Validate password
        self.password_manager.validate_password(&request.password)?;

        // Hash password
        let password_hash = self.password_manager.hash_password(&request.password)?;

        // Create user
        let mut user = User::new(&request.username, &request.email, &password_hash);
        user.tenant_id = request.tenant_id;

        // Store user
        let user = self.user_store.create(user).await?;

        // Audit log
        let event = AuditEvent::new(AuditEventType::UserCreated, "auth.register")
            .with_actor(&user.id, "user")
            .with_resource("user", &user.id)
            .with_outcome(AuditOutcome::Success);

        let event = if let Some(ip) = ip {
            event.with_ip_str(ip)
        } else {
            event
        };

        self.audit_logger.log(event).await;

        Ok(user)
    }

    /// Authenticate a user and return tokens
    pub async fn login(&self, request: LoginRequest, ip: Option<&str>) -> Result<LoginResponse> {
        // Find user by username or email
        let user = self
            .user_store
            .find_by_username(&request.username_or_email)
            .await?
            .or(self
                .user_store
                .find_by_email(&request.username_or_email)
                .await?)
            .ok_or(SecurityError::AuthenticationFailed(
                "Invalid credentials".to_string(),
            ))?;

        // Check if account is locked
        if user.is_locked() {
            self.log_login_failure(&user.id, ip, "Account locked").await;
            return Err(SecurityError::AuthenticationFailed(
                "Account is temporarily locked".to_string(),
            ));
        }

        // Check if account is active
        if !user.is_active {
            self.log_login_failure(&user.id, ip, "Account disabled").await;
            return Err(SecurityError::AuthenticationFailed(
                "Account is disabled".to_string(),
            ));
        }

        // Check email verification if required
        if self.config.require_email_verification && !user.email_verified {
            self.log_login_failure(&user.id, ip, "Email not verified").await;
            return Err(SecurityError::AuthenticationFailed(
                "Email not verified".to_string(),
            ));
        }

        // Verify password
        let password_valid = self
            .password_manager
            .verify_password(&request.password, &user.password_hash)?;

        if !password_valid {
            self.handle_failed_login(&user).await?;
            self.log_login_failure(&user.id, ip, "Invalid password").await;
            return Err(SecurityError::AuthenticationFailed(
                "Invalid credentials".to_string(),
            ));
        }

        // Generate tokens
        let tokens = self.jwt_manager.generate_token_pair(
            &user.id,
            &user.email,
            &user.username,
            user.roles.clone(),
        )?;

        // Update user login info
        let mut updated_user = user.clone();
        updated_user.last_login_at = Some(Utc::now());
        updated_user.failed_login_attempts = 0;
        updated_user.locked_until = None;
        let _ = self.user_store.update(updated_user).await;

        // Audit log
        let event = AuditEvent::new(AuditEventType::LoginSuccess, "auth.login")
            .with_actor(&user.id, "user")
            .with_outcome(AuditOutcome::Success);

        let event = if let Some(ip) = ip {
            event.with_ip_str(ip)
        } else {
            event
        };

        self.audit_logger.log(event).await;

        Ok(LoginResponse {
            tokens,
            user: UserInfo::from(&user),
        })
    }

    /// Refresh tokens
    pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair> {
        // Validate refresh token
        let claims = self.jwt_manager.validate_refresh_token(refresh_token)?;

        // Check if token is blacklisted
        if self.token_blacklist.is_blacklisted(&claims.jti).await {
            return Err(SecurityError::TokenRevoked);
        }

        // Blacklist old refresh token
        let expires_at = DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now);
        self.token_blacklist.blacklist(&claims.jti, expires_at).await?;

        // Generate new tokens
        let tokens = self.jwt_manager.generate_token_pair(
            &claims.sub,
            &claims.email,
            &claims.username,
            claims.roles,
        )?;

        // Audit log
        let event = AuditEvent::new(AuditEventType::TokenRefresh, "auth.refresh")
            .with_actor(&claims.sub, "user")
            .with_outcome(AuditOutcome::Success);

        self.audit_logger.log(event).await;

        Ok(tokens)
    }

    /// Logout (revoke tokens)
    pub async fn logout(&self, access_token: &str) -> Result<()> {
        // Validate token (to get claims)
        let claims = self.jwt_manager.validate_access_token(access_token)?;

        // Blacklist the token
        let expires_at = DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now);
        self.token_blacklist.blacklist(&claims.jti, expires_at).await?;

        // Audit log
        let event = AuditEvent::new(AuditEventType::Logout, "auth.logout")
            .with_actor(&claims.sub, "user")
            .with_outcome(AuditOutcome::Success);

        self.audit_logger.log(event).await;

        Ok(())
    }

    /// Validate an access token
    pub async fn validate_token(&self, token: &str) -> Result<Claims> {
        let claims = self.jwt_manager.validate_access_token(token)?;

        // Check if token is blacklisted
        if self.token_blacklist.is_blacklisted(&claims.jti).await {
            return Err(SecurityError::TokenRevoked);
        }

        Ok(claims)
    }

    /// Create an auth context from a token
    pub async fn get_auth_context(&self, token: &str) -> Result<AuthContext> {
        let claims = self.validate_token(token).await?;

        let roles: Vec<Role> = claims
            .roles
            .iter()
            .filter_map(|s| Role::from_str(s).ok())
            .collect();

        Ok(AuthContext::new(claims.sub, roles))
    }

    /// Get the RBAC manager
    pub fn rbac(&self) -> &RbacManager {
        &self.rbac_manager
    }

    /// Get the JWT manager
    pub fn jwt(&self) -> &JwtManager {
        &self.jwt_manager
    }

    /// Get the password manager
    pub fn password(&self) -> &PasswordManager {
        &self.password_manager
    }

    /// Change password
    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let user = self
            .user_store
            .find_by_id(user_id)
            .await?
            .ok_or(SecurityError::UserNotFound)?;

        // Verify old password
        if !self.password_manager.verify_password(old_password, &user.password_hash)? {
            return Err(SecurityError::AuthenticationFailed(
                "Invalid current password".to_string(),
            ));
        }

        // Validate and hash new password
        let new_hash = self.password_manager.hash_password(new_password)?;

        // Update user
        let mut updated_user = user;
        updated_user.password_hash = new_hash;
        updated_user.updated_at = Utc::now();
        self.user_store.update(updated_user).await?;

        // Audit log
        let event = AuditEvent::new(AuditEventType::PasswordChanged, "auth.change_password")
            .with_actor(user_id, "user")
            .with_resource("user", user_id)
            .with_outcome(AuditOutcome::Success);

        self.audit_logger.log(event).await;

        Ok(())
    }

    /// Handle failed login attempt
    async fn handle_failed_login(&self, user: &User) -> Result<()> {
        let mut updated_user = user.clone();
        updated_user.failed_login_attempts += 1;

        // Lock account if too many failed attempts
        if updated_user.failed_login_attempts >= self.config.max_failed_attempts {
            updated_user.locked_until = Some(
                Utc::now() + chrono::Duration::seconds(self.config.lockout_duration_secs),
            );
        }

        self.user_store.update(updated_user).await?;
        Ok(())
    }

    /// Log a failed login attempt
    async fn log_login_failure(&self, user_id: &str, ip: Option<&str>, reason: &str) {
        let event = AuditEvent::new(AuditEventType::LoginFailure, "auth.login")
            .with_actor(user_id, "user")
            .with_outcome(AuditOutcome::Failure)
            .with_description(reason);

        let event = if let Some(ip) = ip {
            event.with_ip_str(ip)
        } else {
            event
        };

        self.audit_logger.log(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_service() -> AuthService {
        AuthService::in_memory(AuthServiceConfig::default()).unwrap()
    }

    #[tokio::test]
    async fn test_register_and_login() {
        let service = create_test_service().await;

        // Register
        let register_req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            tenant_id: None,
        };

        let user = service.register(register_req, None).await.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");

        // Login
        let login_req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "SecurePass123".to_string(),
        };

        let response = service.login(login_req, None).await.unwrap();
        assert!(!response.tokens.access_token.is_empty());
        assert_eq!(response.user.username, "testuser");
    }

    #[tokio::test]
    async fn test_invalid_login() {
        let service = create_test_service().await;

        // Register
        let register_req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            tenant_id: None,
        };

        service.register(register_req, None).await.unwrap();

        // Login with wrong password
        let login_req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "wrongpassword".to_string(),
        };

        let result = service.login(login_req, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_validation() {
        let service = create_test_service().await;

        // Register and login
        let register_req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            tenant_id: None,
        };

        service.register(register_req, None).await.unwrap();

        let login_req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "SecurePass123".to_string(),
        };

        let response = service.login(login_req, None).await.unwrap();

        // Validate token
        let claims = service
            .validate_token(&response.tokens.access_token)
            .await
            .unwrap();

        assert_eq!(claims.username, "testuser");
    }

    #[tokio::test]
    async fn test_token_refresh() {
        let service = create_test_service().await;

        // Register and login
        let register_req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            tenant_id: None,
        };

        service.register(register_req, None).await.unwrap();

        let login_req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "SecurePass123".to_string(),
        };

        let response = service.login(login_req, None).await.unwrap();

        // Refresh tokens
        let new_tokens = service
            .refresh_tokens(&response.tokens.refresh_token)
            .await
            .unwrap();

        assert_ne!(response.tokens.access_token, new_tokens.access_token);

        // Old refresh token should be blacklisted
        let result = service
            .refresh_tokens(&response.tokens.refresh_token)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout() {
        let service = create_test_service().await;

        // Register and login
        let register_req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            tenant_id: None,
        };

        service.register(register_req, None).await.unwrap();

        let login_req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "SecurePass123".to_string(),
        };

        let response = service.login(login_req, None).await.unwrap();

        // Logout
        service.logout(&response.tokens.access_token).await.unwrap();

        // Token should be invalid after logout
        let result = service.validate_token(&response.tokens.access_token).await;
        assert!(result.is_err());
    }
}
