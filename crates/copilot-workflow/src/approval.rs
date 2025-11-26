//! Approval gate implementation for workflow steps

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Status of an approval request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Approval is pending
    Pending,
    /// Approval was granted
    Approved,
    /// Approval was denied
    Denied,
    /// Approval request timed out
    Timeout,
    /// Approval was cancelled
    Cancelled,
}

/// Approval request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique approval ID
    pub id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Step ID requiring approval
    pub step_id: String,
    /// Approval title/summary
    pub title: String,
    /// Detailed description of what is being approved
    pub description: String,
    /// Current status
    pub status: ApprovalStatus,
    /// User/entity that requested approval
    pub requester: String,
    /// User/entity that approved/denied (if applicable)
    pub approver: Option<String>,
    /// Context data for the approval
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Response timestamp
    pub responded_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Approval response message
    pub response_message: Option<String>,
    /// Notification channels to use
    #[serde(default)]
    pub notification_channels: Vec<String>,
}

impl ApprovalRequest {
    /// Create a new approval request
    pub fn new(
        workflow_id: impl Into<String>,
        step_id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        requester: impl Into<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workflow_id: workflow_id.into(),
            step_id: step_id.into(),
            title: title.into(),
            description: description.into(),
            status: ApprovalStatus::Pending,
            requester: requester.into(),
            approver: None,
            context: HashMap::new(),
            timeout_secs,
            created_at: chrono::Utc::now(),
            responded_at: None,
            response_message: None,
            notification_channels: Vec::new(),
        }
    }

    /// Add context data
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Add notification channel
    pub fn with_notification(mut self, channel: impl Into<String>) -> Self {
        self.notification_channels.push(channel.into());
        self
    }

    /// Check if the approval has timed out
    pub fn is_timed_out(&self) -> bool {
        if self.status != ApprovalStatus::Pending {
            return false;
        }

        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds() as u64;

        elapsed >= self.timeout_secs
    }

    /// Approve the request
    pub fn approve(mut self, approver: impl Into<String>, message: Option<String>) -> Self {
        self.status = ApprovalStatus::Approved;
        self.approver = Some(approver.into());
        self.response_message = message;
        self.responded_at = Some(chrono::Utc::now());
        self
    }

    /// Deny the request
    pub fn deny(mut self, approver: impl Into<String>, message: Option<String>) -> Self {
        self.status = ApprovalStatus::Denied;
        self.approver = Some(approver.into());
        self.response_message = message;
        self.responded_at = Some(chrono::Utc::now());
        self
    }

    /// Mark as timed out
    pub fn timeout(mut self) -> Self {
        self.status = ApprovalStatus::Timeout;
        self.responded_at = Some(chrono::Utc::now());
        self
    }

    /// Cancel the request
    pub fn cancel(mut self) -> Self {
        self.status = ApprovalStatus::Cancelled;
        self.responded_at = Some(chrono::Utc::now());
        self
    }
}

