//! Integration tests for the copilot-security crate.

use copilot_security::{
    auth::{AuthService, AuthServiceConfig, LoginRequest, RegisterRequest},
    jwt::JwtManager,
    password::{PasswordConfig, PasswordManager},
    rbac::{AuthContext, Permission, RbacManager, Role},
    api_key::{ApiKeyManager, ApiKeyScope},
    rate_limit::{RateLimitConfig, RateLimitKey, RateLimitManager, RateLimitTier},
    audit::{AuditEvent, AuditEventType, AuditOutcome, AuditLogger, InMemoryAuditLogger},
};
use std::collections::HashSet;

// ==================== JWT Tests ====================

#[test]
fn test_jwt_token_generation_and_validation() {
    let manager = JwtManager::from_secret("test-secret-key-for-integration-testing");

    let token_pair = manager
        .generate_token_pair(
            "user-123",
            "test@example.com",
            "testuser",
            vec!["user".to_string(), "admin".to_string()],
        )
        .expect("Failed to generate token pair");

    assert!(!token_pair.access_token.is_empty());
    assert!(!token_pair.refresh_token.is_empty());
    assert_eq!(token_pair.token_type, "Bearer");

    // Validate access token
    let claims = manager
        .validate_access_token(&token_pair.access_token)
        .expect("Failed to validate access token");

    assert_eq!(claims.sub, "user-123");
    assert_eq!(claims.email, "test@example.com");
    assert_eq!(claims.username, "testuser");
    assert_eq!(claims.roles, vec!["user", "admin"]);

    // Validate refresh token
    let refresh_claims = manager
        .validate_refresh_token(&token_pair.refresh_token)
        .expect("Failed to validate refresh token");

    assert_eq!(refresh_claims.sub, "user-123");
}

#[test]
fn test_jwt_token_refresh() {
    let manager = JwtManager::from_secret("test-secret-key");

    let pair1 = manager
        .generate_token_pair("user-123", "test@example.com", "testuser", vec!["user".to_string()])
        .unwrap();

    let pair2 = manager
        .refresh_tokens(&pair1.refresh_token)
        .expect("Failed to refresh tokens");

    // New tokens should be different
    assert_ne!(pair1.access_token, pair2.access_token);
    assert_ne!(pair1.refresh_token, pair2.refresh_token);

    // New access token should be valid
    let claims = manager.validate_access_token(&pair2.access_token).unwrap();
    assert_eq!(claims.sub, "user-123");
}

#[test]
fn test_jwt_invalid_token() {
    let manager = JwtManager::from_secret("test-secret");

    let result = manager.validate_access_token("invalid-token");
    assert!(result.is_err());
}

#[test]
fn test_jwt_wrong_secret() {
    let manager1 = JwtManager::from_secret("secret-1");
    let manager2 = JwtManager::from_secret("secret-2");

    let pair = manager1
        .generate_token_pair("user-123", "test@example.com", "testuser", vec![])
        .unwrap();

    // Token from manager1 should not validate with manager2
    let result = manager2.validate_access_token(&pair.access_token);
    assert!(result.is_err());
}

// ==================== Password Tests ====================

#[test]
fn test_password_hashing_and_verification() {
    let manager = PasswordManager::new(PasswordConfig {
        argon2_memory_cost: 4096, // Lower for tests
        argon2_time_cost: 1,
        argon2_parallelism: 1,
        ..Default::default()
    })
    .unwrap();

    let password = "SecurePassword123";
    let hash = manager.hash_password(password).expect("Failed to hash password");

    // Verify correct password
    assert!(manager.verify_password(password, &hash).unwrap());

    // Verify incorrect password
    assert!(!manager.verify_password("WrongPassword123", &hash).unwrap());
}

#[test]
fn test_password_validation() {
    let manager = PasswordManager::new(PasswordConfig {
        min_length: 8,
        require_uppercase: true,
        require_lowercase: true,
        require_digit: true,
        require_special: false,
        argon2_memory_cost: 4096,
        argon2_time_cost: 1,
        argon2_parallelism: 1,
        ..Default::default()
    })
    .unwrap();

    // Valid password
    assert!(manager.validate_password("ValidPass1").is_ok());

    // Too short
    assert!(manager.validate_password("Short1").is_err());

    // No uppercase
    assert!(manager.validate_password("nouppercase1").is_err());

    // No lowercase
    assert!(manager.validate_password("NOLOWERCASE1").is_err());

    // No digit
    assert!(manager.validate_password("NoDigitHere").is_err());
}

