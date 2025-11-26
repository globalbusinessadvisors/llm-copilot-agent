//! Audit logging
//!
//! Provides comprehensive audit logging for security-relevant events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use tracing::{info, warn};
use uuid::Uuid;

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication events
    LoginSuccess,
    LoginFailure,
    Logout,
    TokenRefresh,
    TokenRevoked,
    PasswordChanged,
    PasswordResetRequested,
    PasswordResetCompleted,

    // Authorization events
    AccessGranted,
    AccessDenied,
    PermissionElevated,
    PermissionRevoked,

    // User management
    UserCreated,
    UserUpdated,
    UserDeleted,
    UserDisabled,
    UserEnabled,
    RoleAssigned,
    RoleRevoked,

    // API key events
    ApiKeyCreated,
    ApiKeyUsed,
    ApiKeyRevoked,
    ApiKeyExpired,

    // Resource access
    ResourceCreated,
    ResourceRead,
    ResourceUpdated,
    ResourceDeleted,

    // System events
    ConfigChanged,
    SystemStartup,
    SystemShutdown,
    MaintenanceStarted,
    MaintenanceEnded,

    // Security events
    RateLimitExceeded,
    SuspiciousActivity,
    SecurityAlertTriggered,
    IpBlocked,
    IpUnblocked,

    // Data events
    DataExported,
    DataImported,
    DataPurged,
}

impl AuditEventType {
    /// Get the severity level of this event type
    pub fn severity(&self) -> AuditSeverity {
        match self {
            // Critical events
            AuditEventType::SecurityAlertTriggered
            | AuditEventType::SuspiciousActivity
            | AuditEventType::IpBlocked => AuditSeverity::Critical,

            // High severity
            AuditEventType::LoginFailure
            | AuditEventType::AccessDenied
            | AuditEventType::RateLimitExceeded
            | AuditEventType::UserDeleted
            | AuditEventType::ApiKeyRevoked
            | AuditEventType::TokenRevoked => AuditSeverity::High,

            // Medium severity
            AuditEventType::LoginSuccess
            | AuditEventType::Logout
            | AuditEventType::PasswordChanged
            | AuditEventType::UserCreated
            | AuditEventType::UserUpdated
            | AuditEventType::RoleAssigned
            | AuditEventType::RoleRevoked
            | AuditEventType::ApiKeyCreated
            | AuditEventType::ResourceDeleted
            | AuditEventType::ConfigChanged
            | AuditEventType::DataExported
            | AuditEventType::DataPurged => AuditSeverity::Medium,

            // Low severity
            AuditEventType::TokenRefresh
            | AuditEventType::AccessGranted
            | AuditEventType::PermissionElevated
            | AuditEventType::PermissionRevoked
            | AuditEventType::UserDisabled
            | AuditEventType::UserEnabled
            | AuditEventType::ApiKeyUsed
            | AuditEventType::ApiKeyExpired
            | AuditEventType::ResourceCreated
            | AuditEventType::ResourceRead
            | AuditEventType::ResourceUpdated
            | AuditEventType::PasswordResetRequested
            | AuditEventType::PasswordResetCompleted
            | AuditEventType::IpUnblocked
            | AuditEventType::DataImported => AuditSeverity::Low,

            // Info level
            AuditEventType::SystemStartup
            | AuditEventType::SystemShutdown
            | AuditEventType::MaintenanceStarted
            | AuditEventType::MaintenanceEnded => AuditSeverity::Info,
        }
    }

