-- Support Service Initial Schema
-- Migration: 001_initial_schema

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ===========================================
-- Tickets Table
-- ===========================================
CREATE TABLE IF NOT EXISTS tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_number VARCHAR(20) NOT NULL UNIQUE,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    assignee_id UUID,
    subject VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    priority VARCHAR(20) NOT NULL DEFAULT 'medium',
    category VARCHAR(50) NOT NULL,
    tags JSONB DEFAULT '[]',
    metadata JSONB DEFAULT '{}',
    first_response_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    satisfaction_rating INTEGER,
    satisfaction_comment TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_ticket_status CHECK (
        status IN ('open', 'pending', 'in_progress', 'waiting_on_customer', 'resolved', 'closed')
    ),
    CONSTRAINT valid_ticket_priority CHECK (
        priority IN ('low', 'medium', 'high', 'urgent')
    ),
    CONSTRAINT valid_ticket_category CHECK (
        category IN ('general', 'billing', 'technical', 'account', 'feature_request', 'bug_report', 'security', 'other')
    ),
    CONSTRAINT valid_satisfaction_rating CHECK (
        satisfaction_rating IS NULL OR (satisfaction_rating >= 1 AND satisfaction_rating <= 5)
    )
);

CREATE INDEX idx_tickets_tenant ON tickets(tenant_id);
CREATE INDEX idx_tickets_user ON tickets(user_id);
CREATE INDEX idx_tickets_assignee ON tickets(assignee_id);
CREATE INDEX idx_tickets_status ON tickets(status);
CREATE INDEX idx_tickets_priority ON tickets(priority);
CREATE INDEX idx_tickets_category ON tickets(category);
CREATE INDEX idx_tickets_created_at ON tickets(created_at DESC);
CREATE INDEX idx_tickets_open ON tickets(status, priority) WHERE status NOT IN ('resolved', 'closed');

-- ===========================================
-- Ticket Messages Table
-- ===========================================
CREATE TABLE IF NOT EXISTS ticket_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    content TEXT NOT NULL,
    is_staff BOOLEAN DEFAULT FALSE,
    is_internal BOOLEAN DEFAULT FALSE,
    attachments JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_ticket_messages_ticket ON ticket_messages(ticket_id);
CREATE INDEX idx_ticket_messages_created_at ON ticket_messages(created_at);
CREATE INDEX idx_ticket_messages_internal ON ticket_messages(ticket_id, is_internal);

-- ===========================================
-- Ticket Activity Table
-- ===========================================
CREATE TABLE IF NOT EXISTS ticket_activity (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    activity_type VARCHAR(50) NOT NULL,
    description TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_activity_type CHECK (
        activity_type IN (
            'created', 'status_changed', 'priority_changed', 'assigned',
            'unassigned', 'message_added', 'internal_note_added', 'category_changed',
            'escalated', 'merged', 'split', 'resolved', 'closed', 'reopened'
        )
    )
);

CREATE INDEX idx_ticket_activity_ticket ON ticket_activity(ticket_id);
CREATE INDEX idx_ticket_activity_created_at ON ticket_activity(created_at DESC);

-- ===========================================
-- Articles Table (Knowledge Base)
-- ===========================================
CREATE TABLE IF NOT EXISTS articles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    content TEXT NOT NULL,
    excerpt VARCHAR(500),
    category VARCHAR(50) NOT NULL,
    tags JSONB DEFAULT '[]',
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    author_id UUID NOT NULL,
    views INTEGER DEFAULT 0,
    helpful_count INTEGER DEFAULT 0,
    not_helpful_count INTEGER DEFAULT 0,
    related_articles JSONB DEFAULT '[]',
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_article_status CHECK (
        status IN ('draft', 'published', 'archived')
    ),
    CONSTRAINT valid_article_category CHECK (
        category IN (
            'getting_started', 'api_reference', 'tutorials', 'troubleshooting',
            'billing', 'account', 'security', 'integrations', 'faq', 'release_notes'
        )
    )
);

CREATE INDEX idx_articles_slug ON articles(slug);
CREATE INDEX idx_articles_category ON articles(category);
CREATE INDEX idx_articles_status ON articles(status);
CREATE INDEX idx_articles_author ON articles(author_id);
CREATE INDEX idx_articles_published ON articles(published_at DESC) WHERE status = 'published';
CREATE INDEX idx_articles_views ON articles(views DESC) WHERE status = 'published';

-- Full-text search index
CREATE INDEX idx_articles_search ON articles USING gin(to_tsvector('english', title || ' ' || content));

