-- Billing Service Initial Schema
-- Migration: 001_initial_schema
-- Created: 2024-01-01

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ===========================================
-- Subscriptions Table
-- ===========================================
CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    plan_id VARCHAR(100) NOT NULL,
    plan_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'incomplete',
    stripe_subscription_id VARCHAR(255),
    stripe_customer_id VARCHAR(255),
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end TIMESTAMPTZ NOT NULL,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    canceled_at TIMESTAMPTZ,
    trial_start TIMESTAMPTZ,
    trial_end TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_status CHECK (
        status IN ('active', 'past_due', 'canceled', 'incomplete', 'incomplete_expired', 'trialing', 'paused')
    ),
    CONSTRAINT valid_plan_type CHECK (
        plan_type IN ('free', 'starter', 'professional', 'enterprise', 'custom')
    )
);

-- Indexes for subscriptions
CREATE INDEX idx_subscriptions_tenant_id ON subscriptions(tenant_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);
CREATE INDEX idx_subscriptions_stripe_subscription_id ON subscriptions(stripe_subscription_id);
CREATE INDEX idx_subscriptions_stripe_customer_id ON subscriptions(stripe_customer_id);

-- ===========================================
-- Usage Events Table
-- ===========================================
CREATE TABLE IF NOT EXISTS usage_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    user_id UUID,
    type VARCHAR(50) NOT NULL,
    unit VARCHAR(50) NOT NULL,
    quantity DECIMAL(20, 6) NOT NULL,
    metadata JSONB DEFAULT '{}',
    resource_id VARCHAR(255),
    resource_type VARCHAR(100),
    model VARCHAR(100),
    endpoint VARCHAR(255),
    status_code INTEGER,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    billing_period_start TIMESTAMPTZ NOT NULL,
    billing_period_end TIMESTAMPTZ NOT NULL,

    CONSTRAINT valid_type CHECK (
        type IN ('api_call', 'token_input', 'token_output', 'storage', 'compute', 'embedding', 'workflow_run', 'context_search')
    ),
    CONSTRAINT valid_unit CHECK (
        unit IN ('count', 'tokens', 'bytes', 'seconds', 'milliseconds')
    ),
    CONSTRAINT positive_quantity CHECK (quantity > 0)
);

-- Partitioning by month for better performance (PostgreSQL 10+)
-- For older versions, create manual partitions or use inheritance

-- Indexes for usage_events
CREATE INDEX idx_usage_events_tenant_id ON usage_events(tenant_id);
CREATE INDEX idx_usage_events_timestamp ON usage_events(timestamp);
CREATE INDEX idx_usage_events_type ON usage_events(type);
CREATE INDEX idx_usage_events_tenant_period ON usage_events(tenant_id, billing_period_start, billing_period_end);
CREATE INDEX idx_usage_events_user_id ON usage_events(user_id) WHERE user_id IS NOT NULL;

-- ===========================================
-- Usage Quotas Table
-- ===========================================
CREATE TABLE IF NOT EXISTS usage_quotas (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    api_calls_limit BIGINT,
    input_tokens_limit BIGINT,
    output_tokens_limit BIGINT,
    storage_bytes_limit BIGINT,
    compute_seconds_limit DECIMAL(20, 6),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_tenant_period UNIQUE (tenant_id, period_start)
);

-- Indexes for usage_quotas
CREATE INDEX idx_usage_quotas_tenant_id ON usage_quotas(tenant_id);
CREATE INDEX idx_usage_quotas_period ON usage_quotas(period_start, period_end);

-- ===========================================
-- Invoices Table
-- ===========================================
CREATE TABLE IF NOT EXISTS invoices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    subscription_id UUID REFERENCES subscriptions(id),
    stripe_invoice_id VARCHAR(255),
    number VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    currency VARCHAR(10) DEFAULT 'usd',
    subtotal DECIMAL(12, 2) NOT NULL,
    tax DECIMAL(12, 2) DEFAULT 0,
    total DECIMAL(12, 2) NOT NULL,
    amount_due DECIMAL(12, 2) NOT NULL,
    amount_paid DECIMAL(12, 2) DEFAULT 0,
    amount_remaining DECIMAL(12, 2) NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    due_date TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    hosted_invoice_url TEXT,
    pdf_url TEXT,
    line_items JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_invoice_status CHECK (
        status IN ('draft', 'open', 'paid', 'void', 'uncollectible')
    )
);