#[test]
fn test_password_unique_salts() {
    let manager = PasswordManager::new(PasswordConfig {
        argon2_memory_cost: 4096,
        argon2_time_cost: 1,
        argon2_parallelism: 1,
        ..Default::default()
    })
    .unwrap();

    let password = "SamePassword123";
    let hash1 = manager.hash_password(password).unwrap();
    let hash2 = manager.hash_password(password).unwrap();

    // Hashes should be different due to unique salts
    assert_ne!(hash1, hash2);

    // Both should verify correctly
    assert!(manager.verify_password(password, &hash1).unwrap());
    assert!(manager.verify_password(password, &hash2).unwrap());
}

// ==================== RBAC Tests ====================

#[test]
fn test_rbac_role_permissions() {
    let manager = RbacManager::new();

    // Admin should have user management permissions
    let admin_perms = manager.get_permissions(&Role::Admin);
    assert!(admin_perms.contains(&Permission::UsersRead));
    assert!(admin_perms.contains(&Permission::UsersWrite));

    // User should have limited permissions
    let user_perms = manager.get_permissions(&Role::User);
    assert!(user_perms.contains(&Permission::ConversationsRead));
    assert!(user_perms.contains(&Permission::ConversationsWrite));
    assert!(!user_perms.contains(&Permission::UsersWrite));

    // Viewer should only have read permissions
    let viewer_perms = manager.get_permissions(&Role::Viewer);
    assert!(viewer_perms.contains(&Permission::ConversationsRead));
    assert!(!viewer_perms.contains(&Permission::ConversationsWrite));

    // SuperAdmin should have all permissions
    let super_admin_perms = manager.get_permissions(&Role::SuperAdmin);
    assert!(super_admin_perms.contains(&Permission::UsersDelete));
    assert!(super_admin_perms.contains(&Permission::SystemAdmin));
}

#[test]
fn test_rbac_auth_context() {
    let rbac = RbacManager::new();
    let context = AuthContext::new("user-123".to_string(), vec![Role::User, Role::Admin]);

    // Check permissions from combined roles
    assert!(context.has_permission(&rbac, &Permission::ConversationsRead));
    assert!(context.has_permission(&rbac, &Permission::UsersRead));
}

#[test]
fn test_rbac_role_hierarchy() {
    let manager = RbacManager::new();

    // Get all permissions for a role
    let admin_perms = manager.get_permissions(&Role::Admin);
    let user_perms = manager.get_permissions(&Role::User);

    // Admin should have more permissions than User
    assert!(admin_perms.len() > user_perms.len());
}

#[test]
fn test_rbac_check_permission() {
    let rbac = RbacManager::new();

    // User role should have conversation permissions
    assert!(rbac
        .check_permission(&["user".to_string()], &Permission::ConversationsWrite)
        .is_ok());

    // User role should NOT have admin permissions
    assert!(rbac
        .check_permission(&["user".to_string()], &Permission::UsersAdmin)
        .is_err());
}

// ==================== API Key Tests ====================

#[test]
fn test_api_key_generation() {
    let manager = ApiKeyManager::new();

    let generated = manager.generate_key(
        "Test Key",
        "user-123",
        None,
        None,
        None,
        None,
    );

    // Check key format
    assert!(generated.key.starts_with("cplt_v1_"));
    assert!(generated.metadata.prefix.starts_with("cplt_v1_"));
    assert!(generated.metadata.prefix.ends_with("..."));

    // Check metadata
    assert_eq!(generated.metadata.name, "Test Key");
    assert_eq!(generated.metadata.user_id, "user-123");
    assert!(generated.metadata.is_active);
}

#[test]
fn test_api_key_verification() {
    let manager = ApiKeyManager::new();

    let generated = manager.generate_key(
        "Test Key",
        "user-123",
        None,
        None,
        None,
        None,
    );

    // Verify correct key
    assert!(manager.verify_key(&generated.key, &generated.metadata).is_ok());

    // Verify incorrect key
    let wrong_key = format!("{}_wrong", generated.key);
    assert!(manager.verify_key(&wrong_key, &generated.metadata).is_err());
}

