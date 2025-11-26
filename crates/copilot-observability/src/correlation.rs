//! Request correlation and context propagation
//!
//! Provides correlation ID management for distributed tracing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

/// Correlation context for request tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// Unique request ID
    pub request_id: String,
    /// Trace ID (for distributed tracing)
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Tenant ID (for multi-tenant)
    pub tenant_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// Baggage items (propagated context)
    pub baggage: std::collections::HashMap<String, String>,
}

impl Default for CorrelationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrelationContext {
    /// Create a new correlation context
    pub fn new() -> Self {
        let trace_id = Uuid::new_v4().to_string().replace('-', "");
        Self {
            request_id: Uuid::new_v4().to_string(),
            trace_id: trace_id.clone(),
            span_id: Self::generate_span_id(),
            parent_span_id: None,
            tenant_id: None,
            user_id: None,
            session_id: None,
            timestamp: Utc::now(),
            baggage: std::collections::HashMap::new(),
        }
    }

    /// Create from incoming headers
    pub fn from_headers(headers: &[(String, String)]) -> Self {
        let mut ctx = Self::new();

        for (key, value) in headers {
            match key.to_lowercase().as_str() {
                "x-request-id" | "x-correlation-id" => ctx.request_id = value.clone(),
                "traceparent" => {
                    // Parse W3C traceparent format: version-trace_id-parent_id-flags
                    let parts: Vec<&str> = value.split('-').collect();
                    if parts.len() >= 4 {
                        ctx.trace_id = parts[1].to_string();
                        ctx.parent_span_id = Some(parts[2].to_string());
                    }
                }
                "x-trace-id" => {
                    // Simple trace ID header - use value directly
                    ctx.trace_id = value.clone();
                }
                "x-span-id" => ctx.parent_span_id = Some(value.clone()),
                "x-tenant-id" => ctx.tenant_id = Some(value.clone()),
                "x-user-id" => ctx.user_id = Some(value.clone()),
                "x-session-id" => ctx.session_id = Some(value.clone()),
                key if key.starts_with("baggage-") => {
                    let baggage_key = key.trim_start_matches("baggage-");
                    ctx.baggage.insert(baggage_key.to_string(), value.clone());
                }
                _ => {}
            }
        }

        ctx
    }

    /// Generate a new span ID
    fn generate_span_id() -> String {
        format!("{:016x}", rand::random::<u64>())
    }

    /// Create a child context for a nested span
    pub fn child(&self) -> Self {
        Self {
            request_id: self.request_id.clone(),
            trace_id: self.trace_id.clone(),
            span_id: Self::generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            tenant_id: self.tenant_id.clone(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            baggage: self.baggage.clone(),
        }
    }

    /// Set tenant ID
    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Add baggage item
    pub fn with_baggage(mut self, key: &str, value: &str) -> Self {
        self.baggage.insert(key.to_string(), value.to_string());
        self
    }

    /// Get headers for propagation
    pub fn to_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("X-Request-ID".to_string(), self.request_id.clone()),
            (
                "traceparent".to_string(),
                format!("00-{}-{}-01", self.trace_id, self.span_id),
            ),
        ];

        if let Some(ref tenant_id) = self.tenant_id {
            headers.push(("X-Tenant-ID".to_string(), tenant_id.clone()));
        }

        if let Some(ref user_id) = self.user_id {
            headers.push(("X-User-ID".to_string(), user_id.clone()));
        }

        if let Some(ref session_id) = self.session_id {
            headers.push(("X-Session-ID".to_string(), session_id.clone()));
        }

        for (key, value) in &self.baggage {
            headers.push((format!("baggage-{}", key), value.clone()));
        }

        headers
    }

    /// Get W3C traceparent header
    pub fn traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id, self.span_id)
    }
}

// Thread-local context storage
thread_local! {
    static CURRENT_CONTEXT: std::cell::RefCell<Option<CorrelationContext>> = const { std::cell::RefCell::new(None) };
}

/// Set current context for the thread
pub fn set_current_context(ctx: CorrelationContext) {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ctx);
    });
}

/// Get current context from thread
pub fn get_current_context() -> Option<CorrelationContext> {
    CURRENT_CONTEXT.with(|c| c.borrow().clone())
}

/// Clear current context
pub fn clear_current_context() {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = None;
    });
}

/// Context guard that restores previous context on drop
pub struct ContextGuard {
    previous: Option<CorrelationContext>,
}

