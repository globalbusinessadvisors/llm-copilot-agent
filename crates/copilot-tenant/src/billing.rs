//! Billing integration
//!
//! Provides billing hooks and integration points.

use crate::{metering::UsageSummary, Result, Tenant, TenantError, TenantTier};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Subscription status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// Active subscription
    Active,
    /// Subscription is trialing
    Trialing,
    /// Payment past due
    PastDue,
    /// Subscription canceled
    Canceled,
    /// Subscription expired
    Expired,
    /// Subscription paused
    Paused,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Trialing => "trialing",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
            Self::Paused => "paused",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Trialing)
    }
}

/// Subscription details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Subscription ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// External subscription ID (e.g., Stripe subscription ID)
    pub external_id: Option<String>,
    /// Plan/tier
    pub tier: TenantTier,
    /// Status
    pub status: SubscriptionStatus,
    /// Billing period start
    pub current_period_start: DateTime<Utc>,
    /// Billing period end
    pub current_period_end: DateTime<Utc>,
    /// Cancel at period end
    pub cancel_at_period_end: bool,
    /// Trial end date
    pub trial_end: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(tenant_id: &str, tier: TenantTier) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            external_id: None,
            tier,
            status: SubscriptionStatus::Active,
            current_period_start: now,
            current_period_end: now + chrono::Duration::days(30),
            cancel_at_period_end: false,
            trial_end: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a trial subscription
    pub fn new_trial(tenant_id: &str, tier: TenantTier, trial_days: u32) -> Self {
        let now = Utc::now();
        let trial_end = now + chrono::Duration::days(trial_days as i64);
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            external_id: None,
            tier,
            status: SubscriptionStatus::Trialing,
            current_period_start: now,
            current_period_end: trial_end,
            cancel_at_period_end: false,
            trial_end: Some(trial_end),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if subscription is active
    pub fn is_active(&self) -> bool {
        self.status.is_active() && Utc::now() < self.current_period_end
    }
}

/// Invoice status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    /// Draft invoice
    Draft,
    /// Open/pending invoice
    Open,
    /// Paid invoice
    Paid,
    /// Void invoice
    Void,
    /// Uncollectible
    Uncollectible,
}

/// Invoice line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    /// Description
    pub description: String,
    /// Quantity
    pub quantity: u64,
    /// Unit price in cents
    pub unit_price_cents: f64,
    /// Total amount in cents
    pub amount_cents: f64,
}

/// Invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// Invoice ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// External invoice ID (e.g., Stripe invoice ID)
    pub external_id: Option<String>,
    /// Invoice number
    pub number: String,
    /// Status
    pub status: InvoiceStatus,
    /// Line items
    pub line_items: Vec<InvoiceLineItem>,
    /// Subtotal in cents
    pub subtotal_cents: f64,
    /// Tax in cents
    pub tax_cents: f64,
    /// Total in cents
    pub total_cents: f64,
    /// Currency
    pub currency: String,
    /// Period start
    pub period_start: DateTime<Utc>,
    /// Period end
    pub period_end: DateTime<Utc>,
    /// Due date
    pub due_date: DateTime<Utc>,
    /// Paid at
    pub paid_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Invoice {
    /// Create from usage summary
    pub fn from_usage(tenant_id: &str, summary: &UsageSummary) -> Self {
        let now = Utc::now();
        let mut line_items = Vec::new();

        for (resource, quantity) in &summary.usage {
            if *quantity > 0 {
                let unit_price = resource.unit_price_cents();
                line_items.push(InvoiceLineItem {
                    description: format!("{} usage", resource.as_str()),
                    quantity: *quantity,
                    unit_price_cents: unit_price,
                    amount_cents: *quantity as f64 * unit_price,
                });
            }
        }

        let subtotal = summary.total_cost_cents;
        let tax = subtotal * 0.0; // Tax calculation would go here

        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            external_id: None,
            number: format!("INV-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            status: InvoiceStatus::Draft,
            line_items,
            subtotal_cents: subtotal,
            tax_cents: tax,
            total_cents: subtotal + tax,
            currency: "USD".to_string(),
            period_start: summary.period_start,
            period_end: summary.period_end,
            due_date: now + chrono::Duration::days(30),
            paid_at: None,
            created_at: now,
        }
    }

    /// Get total in dollars
    pub fn total_dollars(&self) -> f64 {
        self.total_cents / 100.0
    }
}