-- ===========================================
-- Article Feedback Table
-- ===========================================
CREATE TABLE IF NOT EXISTS article_feedback (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    article_id UUID NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    helpful BOOLEAN NOT NULL,
    comment TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_article_feedback_article ON article_feedback(article_id);
CREATE INDEX idx_article_feedback_helpful ON article_feedback(helpful);

-- ===========================================
-- Canned Responses Table
-- ===========================================
CREATE TABLE IF NOT EXISTS canned_responses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    content TEXT NOT NULL,
    category VARCHAR(50),
    shortcut VARCHAR(50) UNIQUE,
    is_active BOOLEAN DEFAULT TRUE,
    usage_count INTEGER DEFAULT 0,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_canned_responses_category ON canned_responses(category);
CREATE INDEX idx_canned_responses_shortcut ON canned_responses(shortcut);
CREATE INDEX idx_canned_responses_active ON canned_responses(is_active);

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

CREATE TRIGGER update_tickets_updated_at
    BEFORE UPDATE ON tickets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_articles_updated_at
    BEFORE UPDATE ON articles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_canned_responses_updated_at
    BEFORE UPDATE ON canned_responses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ===========================================
-- Function to generate ticket number
-- ===========================================

CREATE OR REPLACE FUNCTION generate_ticket_number()
RETURNS TRIGGER AS $$
DECLARE
    year_code VARCHAR(2);
    sequence_num INTEGER;
BEGIN
    year_code := TO_CHAR(NOW(), 'YY');

    SELECT COALESCE(MAX(CAST(SUBSTRING(ticket_number FROM 4) AS INTEGER)), 0) + 1
    INTO sequence_num
    FROM tickets
    WHERE ticket_number LIKE year_code || '-%';

    NEW.ticket_number := year_code || '-' || LPAD(sequence_num::TEXT, 6, '0');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_ticket_number
    BEFORE INSERT ON tickets
    FOR EACH ROW
    WHEN (NEW.ticket_number IS NULL)
    EXECUTE FUNCTION generate_ticket_number();

-- ===========================================
-- Function to track ticket activity
-- ===========================================

CREATE OR REPLACE FUNCTION record_ticket_activity()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO ticket_activity (ticket_id, user_id, activity_type, description)
        VALUES (NEW.id, NEW.user_id, 'created', 'Ticket created');
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.status != NEW.status THEN
            INSERT INTO ticket_activity (ticket_id, user_id, activity_type, description, metadata)
            VALUES (
                NEW.id,
                COALESCE(NEW.assignee_id, NEW.user_id),
                CASE
                    WHEN NEW.status = 'resolved' THEN 'resolved'
                    WHEN NEW.status = 'closed' THEN 'closed'
                    WHEN OLD.status IN ('resolved', 'closed') AND NEW.status = 'open' THEN 'reopened'
                    ELSE 'status_changed'
                END,
                'Status changed from ' || OLD.status || ' to ' || NEW.status,
                jsonb_build_object('old_status', OLD.status, 'new_status', NEW.status)
            );
        END IF;

        IF OLD.priority != NEW.priority THEN
            INSERT INTO ticket_activity (ticket_id, user_id, activity_type, description, metadata)
            VALUES (
                NEW.id,
                COALESCE(NEW.assignee_id, NEW.user_id),
                'priority_changed',
                'Priority changed from ' || OLD.priority || ' to ' || NEW.priority,
                jsonb_build_object('old_priority', OLD.priority, 'new_priority', NEW.priority)
            );
        END IF;

        IF OLD.assignee_id IS DISTINCT FROM NEW.assignee_id THEN
            INSERT INTO ticket_activity (ticket_id, user_id, activity_type, description, metadata)
            VALUES (
                NEW.id,
                COALESCE(NEW.assignee_id, NEW.user_id),
                CASE WHEN NEW.assignee_id IS NULL THEN 'unassigned' ELSE 'assigned' END,
                CASE
                    WHEN NEW.assignee_id IS NULL THEN 'Ticket unassigned'
                    ELSE 'Ticket assigned'
                END,
                jsonb_build_object('old_assignee', OLD.assignee_id, 'new_assignee', NEW.assignee_id)
            );
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER ticket_activity_trigger
    AFTER INSERT OR UPDATE ON tickets
    FOR EACH ROW
    EXECUTE FUNCTION record_ticket_activity();

-- ===========================================
-- Sample Knowledge Base Articles
-- ===========================================

INSERT INTO articles (title, slug, content, excerpt, category, tags, status, author_id, published_at) VALUES
(
    'Getting Started with LLM CoPilot',
    'getting-started',
    '# Getting Started with LLM CoPilot

Welcome to LLM CoPilot! This guide will help you get up and running quickly.

## Step 1: Create Your Account

Visit our signup page and create your account using your email address or SSO provider.

## Step 2: Get Your API Key

After signing in, navigate to the API Keys section in your dashboard to generate your first API key.

## Step 3: Make Your First API Call

Use your API key to make your first request:

```bash
curl -X POST https://api.llm-copilot.com/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d ''{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}]}''
```

## Next Steps

- Explore our API documentation
- Set up usage alerts
- Configure webhooks for real-time updates',
    'Get started with LLM CoPilot in minutes. Learn how to create an account, get your API key, and make your first API call.',
    'getting_started',
    '["quickstart", "setup", "api-key"]',
    'published',
    '00000000-0000-0000-0000-000000000001',
    NOW()
),
(
    'API Authentication Guide',
    'api-authentication',
    '# API Authentication

Learn how to authenticate your API requests securely.

## API Keys

API keys are the primary method of authentication. Each key has configurable permissions and rate limits.

### Creating API Keys

1. Navigate to Settings > API Keys
2. Click "Create New Key"
3. Select permissions and rate limits
4. Copy and securely store your key

### Using API Keys

Include your API key in the Authorization header:

```
Authorization: Bearer sk_live_xxxxxxxxxxxx
```

## OAuth 2.0

For applications requiring user authorization, we support OAuth 2.0 flows.

## Best Practices

- Never expose API keys in client-side code
- Use environment variables for key storage
- Rotate keys regularly
- Use separate keys for development and production',
    'Comprehensive guide to authenticating your API requests using API keys and OAuth 2.0.',
    'api_reference',
    '["authentication", "api-keys", "oauth", "security"]',
    'published',
    '00000000-0000-0000-0000-000000000001',
    NOW()
),
(
    'Understanding Your Bill',
    'understanding-your-bill',
    '# Understanding Your Bill

Learn how billing works and how to read your invoices.

## Usage-Based Pricing

Our pricing is based on actual usage:

- **API Calls**: Charged per request
- **Tokens**: Input and output tokens are priced separately
- **Storage**: Charged per GB/month
- **Compute**: Charged per minute of processing time

## Invoice Breakdown

Each invoice includes:

1. Usage summary by category
2. Detailed line items
3. Any applicable discounts
4. Tax calculations

## Payment Methods

We accept:
- Credit/Debit cards
- ACH bank transfers (US only)
- Wire transfers (Enterprise)

## Setting Up Billing Alerts

1. Go to Settings > Billing
2. Click "Add Alert"
3. Set threshold and notification preferences',
    'Learn how to understand your LLM CoPilot bill, including usage-based pricing and invoice details.',
    'billing',
    '["billing", "invoices", "pricing", "payments"]',
    'published',
    '00000000-0000-0000-0000-000000000001',
    NOW()
),
(
    'Troubleshooting API Errors',
    'troubleshooting-api-errors',
    '# Troubleshooting API Errors

Common API errors and how to resolve them.

## HTTP Status Codes

### 400 Bad Request
Your request was malformed. Check the request body and parameters.

### 401 Unauthorized
Invalid or missing API key. Verify your credentials.

### 403 Forbidden
Your API key doesn''t have permission for this operation.

### 429 Rate Limited
You''ve exceeded your rate limit. Implement exponential backoff.

### 500 Internal Server Error
Server-side issue. Retry with exponential backoff.

## Common Issues

### "Invalid API Key"
- Verify the key is correct and active
- Check for extra whitespace
- Ensure you''re using the right environment

### "Rate Limit Exceeded"
- Implement request queuing
- Use batch endpoints when available
- Contact support for limit increases

### Timeout Errors
- Increase request timeout
- Check network connectivity
- Use streaming for long operations',
    'Learn how to diagnose and fix common API errors including authentication issues and rate limiting.',
    'troubleshooting',
    '["errors", "debugging", "api", "troubleshooting"]',
    'published',
    '00000000-0000-0000-0000-000000000001',
    NOW()
);

-- ===========================================
-- Sample Canned Responses
-- ===========================================

INSERT INTO canned_responses (name, content, category, shortcut, created_by) VALUES
(
    'Welcome Response',
    'Thank you for contacting LLM CoPilot Support! I''m happy to help you today. Could you please provide more details about your issue?',
    'general',
    '/welcome',
    '00000000-0000-0000-0000-000000000001'
),
(
    'Request API Key Info',
    'To help you with your API key issue, could you please provide the following information:\n\n1. The first 8 characters of your API key (e.g., sk_live_xxxx)\n2. When you last successfully used the key\n3. The exact error message you''re seeing',
    'technical',
    '/apikey',
    '00000000-0000-0000-0000-000000000001'
),
(
    'Billing Question Response',
    'I understand you have a billing question. I''ll be happy to help! Could you please provide:\n\n1. Your account email address\n2. The invoice number (if applicable)\n3. A brief description of your billing concern',
    'billing',
    '/billing',
    '00000000-0000-0000-0000-000000000001'
),
(
    'Ticket Resolution',
    'Great news! Based on our conversation, it looks like your issue has been resolved. Is there anything else I can help you with?\n\nIf everything looks good, I''ll go ahead and close this ticket. You can always reply to reopen it if needed.',
    'general',
    '/resolved',
    '00000000-0000-0000-0000-000000000001'
);