impl ContextGuard {
    /// Enter a new context
    pub fn enter(ctx: CorrelationContext) -> Self {
        let previous = get_current_context();
        set_current_context(ctx);
        Self { previous }
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        if let Some(ref ctx) = self.previous {
            set_current_context(ctx.clone());
        } else {
            clear_current_context();
        }
    }
}

/// Context propagator for async tasks
#[derive(Debug, Clone)]
pub struct ContextPropagator {
    contexts: Arc<RwLock<std::collections::HashMap<String, CorrelationContext>>>,
}

impl Default for ContextPropagator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextPropagator {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Store context by request ID
    pub fn store(&self, ctx: &CorrelationContext) {
        self.contexts
            .write()
            .insert(ctx.request_id.clone(), ctx.clone());
    }

    /// Retrieve context by request ID
    pub fn retrieve(&self, request_id: &str) -> Option<CorrelationContext> {
        self.contexts.read().get(request_id).cloned()
    }

    /// Remove context
    pub fn remove(&self, request_id: &str) -> Option<CorrelationContext> {
        self.contexts.write().remove(request_id)
    }

    /// Clear old contexts (cleanup)
    pub fn cleanup(&self, max_age_seconds: i64) {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_seconds);
        self.contexts
            .write()
            .retain(|_, ctx| ctx.timestamp > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_context_new() {
        let ctx = CorrelationContext::new();

        assert!(!ctx.request_id.is_empty());
        assert!(!ctx.trace_id.is_empty());
        assert!(!ctx.span_id.is_empty());
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_correlation_context_child() {
        let parent = CorrelationContext::new();
        let child = parent.child();

        assert_eq!(parent.request_id, child.request_id);
        assert_eq!(parent.trace_id, child.trace_id);
        assert_ne!(parent.span_id, child.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
    }

    #[test]
    fn test_from_headers() {
        let headers = vec![
            ("X-Request-ID".to_string(), "req-123".to_string()),
            ("X-Trace-ID".to_string(), "trace-456".to_string()),
            ("X-Tenant-ID".to_string(), "tenant-789".to_string()),
        ];

        let ctx = CorrelationContext::from_headers(&headers);

        assert_eq!(ctx.request_id, "req-123");
        assert_eq!(ctx.trace_id, "trace-456");
        assert_eq!(ctx.tenant_id, Some("tenant-789".to_string()));
    }

    #[test]
    fn test_to_headers() {
        let ctx = CorrelationContext::new()
            .with_tenant("tenant-1")
            .with_user("user-1")
            .with_baggage("custom", "value");

        let headers = ctx.to_headers();

        assert!(headers.iter().any(|(k, _)| k == "X-Request-ID"));
        assert!(headers.iter().any(|(k, _)| k == "traceparent"));
        assert!(headers.iter().any(|(k, _)| k == "X-Tenant-ID"));
        assert!(headers.iter().any(|(k, _)| k == "baggage-custom"));
    }

    #[test]
    fn test_traceparent_format() {
        let ctx = CorrelationContext::new();
        let traceparent = ctx.traceparent();

        assert!(traceparent.starts_with("00-"));
        assert!(traceparent.ends_with("-01"));
        let parts: Vec<&str> = traceparent.split('-').collect();
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn test_context_guard() {
        let ctx1 = CorrelationContext::new().with_tenant("tenant-1");
        let ctx2 = CorrelationContext::new().with_tenant("tenant-2");

        set_current_context(ctx1.clone());

        {
            let _guard = ContextGuard::enter(ctx2.clone());
            let current = get_current_context().unwrap();
            assert_eq!(current.tenant_id, Some("tenant-2".to_string()));
        }

        // After guard drops, context should be restored
        let current = get_current_context().unwrap();
        assert_eq!(current.tenant_id, Some("tenant-1".to_string()));

        clear_current_context();
    }

    #[test]
    fn test_context_propagator() {
        let propagator = ContextPropagator::new();
        let ctx = CorrelationContext::new().with_tenant("tenant-1");

        propagator.store(&ctx);

        let retrieved = propagator.retrieve(&ctx.request_id).unwrap();
        assert_eq!(retrieved.tenant_id, ctx.tenant_id);

        propagator.remove(&ctx.request_id);
        assert!(propagator.retrieve(&ctx.request_id).is_none());
    }
}