#[test]
fn test_api_key_scopes() {
    let manager = ApiKeyManager::new();

    let mut scopes = HashSet::new();
    scopes.insert(ApiKeyScope::Read);
    scopes.insert(ApiKeyScope::Chat);

    let generated = manager.generate_key(
        "Limited Key",
        "user-123",
        None,
        Some(scopes),
        None,
        None,
    );

    // Check scopes
    assert!(generated.metadata.has_scope(&ApiKeyScope::Read));
    assert!(generated.metadata.has_scope(&ApiKeyScope::Chat));
    assert!(!generated.metadata.has_scope(&ApiKeyScope::Admin));
    assert!(!generated.metadata.has_scope(&ApiKeyScope::Workflows));

    // Admin scope grants all
    let admin_scopes = ApiKeyScope::all_scopes();
    let admin_key = manager.generate_key(
        "Admin Key",
        "admin-123",
        None,
        Some(admin_scopes),
        None,
        None,
    );

    assert!(admin_key.metadata.has_scope(&ApiKeyScope::Read));
    assert!(admin_key.metadata.has_scope(&ApiKeyScope::Workflows));
    assert!(admin_key.metadata.has_scope(&ApiKeyScope::Sandbox));
}

#[test]
fn test_api_key_format_validation() {
    let manager = ApiKeyManager::new();

    let generated = manager.generate_key("Test", "user-123", None, None, None, None);

    // Valid format
    assert!(manager.validate_key_format(&generated.key).is_ok());

    // Invalid formats
    assert!(manager.validate_key_format("invalid").is_err());
    assert!(manager.validate_key_format("cplt_invalid").is_err());
    assert!(manager.validate_key_format("other_v1_abc").is_err());
}

// ==================== Rate Limit Tests ====================

#[tokio::test]
async fn test_rate_limiting_basic() {
    let manager = RateLimitManager::new(RateLimitConfig {
        enabled: true,
        ..Default::default()
    });

    let key = RateLimitKey::Ip("127.0.0.1".to_string());
    let tier = RateLimitTier::Free; // 60 rpm, 20 burst

    // First few requests should succeed
    for _ in 0..5 {
        let result = manager.check_limit(&key, &tier).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.allowed);
    }
}

