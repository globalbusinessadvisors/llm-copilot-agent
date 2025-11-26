-- Billing Service - Tenants Table
-- Migration: 002_add_tenants_table
-- Created: 2024-01-01
--
-- This migration creates a tenants table if it doesn't exist
-- Skip this if your application already has a tenants table

-- Create tenants table if not exists
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    stripe_customer_id VARCHAR(255),
    plan_type VARCHAR(50) DEFAULT 'free',
    status VARCHAR(50) DEFAULT 'active',
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_tenant_status CHECK (
        status IN ('active', 'suspended', 'deleted')
    )
);

-- Create unique index on email
CREATE UNIQUE INDEX IF NOT EXISTS idx_tenants_email ON tenants(email);

-- Create index on stripe_customer_id
CREATE INDEX IF NOT EXISTS idx_tenants_stripe_customer_id ON tenants(stripe_customer_id);

-- Create index on status
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants(status);

-- Apply updated_at trigger
DROP TRIGGER IF EXISTS update_tenants_updated_at ON tenants;
CREATE TRIGGER update_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add foreign key constraints to existing tables
-- These may fail if tenants table already exists with different schema
-- Comment out if not needed

-- ALTER TABLE subscriptions
--     ADD CONSTRAINT fk_subscriptions_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE usage_events
--     ADD CONSTRAINT fk_usage_events_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE usage_quotas
--     ADD CONSTRAINT fk_usage_quotas_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE invoices
--     ADD CONSTRAINT fk_invoices_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE payment_methods
--     ADD CONSTRAINT fk_payment_methods_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- ALTER TABLE billing_events
--     ADD CONSTRAINT fk_billing_events_tenant
--     FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;
