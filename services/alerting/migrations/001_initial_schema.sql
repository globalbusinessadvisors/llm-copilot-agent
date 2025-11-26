-- Alerting Service Initial Schema
-- Migration: 001_initial_schema

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ===========================================
-- On-Call Users Table
-- ===========================================
CREATE TABLE IF NOT EXISTS on_call_users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    phone VARCHAR(50),
    slack_user_id VARCHAR(50),
    notification_preferences JSONB DEFAULT '{"email": true, "slack": true, "sms": false, "phone": false}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_on_call_users_email ON on_call_users(email);
CREATE INDEX idx_on_call_users_slack ON on_call_users(slack_user_id);

-- ===========================================
-- On-Call Schedules Table
-- ===========================================
CREATE TABLE IF NOT EXISTS on_call_schedules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    timezone VARCHAR(50) DEFAULT 'UTC',
    rotations JSONB DEFAULT '[]',
    overrides JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_on_call_schedules_name ON on_call_schedules(name);

-- ===========================================
-- On-Call Overrides Table
-- ===========================================
CREATE TABLE IF NOT EXISTS on_call_overrides (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    schedule_id UUID NOT NULL REFERENCES on_call_schedules(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES on_call_users(id) ON DELETE CASCADE,
    start_at TIMESTAMPTZ NOT NULL,
    end_at TIMESTAMPTZ NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_override_dates CHECK (end_at > start_at)
);

CREATE INDEX idx_on_call_overrides_schedule ON on_call_overrides(schedule_id);
CREATE INDEX idx_on_call_overrides_user ON on_call_overrides(user_id);
CREATE INDEX idx_on_call_overrides_dates ON on_call_overrides(start_at, end_at);

-- ===========================================
-- Escalation Policies Table
-- ===========================================
CREATE TABLE IF NOT EXISTS escalation_policies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    steps JSONB NOT NULL DEFAULT '[]',
    repeat_after_minutes INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_escalation_policies_name ON escalation_policies(name);

-- ===========================================
-- Alert Rules Table
-- ===========================================
CREATE TABLE IF NOT EXISTS alert_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    condition_type VARCHAR(50) NOT NULL,
    condition JSONB NOT NULL,
    severity VARCHAR(20) NOT NULL DEFAULT 'warning',
    tags JSONB DEFAULT '{}',
    escalation_policy_id UUID REFERENCES escalation_policies(id) ON DELETE SET NULL,
    notification_channels JSONB NOT NULL DEFAULT '["email"]',
    cooldown_period INTEGER DEFAULT 300,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_condition_type CHECK (
        condition_type IN ('threshold', 'rate_of_change', 'absence', 'anomaly')
    ),
    CONSTRAINT valid_severity CHECK (
        severity IN ('info', 'warning', 'error', 'critical')
    )
);

CREATE INDEX idx_alert_rules_name ON alert_rules(name);
CREATE INDEX idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX idx_alert_rules_severity ON alert_rules(severity);
CREATE INDEX idx_alert_rules_escalation_policy ON alert_rules(escalation_policy_id);

-- ===========================================
-- Alerts Table
-- ===========================================
CREATE TABLE IF NOT EXISTS alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rule_id UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    severity VARCHAR(20) NOT NULL DEFAULT 'warning',
    status VARCHAR(20) NOT NULL DEFAULT 'triggered',
    source VARCHAR(255) NOT NULL,
    tags JSONB DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    triggered_at TIMESTAMPTZ DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by VARCHAR(255),
    resolved_at TIMESTAMPTZ,
    resolved_by VARCHAR(255),
    escalation_level INTEGER DEFAULT 0,
    notifications_sent JSONB DEFAULT '[]',

    CONSTRAINT valid_alert_status CHECK (
        status IN ('triggered', 'acknowledged', 'resolved', 'suppressed')
    ),
    CONSTRAINT valid_alert_severity CHECK (
        severity IN ('info', 'warning', 'error', 'critical')
    )
);

CREATE INDEX idx_alerts_rule ON alerts(rule_id);
CREATE INDEX idx_alerts_status ON alerts(status);
CREATE INDEX idx_alerts_severity ON alerts(severity);
CREATE INDEX idx_alerts_triggered_at ON alerts(triggered_at DESC);
CREATE INDEX idx_alerts_source ON alerts(source);
CREATE INDEX idx_alerts_active ON alerts(status) WHERE status IN ('triggered', 'acknowledged');

-- ===========================================
-- Notification Logs Table
-- ===========================================
CREATE TABLE IF NOT EXISTS notification_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_id UUID NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,
    channel VARCHAR(20) NOT NULL,
    recipient VARCHAR(255) NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    sent_at TIMESTAMPTZ DEFAULT NOW(),
    duration_ms INTEGER,

    CONSTRAINT valid_notification_channel CHECK (
        channel IN ('email', 'slack', 'pagerduty', 'webhook', 'sms')
    )
);

CREATE INDEX idx_notification_logs_alert ON notification_logs(alert_id);
CREATE INDEX idx_notification_logs_channel ON notification_logs(channel);
CREATE INDEX idx_notification_logs_sent_at ON notification_logs(sent_at DESC);
CREATE INDEX idx_notification_logs_success ON notification_logs(success);