    /// Get as string
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::LoginSuccess => "login_success",
            AuditEventType::LoginFailure => "login_failure",
            AuditEventType::Logout => "logout",
            AuditEventType::TokenRefresh => "token_refresh",
            AuditEventType::TokenRevoked => "token_revoked",
            AuditEventType::PasswordChanged => "password_changed",
            AuditEventType::PasswordResetRequested => "password_reset_requested",
            AuditEventType::PasswordResetCompleted => "password_reset_completed",
            AuditEventType::AccessGranted => "access_granted",
            AuditEventType::AccessDenied => "access_denied",
            AuditEventType::PermissionElevated => "permission_elevated",
            AuditEventType::PermissionRevoked => "permission_revoked",
            AuditEventType::UserCreated => "user_created",
            AuditEventType::UserUpdated => "user_updated",
            AuditEventType::UserDeleted => "user_deleted",
            AuditEventType::UserDisabled => "user_disabled",
            AuditEventType::UserEnabled => "user_enabled",
            AuditEventType::RoleAssigned => "role_assigned",
            AuditEventType::RoleRevoked => "role_revoked",
            AuditEventType::ApiKeyCreated => "api_key_created",
            AuditEventType::ApiKeyUsed => "api_key_used",
            AuditEventType::ApiKeyRevoked => "api_key_revoked",
            AuditEventType::ApiKeyExpired => "api_key_expired",
            AuditEventType::ResourceCreated => "resource_created",
            AuditEventType::ResourceRead => "resource_read",
            AuditEventType::ResourceUpdated => "resource_updated",
            AuditEventType::ResourceDeleted => "resource_deleted",
            AuditEventType::ConfigChanged => "config_changed",
            AuditEventType::SystemStartup => "system_startup",
            AuditEventType::SystemShutdown => "system_shutdown",
            AuditEventType::MaintenanceStarted => "maintenance_started",
            AuditEventType::MaintenanceEnded => "maintenance_ended",
            AuditEventType::RateLimitExceeded => "rate_limit_exceeded",
            AuditEventType::SuspiciousActivity => "suspicious_activity",
            AuditEventType::SecurityAlertTriggered => "security_alert_triggered",
            AuditEventType::IpBlocked => "ip_blocked",
            AuditEventType::IpUnblocked => "ip_unblocked",
            AuditEventType::DataExported => "data_exported",
            AuditEventType::DataImported => "data_imported",
            AuditEventType::DataPurged => "data_purged",
        }
    }
}

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Info => "info",
            AuditSeverity::Low => "low",
            AuditSeverity::Medium => "medium",
            AuditSeverity::High => "high",
            AuditSeverity::Critical => "critical",
        }
    }
}

/// Audit event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: AuditEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Actor (user ID or system)
    pub actor_id: Option<String>,
    /// Actor type (user, service, system)
    pub actor_type: String,
    /// Target resource type
    pub resource_type: Option<String>,
    /// Target resource ID
    pub resource_id: Option<String>,
    /// Action performed
    pub action: String,
    /// Outcome (success, failure, error)
    pub outcome: AuditOutcome,
    /// Client IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Tenant ID (for multi-tenant)
    pub tenant_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Description
    pub description: Option<String>,
}

/// Audit event outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Success,
    Failure,
    Error,
    Unknown,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType, action: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            severity: event_type.severity(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_type: "unknown".to_string(),
            resource_type: None,
            resource_id: None,
            action: action.to_string(),
            outcome: AuditOutcome::Unknown,
            ip_address: None,
            user_agent: None,
            request_id: None,
            session_id: None,
            tenant_id: None,
            metadata: HashMap::new(),
            description: None,
        }
    }

    /// Set the actor
    pub fn with_actor(mut self, actor_id: &str, actor_type: &str) -> Self {
        self.actor_id = Some(actor_id.to_string());
        self.actor_type = actor_type.to_string();
        self
    }

    /// Set the resource
    pub fn with_resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    /// Set the outcome
    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    /// Set the IP address
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    /// Set the IP address from string
    pub fn with_ip_str(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    /// Set the user agent
    pub fn with_user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// Set the request ID
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Set the tenant ID
    pub fn with_tenant_id(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Audit logger trait for different backends
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log an audit event
    async fn log(&self, event: AuditEvent);

    /// Query audit events (if supported)
    async fn query(
        &self,
        filter: AuditFilter,
        limit: usize,
        offset: usize,
    ) -> Vec<AuditEvent>;
}

/// Audit event filter for querying
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub event_types: Option<Vec<AuditEventType>>,
    pub severity_min: Option<AuditSeverity>,
    pub actor_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub outcome: Option<AuditOutcome>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub tenant_id: Option<String>,
}

/// Tracing-based audit logger (logs to tracing/structured logging)
#[derive(Debug, Clone, Default)]
pub struct TracingAuditLogger;

#[async_trait::async_trait]
impl AuditLogger for TracingAuditLogger {
    async fn log(&self, event: AuditEvent) {
        let json = serde_json::to_string(&event).unwrap_or_default();

        match event.severity {
            AuditSeverity::Critical | AuditSeverity::High => {
                warn!(
                    audit = true,
                    event_type = event.event_type.as_str(),
                    severity = event.severity.as_str(),
                    actor_id = ?event.actor_id,
                    resource_type = ?event.resource_type,
                    resource_id = ?event.resource_id,
                    outcome = ?event.outcome,
                    "AUDIT: {}", json
                );
            }
            _ => {
                info!(
                    audit = true,
                    event_type = event.event_type.as_str(),
                    severity = event.severity.as_str(),
                    actor_id = ?event.actor_id,
                    resource_type = ?event.resource_type,
                    resource_id = ?event.resource_id,
                    outcome = ?event.outcome,
                    "AUDIT: {}", json
                );
            }
        }
    }