/// Payment method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    /// Payment method ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// External payment method ID
    pub external_id: Option<String>,
    /// Type (card, bank_account, etc.)
    pub method_type: String,
    /// Last 4 digits (for cards)
    pub last4: Option<String>,
    /// Brand (for cards)
    pub brand: Option<String>,
    /// Expiration month
    pub exp_month: Option<u32>,
    /// Expiration year
    pub exp_year: Option<u32>,
    /// Is default
    pub is_default: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Billing event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BillingEvent {
    /// Subscription created
    SubscriptionCreated { subscription: Subscription },
    /// Subscription updated
    SubscriptionUpdated { subscription: Subscription },
    /// Subscription canceled
    SubscriptionCanceled { subscription_id: String },
    /// Invoice created
    InvoiceCreated { invoice: Invoice },
    /// Invoice paid
    InvoicePaid { invoice_id: String, paid_at: DateTime<Utc> },
    /// Invoice payment failed
    InvoicePaymentFailed { invoice_id: String, error: String },
    /// Payment method added
    PaymentMethodAdded { payment_method: PaymentMethod },
    /// Payment method removed
    PaymentMethodRemoved { payment_method_id: String },
}

/// Billing provider trait for external integrations
#[async_trait]
pub trait BillingProvider: Send + Sync {
    /// Create a customer in the billing system
    async fn create_customer(&self, tenant: &Tenant) -> Result<String>;

    /// Create a subscription
    async fn create_subscription(
        &self,
        tenant_id: &str,
        customer_id: &str,
        tier: TenantTier,
    ) -> Result<Subscription>;

    /// Cancel a subscription
    async fn cancel_subscription(&self, subscription_id: &str) -> Result<()>;

    /// Create an invoice
    async fn create_invoice(&self, invoice: &Invoice) -> Result<String>;

    /// Process a payment
    async fn process_payment(&self, invoice_id: &str) -> Result<()>;

    /// Add a payment method
    async fn add_payment_method(
        &self,
        tenant_id: &str,
        payment_token: &str,
    ) -> Result<PaymentMethod>;

    /// Get billing portal URL
    async fn get_portal_url(&self, tenant_id: &str) -> Result<String>;
}

/// In-memory billing provider for testing
#[derive(Debug, Default)]
pub struct InMemoryBillingProvider {
    customers: Arc<RwLock<HashMap<String, String>>>,
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    invoices: Arc<RwLock<HashMap<String, Invoice>>>,
    payment_methods: Arc<RwLock<HashMap<String, PaymentMethod>>>,
}

impl InMemoryBillingProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BillingProvider for InMemoryBillingProvider {
    async fn create_customer(&self, tenant: &Tenant) -> Result<String> {
        let customer_id = format!("cus_{}", Uuid::new_v4().to_string()[..12].to_string());
        self.customers
            .write()
            .insert(tenant.id.clone(), customer_id.clone());
        info!(tenant_id = %tenant.id, customer_id = %customer_id, "Created billing customer");
        Ok(customer_id)
    }

