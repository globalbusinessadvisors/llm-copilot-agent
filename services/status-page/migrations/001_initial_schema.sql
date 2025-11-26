-- Status Page Service Initial Schema
-- Migration: 001_initial_schema

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ===========================================
-- Services Table
-- ===========================================
CREATE TABLE IF NOT EXISTS services (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'operational',
    service_group VARCHAR(100),
    display_order INTEGER DEFAULT 0,
    is_public BOOLEAN DEFAULT TRUE,
    health_check_url TEXT,
    health_check_interval INTEGER DEFAULT 60,
    last_check_at TIMESTAMPTZ,
    last_check_latency INTEGER,
    uptime_percent DECIMAL(5, 2),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_service_status CHECK (
        status IN ('operational', 'degraded', 'partial_outage', 'major_outage', 'maintenance')
    )
);

CREATE INDEX idx_services_slug ON services(slug);
CREATE INDEX idx_services_status ON services(status);
CREATE INDEX idx_services_public ON services(is_public);

-- ===========================================
-- Health Checks Table
-- ===========================================
CREATE TABLE IF NOT EXISTS health_checks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    success BOOLEAN NOT NULL,
    latency INTEGER,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_health_checks_service_id ON health_checks(service_id);
CREATE INDEX idx_health_checks_created_at ON health_checks(created_at);

-- ===========================================
-- Incidents Table
-- ===========================================
CREATE TABLE IF NOT EXISTS incidents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'investigating',
    severity VARCHAR(50) NOT NULL DEFAULT 'minor',
    affected_services JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    postmortem_url TEXT,

    CONSTRAINT valid_incident_status CHECK (
        status IN ('investigating', 'identified', 'monitoring', 'resolved')
    ),
    CONSTRAINT valid_incident_severity CHECK (
        severity IN ('minor', 'major', 'critical')
    )
);

CREATE INDEX idx_incidents_status ON incidents(status);
CREATE INDEX idx_incidents_severity ON incidents(severity);
CREATE INDEX idx_incidents_created_at ON incidents(created_at DESC);

-- ===========================================
-- Incident Updates Table
-- ===========================================
CREATE TABLE IF NOT EXISTS incident_updates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    incident_id UUID NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL
);

CREATE INDEX idx_incident_updates_incident_id ON incident_updates(incident_id);
CREATE INDEX idx_incident_updates_created_at ON incident_updates(created_at DESC);

-- ===========================================
-- Maintenance Windows Table
-- ===========================================
CREATE TABLE IF NOT EXISTS maintenance_windows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'scheduled',
    affected_services JSONB DEFAULT '[]',
    scheduled_start_at TIMESTAMPTZ NOT NULL,
    scheduled_end_at TIMESTAMPTZ NOT NULL,
    actual_start_at TIMESTAMPTZ,
    actual_end_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_maintenance_status CHECK (
        status IN ('scheduled', 'in_progress', 'completed', 'cancelled')
    )
);

CREATE INDEX idx_maintenance_status ON maintenance_windows(status);
CREATE INDEX idx_maintenance_scheduled ON maintenance_windows(scheduled_start_at);

-- ===========================================
-- Daily Uptime Stats Table
-- ===========================================
CREATE TABLE IF NOT EXISTS daily_uptime_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    uptime_percent DECIMAL(5, 2) NOT NULL,
    total_checks INTEGER NOT NULL,
    successful_checks INTEGER NOT NULL,
    avg_latency INTEGER,
    incident_count INTEGER DEFAULT 0,

    CONSTRAINT unique_service_date UNIQUE (service_id, date)
);

CREATE INDEX idx_daily_uptime_service_date ON daily_uptime_stats(service_id, date);

-- ===========================================
-- Subscribers Table (for notifications)
-- ===========================================
CREATE TABLE IF NOT EXISTS status_subscribers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) NOT NULL UNIQUE,
    verified BOOLEAN DEFAULT FALSE,
    verification_token VARCHAR(255),
    subscribed_services JSONB DEFAULT '[]',
    incident_types JSONB DEFAULT '["all"]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    unsubscribed_at TIMESTAMPTZ
);

CREATE INDEX idx_subscribers_email ON status_subscribers(email);
CREATE INDEX idx_subscribers_verified ON status_subscribers(verified);

-- ===========================================
-- Triggers
-- ===========================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_services_updated_at
    BEFORE UPDATE ON services
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_incidents_updated_at
    BEFORE UPDATE ON incidents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_maintenance_updated_at
    BEFORE UPDATE ON maintenance_windows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ===========================================
-- Initial Data
-- ===========================================

INSERT INTO services (name, slug, description, service_group, display_order) VALUES
    ('API Gateway', 'api-gateway', 'Main API endpoint for all services', 'Core', 1),
    ('Authentication', 'auth', 'User authentication and authorization', 'Core', 2),
    ('AI Service', 'ai-service', 'LLM inference and chat completions', 'Core', 3),
    ('Billing Service', 'billing', 'Subscription and payment processing', 'Platform', 4),
    ('Database', 'database', 'Primary database cluster', 'Infrastructure', 5),
    ('Redis Cache', 'redis', 'Caching and session storage', 'Infrastructure', 6),
    ('Webhook Delivery', 'webhooks', 'Outbound webhook delivery', 'Platform', 7),
    ('Dashboard', 'dashboard', 'Web dashboard and portal', 'Platform', 8);