#[tokio::test]
async fn test_rate_limiting_unlimited_tier() {
    let manager = RateLimitManager::default_config();

    let key = RateLimitKey::User("admin".to_string());
    let tier = RateLimitTier::Unlimited;

    // Unlimited tier should always succeed
    for _ in 0..100 {
        let result = manager.check_limit(&key, &tier).await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let manager = RateLimitManager::new(RateLimitConfig {
        enabled: false,
        ..Default::default()
    });

    let key = RateLimitKey::Ip("127.0.0.1".to_string());
    let tier = RateLimitTier::Anonymous; // Most restrictive

    // Should always succeed when disabled
    for _ in 0..100 {
        let result = manager.check_limit(&key, &tier).await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }
}

#[test]
fn test_rate_limit_tiers() {
    // Check tier ordering
    assert!(
        RateLimitTier::Anonymous.requests_per_minute().unwrap()
            < RateLimitTier::Free.requests_per_minute().unwrap()
    );
    assert!(
        RateLimitTier::Free.requests_per_minute().unwrap()
            < RateLimitTier::Standard.requests_per_minute().unwrap()
    );
    assert!(
        RateLimitTier::Standard.requests_per_minute().unwrap()
            < RateLimitTier::Pro.requests_per_minute().unwrap()
    );
    assert!(
        RateLimitTier::Pro.requests_per_minute().unwrap()
            < RateLimitTier::Enterprise.requests_per_minute().unwrap()
    );
    assert!(RateLimitTier::Unlimited.requests_per_minute().is_none());
}

#[test]
fn test_rate_limit_key_generation() {
    let ip_key = RateLimitKey::Ip("192.168.1.1".to_string());
    assert_eq!(ip_key.to_key_string(), "ip:192.168.1.1");

    let user_key = RateLimitKey::User("user-123".to_string());
    assert_eq!(user_key.to_key_string(), "user:user-123");

    let api_key = RateLimitKey::ApiKey("key-456".to_string());
    assert_eq!(api_key.to_key_string(), "apikey:key-456");

    let composite = RateLimitKey::Composite("user-123".to_string(), "/api/chat".to_string());
    assert_eq!(composite.to_key_string(), "user-123:/api/chat");
}

// ==================== Audit Tests ====================

#[tokio::test]
async fn test_audit_logging() {
    let logger = InMemoryAuditLogger::new();

    // Log some events
    let event1 = AuditEvent::new(AuditEventType::LoginSuccess, "auth.login")
        .with_actor("user-123", "user")
        .with_outcome(AuditOutcome::Success);

    let event2 = AuditEvent::new(AuditEventType::AccessDenied, "api.access")
        .with_actor("user-456", "user")
        .with_resource("conversation", "conv-789")
        .with_outcome(AuditOutcome::Failure);

    logger.log(event1).await;
    logger.log(event2).await;

    // Query events
    let all_events = logger.query(Default::default(), 100, 0).await;
    assert_eq!(all_events.len(), 2);

    // Check event details
    let login_event = &all_events[0];
    assert_eq!(login_event.event_type, AuditEventType::LoginSuccess);
    assert_eq!(login_event.outcome, AuditOutcome::Success);

    let denied_event = &all_events[1];
    assert_eq!(denied_event.event_type, AuditEventType::AccessDenied);
    assert_eq!(denied_event.outcome, AuditOutcome::Failure);
}

#[test]
fn test_audit_event_builder() {
    let event = AuditEvent::new(AuditEventType::UserCreated, "admin.users.create")
        .with_actor("admin-123", "admin")
        .with_resource("user", "new-user-456")
        .with_tenant_id("tenant-789")
        .with_ip_str("192.168.1.100")
        .with_description("Created new user account")
        .with_outcome(AuditOutcome::Success)
        .with_metadata("role", "user");

    assert_eq!(event.event_type, AuditEventType::UserCreated);
    assert_eq!(event.action, "admin.users.create");
    assert_eq!(event.actor_id, Some("admin-123".to_string()));
    assert_eq!(event.resource_type, Some("user".to_string()));
    assert_eq!(event.resource_id, Some("new-user-456".to_string()));
    assert_eq!(event.tenant_id, Some("tenant-789".to_string()));
    assert_eq!(event.description, Some("Created new user account".to_string()));
    assert!(event.metadata.contains_key("role"));
}

// ==================== Auth Service Integration Tests ====================

#[tokio::test]
async fn test_auth_service_registration_and_login() {
    let config = AuthServiceConfig {
        password_config: PasswordConfig {
            argon2_memory_cost: 4096,
            argon2_time_cost: 1,
            argon2_parallelism: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let service = AuthService::in_memory(config).expect("Failed to create auth service");

    // Register
    let register_req = RegisterRequest {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "SecurePass123".to_string(),
        tenant_id: None,
    };

    let user = service.register(register_req, None).await.expect("Failed to register");
    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, "test@example.com");

    // Login
    let login_req = LoginRequest {
        username_or_email: "testuser".to_string(),
        password: "SecurePass123".to_string(),
    };

    let response = service.login(login_req, None).await.expect("Failed to login");
    assert!(!response.tokens.access_token.is_empty());
    assert_eq!(response.user.username, "testuser");

    // Validate token
    let claims = service
        .validate_token(&response.tokens.access_token)
        .await
        .expect("Failed to validate token");
    assert_eq!(claims.username, "testuser");
}

#[tokio::test]
async fn test_auth_service_token_refresh() {
    let config = AuthServiceConfig {
        password_config: PasswordConfig {
            argon2_memory_cost: 4096,
            argon2_time_cost: 1,
            argon2_parallelism: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let service = AuthService::in_memory(config).unwrap();

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
        .expect("Failed to refresh tokens");

    assert_ne!(response.tokens.access_token, new_tokens.access_token);
    assert_ne!(response.tokens.refresh_token, new_tokens.refresh_token);

    // Old refresh token should be blacklisted
    let result = service.refresh_tokens(&response.tokens.refresh_token).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_auth_service_logout() {
    let config = AuthServiceConfig {
        password_config: PasswordConfig {
            argon2_memory_cost: 4096,
            argon2_time_cost: 1,
            argon2_parallelism: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let service = AuthService::in_memory(config).unwrap();

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

#[tokio::test]
async fn test_auth_service_invalid_login() {
    let config = AuthServiceConfig {
        password_config: PasswordConfig {
            argon2_memory_cost: 4096,
            argon2_time_cost: 1,
            argon2_parallelism: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let service = AuthService::in_memory(config).unwrap();

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
        password: "WrongPassword123".to_string(),
    };
    let result = service.login(login_req, None).await;
    assert!(result.is_err());
}