    async fn create_subscription(
        &self,
        tenant_id: &str,
        _customer_id: &str,
        tier: TenantTier,
    ) -> Result<Subscription> {
        let subscription = Subscription::new(tenant_id, tier);
        self.subscriptions
            .write()
            .insert(subscription.id.clone(), subscription.clone());
        info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription.id,
            tier = ?tier,
            "Created subscription"
        );
        Ok(subscription)
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write();
        if let Some(sub) = subscriptions.get_mut(subscription_id) {
            sub.status = SubscriptionStatus::Canceled;
            sub.cancel_at_period_end = true;
            sub.updated_at = Utc::now();
            info!(subscription_id = %subscription_id, "Canceled subscription");
            Ok(())
        } else {
            Err(TenantError::NotFound(subscription_id.to_string()))
        }
    }

    async fn create_invoice(&self, invoice: &Invoice) -> Result<String> {
        let external_id = format!("inv_{}", Uuid::new_v4().to_string()[..12].to_string());
        let mut inv = invoice.clone();
        inv.external_id = Some(external_id.clone());
        self.invoices.write().insert(inv.id.clone(), inv);
        info!(invoice_id = %invoice.id, external_id = %external_id, "Created invoice");
        Ok(external_id)
    }

    async fn process_payment(&self, invoice_id: &str) -> Result<()> {
        let mut invoices = self.invoices.write();
        if let Some(inv) = invoices.get_mut(invoice_id) {
            inv.status = InvoiceStatus::Paid;
            inv.paid_at = Some(Utc::now());
            info!(invoice_id = %invoice_id, "Processed payment");
            Ok(())
        } else {
            Err(TenantError::NotFound(invoice_id.to_string()))
        }
    }

    async fn add_payment_method(
        &self,
        tenant_id: &str,
        _payment_token: &str,
    ) -> Result<PaymentMethod> {
        let pm = PaymentMethod {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            external_id: Some(format!("pm_{}", Uuid::new_v4().to_string()[..12].to_string())),
            method_type: "card".to_string(),
            last4: Some("4242".to_string()),
            brand: Some("visa".to_string()),
            exp_month: Some(12),
            exp_year: Some(2025),
            is_default: true,
            created_at: Utc::now(),
        };
        self.payment_methods.write().insert(pm.id.clone(), pm.clone());
        info!(tenant_id = %tenant_id, payment_method_id = %pm.id, "Added payment method");
        Ok(pm)
    }

    async fn get_portal_url(&self, tenant_id: &str) -> Result<String> {
        debug!(tenant_id = %tenant_id, "Generated billing portal URL");
        Ok(format!(
            "https://billing.example.com/portal?tenant={}",
            tenant_id
        ))
    }
}

/// Billing service that coordinates billing operations
pub struct BillingService {
    provider: Arc<dyn BillingProvider>,
    events: Arc<RwLock<Vec<BillingEvent>>>,
}