    async fn query(
        &self,
        _filter: AuditFilter,
        _limit: usize,
        _offset: usize,
    ) -> Vec<AuditEvent> {
        // Tracing logger doesn't support querying
        Vec::new()
    }
}

/// In-memory audit logger (for testing)
#[derive(Debug, Default)]
pub struct InMemoryAuditLogger {
    events: std::sync::Arc<tokio::sync::RwLock<Vec<AuditEvent>>>,
}

impl InMemoryAuditLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_events(&self) -> Vec<AuditEvent> {
        self.events.read().await.clone()
    }

    pub async fn clear(&self) {
        self.events.write().await.clear();
    }
}

#[async_trait::async_trait]
impl AuditLogger for InMemoryAuditLogger {
    async fn log(&self, event: AuditEvent) {
        self.events.write().await.push(event);
    }

    async fn query(
        &self,
        filter: AuditFilter,
        limit: usize,
        offset: usize,
    ) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        events
            .iter()
            .filter(|e| {
                // Apply filters
                if let Some(ref types) = filter.event_types {
                    if !types.contains(&e.event_type) {
                        return false;
                    }
                }
                if let Some(ref min_severity) = filter.severity_min {
                    if e.severity < *min_severity {
                        return false;
                    }
                }
                if let Some(ref actor_id) = filter.actor_id {
                    if e.actor_id.as_ref() != Some(actor_id) {
                        return false;
                    }
                }
                if let Some(ref outcome) = filter.outcome {
                    if e.outcome != *outcome {
                        return false;
                    }
                }
                if let Some(ref start) = filter.start_time {
                    if e.timestamp < *start {
                        return false;
                    }
                }
                if let Some(ref end) = filter.end_time {
                    if e.timestamp > *end {
                        return false;
                    }
                }
                true
            })
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Composite audit logger that logs to multiple backends
pub struct CompositeAuditLogger {
    loggers: Vec<Box<dyn AuditLogger>>,
}

impl CompositeAuditLogger {
    pub fn new() -> Self {
        Self { loggers: Vec::new() }
    }

    pub fn add_logger(mut self, logger: impl AuditLogger + 'static) -> Self {
        self.loggers.push(Box::new(logger));
        self
    }
}

impl Default for CompositeAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AuditLogger for CompositeAuditLogger {
    async fn log(&self, event: AuditEvent) {
        for logger in &self.loggers {
            logger.log(event.clone()).await;
        }
    }

    async fn query(
        &self,
        filter: AuditFilter,
        limit: usize,
        offset: usize,
    ) -> Vec<AuditEvent> {
        // Query from first logger that returns results
        for logger in &self.loggers {
            let results = logger.query(filter.clone(), limit, offset).await;
            if !results.is_empty() {
                return results;
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::LoginSuccess, "user.login")
            .with_actor("user-123", "user")
            .with_outcome(AuditOutcome::Success)
            .with_ip_str("192.168.1.1")
            .with_metadata("method", "password");

        assert_eq!(event.event_type, AuditEventType::LoginSuccess);
        assert_eq!(event.actor_id, Some("user-123".to_string()));
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_logger() {
        let logger = InMemoryAuditLogger::new();

        let event = AuditEvent::new(AuditEventType::LoginSuccess, "user.login")
            .with_actor("user-123", "user")
            .with_outcome(AuditOutcome::Success);

        logger.log(event).await;

        let events = logger.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AuditEventType::LoginSuccess);
    }

    #[tokio::test]
    async fn test_audit_query() {
        let logger = InMemoryAuditLogger::new();

        // Log multiple events
        for i in 0..5 {
            let event = AuditEvent::new(AuditEventType::LoginSuccess, "user.login")
                .with_actor(&format!("user-{}", i), "user")
                .with_outcome(AuditOutcome::Success);
            logger.log(event).await;
        }

        let event = AuditEvent::new(AuditEventType::LoginFailure, "user.login")
            .with_actor("user-bad", "user")
            .with_outcome(AuditOutcome::Failure);
        logger.log(event).await;

        // Query only failures
        let filter = AuditFilter {
            outcome: Some(AuditOutcome::Failure),
            ..Default::default()
        };

        let results = logger.query(filter, 100, 0).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type, AuditEventType::LoginFailure);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AuditSeverity::Info < AuditSeverity::Low);
        assert!(AuditSeverity::Low < AuditSeverity::Medium);
        assert!(AuditSeverity::Medium < AuditSeverity::High);
        assert!(AuditSeverity::High < AuditSeverity::Critical);
    }
}