/// Approval gate manager
#[derive(Debug, Clone)]
pub struct ApprovalGate {
    /// Pending approval requests
    requests: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalGate {
    /// Create a new approval gate
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request approval for a workflow step
    pub async fn request_approval(&self, request: ApprovalRequest) -> String {
        let id = request.id.clone();
        let mut requests = self.requests.write().await;
        requests.insert(id.clone(), request);

        tracing::info!(
            approval_id = %id,
            "Approval request created"
        );

        id
    }

    /// Check the status of an approval request
    pub async fn check_approval(&self, approval_id: &str) -> Option<ApprovalStatus> {
        let mut requests = self.requests.write().await;

        if let Some(request) = requests.get_mut(approval_id) {
            // Check for timeout
            if request.is_timed_out() && request.status == ApprovalStatus::Pending {
                let timed_out = std::mem::replace(request, request.clone().timeout());
                *request = timed_out;

                tracing::warn!(
                    approval_id = %approval_id,
                    "Approval request timed out"
                );
            }

            Some(request.status.clone())
        } else {
            None
        }
    }

    /// Get an approval request
    pub async fn get_request(&self, approval_id: &str) -> Option<ApprovalRequest> {
        let requests = self.requests.read().await;
        requests.get(approval_id).cloned()
    }

    /// List all pending approval requests
    pub async fn list_pending(&self) -> Vec<ApprovalRequest> {
        let requests = self.requests.read().await;
        requests
            .values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    /// List all approval requests for a workflow
    pub async fn list_for_workflow(&self, workflow_id: &str) -> Vec<ApprovalRequest> {
        let requests = self.requests.read().await;
        requests
            .values()
            .filter(|r| r.workflow_id == workflow_id)
            .cloned()
            .collect()
    }

    /// Approve a request
    pub async fn approve(
        &self,
        approval_id: &str,
        approver: impl Into<String>,
        message: Option<String>,
    ) -> Result<(), String> {
        let mut requests = self.requests.write().await;

        if let Some(request) = requests.get_mut(approval_id) {
            if request.status != ApprovalStatus::Pending {
                return Err(format!("Approval is not pending: {:?}", request.status));
            }

            let approved = std::mem::replace(
                request,
                request.clone().approve(approver, message),
            );
            *request = approved;

            tracing::info!(
                approval_id = %approval_id,
                approver = %request.approver.as_ref().unwrap(),
                "Approval granted"
            );

            Ok(())
        } else {
            Err(format!("Approval request not found: {}", approval_id))
        }
    }

    /// Deny a request
    pub async fn deny(
        &self,
        approval_id: &str,
        approver: impl Into<String>,
        message: Option<String>,
    ) -> Result<(), String> {
        let mut requests = self.requests.write().await;

        if let Some(request) = requests.get_mut(approval_id) {
            if request.status != ApprovalStatus::Pending {
                return Err(format!("Approval is not pending: {:?}", request.status));
            }

            let denied = std::mem::replace(
                request,
                request.clone().deny(approver, message),
            );
            *request = denied;

            tracing::warn!(
                approval_id = %approval_id,
                approver = %request.approver.as_ref().unwrap(),
                "Approval denied"
            );

            Ok(())
        } else {
            Err(format!("Approval request not found: {}", approval_id))
        }
    }

    /// Cancel a request
    pub async fn cancel(&self, approval_id: &str) -> Result<(), String> {
        let mut requests = self.requests.write().await;

        if let Some(request) = requests.get_mut(approval_id) {
            let cancelled = std::mem::replace(request, request.clone().cancel());
            *request = cancelled;

            tracing::info!(
                approval_id = %approval_id,
                "Approval request cancelled"
            );

            Ok(())
        } else {
            Err(format!("Approval request not found: {}", approval_id))
        }
    }

    /// Wait for an approval decision with timeout
    pub async fn wait_for_decision(
        &self,
        approval_id: &str,
        poll_interval_ms: u64,
    ) -> Result<ApprovalStatus, String> {
        loop {
            let status = self.check_approval(approval_id).await
                .ok_or_else(|| format!("Approval request not found: {}", approval_id))?;

            match status {
                ApprovalStatus::Pending => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)).await;
                }
                _ => return Ok(status),
            }
        }
    }

    /// Clean up old completed requests
    pub async fn cleanup_old_requests(&self, max_age_secs: u64) {
        let mut requests = self.requests.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_secs as i64);

        requests.retain(|_, request| {
            request.status == ApprovalStatus::Pending
                || request.responded_at.map_or(false, |t| t > cutoff)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_request_lifecycle() {
        let request = ApprovalRequest::new(
            "wf1",
            "step1",
            "Test Approval",
            "Please approve this test",
            "user1",
            300,
        );

        assert_eq!(request.status, ApprovalStatus::Pending);

        let approved = request.approve("approver1", Some("LGTM".to_string()));
        assert_eq!(approved.status, ApprovalStatus::Approved);
        assert_eq!(approved.approver.as_ref().unwrap(), "approver1");
    }

    #[test]
    fn test_approval_gate_sync() {
        // Test the synchronous operations without async runtime complexity
        let request = ApprovalRequest::new(
            "wf1",
            "step1",
            "Test Approval",
            "Please approve this test",
            "user1",
            3600,
        );

        assert_eq!(request.status, ApprovalStatus::Pending);

        let approved = request.approve("approver1", Some("LGTM".to_string()));
        assert_eq!(approved.status, ApprovalStatus::Approved);
        assert_eq!(approved.approver.as_ref().unwrap(), "approver1");
    }

    #[test]
    fn test_approval_denial() {
        let request = ApprovalRequest::new(
            "wf1",
            "step1",
            "Test Approval",
            "Please approve this test",
            "user1",
            3600,
        );

        let denied = request.deny("reviewer1", Some("Not ready".to_string()));
        assert_eq!(denied.status, ApprovalStatus::Denied);
        assert_eq!(denied.approver.as_ref().unwrap(), "reviewer1");
    }

    #[tokio::test]
    async fn test_approval_timeout() {
        let request = ApprovalRequest::new(
            "wf1",
            "step1",
            "Test Approval",
            "Please approve this test",
            "user1",
            0, // Immediate timeout
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(request.is_timed_out());
    }
}