-- Indexes for invoices
CREATE INDEX idx_invoices_tenant_id ON invoices(tenant_id);
CREATE INDEX idx_invoices_subscription_id ON invoices(subscription_id);
CREATE INDEX idx_invoices_stripe_invoice_id ON invoices(stripe_invoice_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_created_at ON invoices(created_at DESC);

-- ===========================================
-- Payment Methods Table
-- ===========================================
CREATE TABLE IF NOT EXISTS payment_methods (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    type VARCHAR(50) NOT NULL,
    stripe_payment_method_id VARCHAR(255),
    is_default BOOLEAN DEFAULT FALSE,
    card_brand VARCHAR(50),
    card_last4 VARCHAR(4),
    card_exp_month INTEGER,
    card_exp_year INTEGER,
    bank_name VARCHAR(255),
    bank_last4 VARCHAR(4),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_payment_type CHECK (
        type IN ('card', 'bank_transfer', 'invoice')
    )
);

-- Indexes for payment_methods
CREATE INDEX idx_payment_methods_tenant_id ON payment_methods(tenant_id);
CREATE INDEX idx_payment_methods_stripe_id ON payment_methods(stripe_payment_method_id);
CREATE INDEX idx_payment_methods_default ON payment_methods(tenant_id, is_default) WHERE is_default = TRUE;

-- ===========================================
-- Billing Events Table (Audit Log)
-- ===========================================
CREATE TABLE IF NOT EXISTS billing_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    type VARCHAR(100) NOT NULL,
    tenant_id UUID NOT NULL,
    subscription_id UUID REFERENCES subscriptions(id),
    invoice_id UUID REFERENCES invoices(id),
    data JSONB DEFAULT '{}',
    timestamp TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_event_type CHECK (
        type IN (
            'subscription.created', 'subscription.updated', 'subscription.canceled', 'subscription.renewed',
            'invoice.created', 'invoice.paid', 'invoice.failed',
            'payment.succeeded', 'payment.failed',
            'quota.warning', 'quota.exceeded'
        )
    )
);

-- Indexes for billing_events
CREATE INDEX idx_billing_events_tenant_id ON billing_events(tenant_id);
CREATE INDEX idx_billing_events_type ON billing_events(type);
CREATE INDEX idx_billing_events_timestamp ON billing_events(timestamp DESC);

-- ===========================================
-- Tenants Table Extension
-- ===========================================
-- Add billing-related columns to tenants table if it doesn't have them
-- This assumes a tenants table already exists

-- Note: Run this only if tenants table exists and doesn't have these columns
-- ALTER TABLE tenants ADD COLUMN IF NOT EXISTS stripe_customer_id VARCHAR(255);
-- ALTER TABLE tenants ADD COLUMN IF NOT EXISTS email VARCHAR(255);
-- ALTER TABLE tenants ADD COLUMN IF NOT EXISTS name VARCHAR(255);
-- CREATE INDEX IF NOT EXISTS idx_tenants_stripe_customer_id ON tenants(stripe_customer_id);

-- ===========================================
-- Functions and Triggers
-- ===========================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers
CREATE TRIGGER update_subscriptions_updated_at
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_usage_quotas_updated_at
    BEFORE UPDATE ON usage_quotas
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_invoices_updated_at
    BEFORE UPDATE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payment_methods_updated_at
    BEFORE UPDATE ON payment_methods
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ===========================================
-- Views for Reporting
-- ===========================================

-- Monthly usage summary view
CREATE OR REPLACE VIEW monthly_usage_summary AS
SELECT
    tenant_id,
    DATE_TRUNC('month', timestamp) as month,
    type,
    SUM(quantity) as total_quantity,
    COUNT(*) as event_count
FROM usage_events
GROUP BY tenant_id, DATE_TRUNC('month', timestamp), type;

-- Active subscriptions view
CREATE OR REPLACE VIEW active_subscriptions AS
SELECT
    s.*,
    q.api_calls_limit,
    q.input_tokens_limit,
    q.output_tokens_limit,
    q.storage_bytes_limit,
    q.compute_seconds_limit
FROM subscriptions s
LEFT JOIN usage_quotas q ON s.tenant_id = q.tenant_id
    AND q.period_start <= NOW()
    AND q.period_end > NOW()
WHERE s.status IN ('active', 'trialing');

-- Revenue by plan view
CREATE OR REPLACE VIEW revenue_by_plan AS
SELECT
    s.plan_type,
    COUNT(DISTINCT s.tenant_id) as tenant_count,
    SUM(i.amount_paid) as total_revenue
FROM subscriptions s
LEFT JOIN invoices i ON s.id = i.subscription_id AND i.status = 'paid'
GROUP BY s.plan_type;

-- ===========================================
-- Grants
-- ===========================================
-- Uncomment and modify based on your database user setup
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO billing_service;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO billing_service;