impl BillingService {
    pub fn new(provider: Arc<dyn BillingProvider>) -> Self {
        Self {
            provider,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create in-memory billing service for testing
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryBillingProvider::new()))
    }

    /// Setup billing for a new tenant
    pub async fn setup_tenant(&self, tenant: &Tenant) -> Result<Subscription> {
        // Create customer in billing system
        let _customer_id = self.provider.create_customer(tenant).await?;

        // Create subscription
        let subscription = self
            .provider
            .create_subscription(&tenant.id, &tenant.id, tenant.tier)
            .await?;

        self.emit_event(BillingEvent::SubscriptionCreated {
            subscription: subscription.clone(),
        });

        Ok(subscription)
    }

    /// Cancel subscription for a tenant
    pub async fn cancel_subscription(&self, subscription_id: &str) -> Result<()> {
        self.provider.cancel_subscription(subscription_id).await?;
        self.emit_event(BillingEvent::SubscriptionCanceled {
            subscription_id: subscription_id.to_string(),
        });
        Ok(())
    }

    /// Generate invoice from usage
    pub async fn generate_invoice(&self, tenant_id: &str, summary: &UsageSummary) -> Result<Invoice> {
        let mut invoice = Invoice::from_usage(tenant_id, summary);
        let external_id = self.provider.create_invoice(&invoice).await?;
        invoice.external_id = Some(external_id);
        invoice.status = InvoiceStatus::Open;

        self.emit_event(BillingEvent::InvoiceCreated {
            invoice: invoice.clone(),
        });

        Ok(invoice)
    }

    /// Process payment for an invoice
    pub async fn process_payment(&self, invoice_id: &str) -> Result<()> {
        self.provider.process_payment(invoice_id).await?;
        self.emit_event(BillingEvent::InvoicePaid {
            invoice_id: invoice_id.to_string(),
            paid_at: Utc::now(),
        });
        Ok(())
    }

    /// Add payment method
    pub async fn add_payment_method(
        &self,
        tenant_id: &str,
        payment_token: &str,
    ) -> Result<PaymentMethod> {
        let pm = self
            .provider
            .add_payment_method(tenant_id, payment_token)
            .await?;
        self.emit_event(BillingEvent::PaymentMethodAdded {
            payment_method: pm.clone(),
        });
        Ok(pm)
    }

    /// Get billing portal URL
    pub async fn get_portal_url(&self, tenant_id: &str) -> Result<String> {
        self.provider.get_portal_url(tenant_id).await
    }

    /// Emit a billing event
    fn emit_event(&self, event: BillingEvent) {
        debug!(event = ?event, "Billing event");
        self.events.write().push(event);
    }

    /// Get recent billing events
    pub fn get_events(&self) -> Vec<BillingEvent> {
        self.events.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metering::MeteredResource;

    #[test]
    fn test_subscription() {
        let sub = Subscription::new("tenant-1", TenantTier::Professional);

        assert!(sub.is_active());
        assert_eq!(sub.status, SubscriptionStatus::Active);
    }

    #[test]
    fn test_trial_subscription() {
        let sub = Subscription::new_trial("tenant-1", TenantTier::Professional, 14);

        assert!(sub.is_active());
        assert_eq!(sub.status, SubscriptionStatus::Trialing);
        assert!(sub.trial_end.is_some());
    }

    #[test]
    fn test_invoice_from_usage() {
        let now = Utc::now();
        let mut summary = UsageSummary::new("tenant-1", now - chrono::Duration::days(30), now);
        summary.add_usage(MeteredResource::ApiCalls, 1000);
        summary.add_usage(MeteredResource::InputTokens, 50000);

        let invoice = Invoice::from_usage("tenant-1", &summary);

        assert_eq!(invoice.tenant_id, "tenant-1");
        assert!(!invoice.line_items.is_empty());
        assert!(invoice.total_cents > 0.0);
    }

    #[tokio::test]
    async fn test_billing_service() {
        let service = BillingService::in_memory();
        let tenant = Tenant::new("Test", "test", "owner", TenantTier::Professional);

        // Setup tenant
        let subscription = service.setup_tenant(&tenant).await.unwrap();
        assert!(subscription.is_active());

        // Add payment method
        let pm = service
            .add_payment_method(&tenant.id, "tok_test")
            .await
            .unwrap();
        assert_eq!(pm.method_type, "card");

        // Get portal URL
        let url = service.get_portal_url(&tenant.id).await.unwrap();
        assert!(url.contains(&tenant.id));

        // Check events
        let events = service.get_events();
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_invoice_generation() {
        let service = BillingService::in_memory();
        let now = Utc::now();

        let mut summary = UsageSummary::new("tenant-1", now - chrono::Duration::days(30), now);
        summary.add_usage(MeteredResource::ApiCalls, 5000);
        summary.add_usage(MeteredResource::TotalTokens, 100000);

        let invoice = service
            .generate_invoice("tenant-1", &summary)
            .await
            .unwrap();

        assert_eq!(invoice.status, InvoiceStatus::Open);
        assert!(invoice.external_id.is_some());

        // Process payment
        service.process_payment(&invoice.id).await.unwrap();
    }
}