-- ===========================================
-- Alert Events Table (for audit trail)
-- ===========================================
CREATE TABLE IF NOT EXISTS alert_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_id UUID NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    actor VARCHAR(255),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_event_type CHECK (
        event_type IN (
            'created', 'acknowledged', 'resolved', 'suppressed',
            'escalated', 'notification_sent', 'notification_failed',
            'comment_added', 'status_changed'
        )
    )
);

CREATE INDEX idx_alert_events_alert ON alert_events(alert_id);
CREATE INDEX idx_alert_events_type ON alert_events(event_type);
CREATE INDEX idx_alert_events_created_at ON alert_events(created_at DESC);

-- ===========================================
-- Alert Comments Table
-- ===========================================
CREATE TABLE IF NOT EXISTS alert_comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_id UUID NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_alert_comments_alert ON alert_comments(alert_id);
CREATE INDEX idx_alert_comments_user ON alert_comments(user_id);

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

CREATE TRIGGER update_on_call_users_updated_at
    BEFORE UPDATE ON on_call_users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_on_call_schedules_updated_at
    BEFORE UPDATE ON on_call_schedules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_escalation_policies_updated_at
    BEFORE UPDATE ON escalation_policies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_alert_rules_updated_at
    BEFORE UPDATE ON alert_rules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_alert_comments_updated_at
    BEFORE UPDATE ON alert_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ===========================================
-- Function to record alert events
-- ===========================================

CREATE OR REPLACE FUNCTION record_alert_event()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO alert_events (alert_id, event_type, metadata)
        VALUES (NEW.id, 'created', jsonb_build_object(
            'severity', NEW.severity,
            'source', NEW.source
        ));
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.status != NEW.status THEN
            INSERT INTO alert_events (alert_id, event_type, actor, metadata)
            VALUES (NEW.id, 'status_changed',
                COALESCE(NEW.acknowledged_by, NEW.resolved_by, 'system'),
                jsonb_build_object(
                    'old_status', OLD.status,
                    'new_status', NEW.status
                )
            );
        END IF;

        IF OLD.escalation_level != NEW.escalation_level THEN
            INSERT INTO alert_events (alert_id, event_type, metadata)
            VALUES (NEW.id, 'escalated', jsonb_build_object(
                'old_level', OLD.escalation_level,
                'new_level', NEW.escalation_level
            ));
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER alert_event_trigger
    AFTER INSERT OR UPDATE ON alerts
    FOR EACH ROW
    EXECUTE FUNCTION record_alert_event();

-- ===========================================
-- Sample Data
-- ===========================================

-- Sample on-call users
INSERT INTO on_call_users (name, email, phone, slack_user_id) VALUES
    ('Alice Johnson', 'alice@llm-copilot.com', '+1-555-0101', 'U001'),
    ('Bob Smith', 'bob@llm-copilot.com', '+1-555-0102', 'U002'),
    ('Carol Williams', 'carol@llm-copilot.com', '+1-555-0103', 'U003'),
    ('David Brown', 'david@llm-copilot.com', '+1-555-0104', 'U004');

-- Sample escalation policy
INSERT INTO escalation_policies (id, name, description, steps, repeat_after_minutes) VALUES
    (
        'a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d',
        'Default Escalation Policy',
        'Standard escalation for production alerts',
        '[
            {
                "order": 0,
                "delayMinutes": 5,
                "targets": [
                    {"type": "schedule", "id": "primary-oncall", "channels": ["slack", "email"]}
                ]
            },
            {
                "order": 1,
                "delayMinutes": 15,
                "targets": [
                    {"type": "schedule", "id": "primary-oncall", "channels": ["slack", "email", "sms"]}
                ]
            },
            {
                "order": 2,
                "delayMinutes": 30,
                "targets": [
                    {"type": "schedule", "id": "secondary-oncall", "channels": ["slack", "email", "sms", "pagerduty"]}
                ]
            }
        ]',
        60
    );

-- Sample alert rules
INSERT INTO alert_rules (name, description, condition_type, condition, severity, notification_channels, escalation_policy_id) VALUES
    (
        'High Error Rate',
        'Alert when error rate exceeds 5%',
        'threshold',
        '{"metric": "error_rate", "operator": "gt", "threshold": 5, "duration": 300, "aggregation": "avg"}',
        'error',
        '["slack", "email"]',
        'a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d'
    ),
    (
        'High Latency',
        'Alert when P95 latency exceeds 1000ms',
        'threshold',
        '{"metric": "latency_p95", "operator": "gt", "threshold": 1000, "duration": 180, "aggregation": "avg"}',
        'warning',
        '["slack"]',
        NULL
    ),
    (
        'Service Down',
        'Alert when health check fails',
        'absence',
        '{"metric": "health_check", "operator": "eq", "threshold": 0, "duration": 60}',
        'critical',
        '["slack", "email", "pagerduty"]',
        'a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d'
    ),
    (
        'Memory Usage Critical',
        'Alert when memory usage exceeds 90%',
        'threshold',
        '{"metric": "memory_usage_percent", "operator": "gt", "threshold": 90, "duration": 120, "aggregation": "max"}',
        'critical',
        '["slack", "email", "sms"]',
        'a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d'
    ),
    (
        'CPU Usage High',
        'Alert when CPU usage exceeds 80%',
        'threshold',
        '{"metric": "cpu_usage_percent", "operator": "gt", "threshold": 80, "duration": 300, "aggregation": "avg"}',
        'warning',
        '["slack"]',
        NULL
    );
