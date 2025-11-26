-- AI Platform Initial Schema Migration
-- Version: 001
-- Description: Creates all tables for model management, agents, tools, and RAG

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector"; -- For pgvector

-- ===========================================
-- Model Management Tables
-- ===========================================

-- Models table
CREATE TABLE IF NOT EXISTS models (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    provider VARCHAR(50) NOT NULL,
    type VARCHAR(50) NOT NULL,
    model_id VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    capabilities JSONB NOT NULL DEFAULT '{}',
    config JSONB NOT NULL DEFAULT '{}',
    pricing JSONB DEFAULT '{}',
    rate_limits JSONB DEFAULT '{}',
    tags TEXT[] DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT models_provider_check CHECK (provider IN ('openai', 'anthropic', 'groq', 'cohere', 'huggingface', 'azure_openai', 'bedrock', 'custom')),
    CONSTRAINT models_status_check CHECK (status IN ('draft', 'testing', 'staging', 'production', 'deprecated', 'disabled'))
);

CREATE INDEX idx_models_provider ON models(provider);
CREATE INDEX idx_models_status ON models(status);
CREATE INDEX idx_models_type ON models(type);
CREATE INDEX idx_models_tags ON models USING GIN(tags);

-- Model versions table
CREATE TABLE IF NOT EXISTS model_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    changelog TEXT,
    config JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT false,
    benchmarks JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    UNIQUE(model_id, version)
);

CREATE INDEX idx_model_versions_model_id ON model_versions(model_id);
CREATE INDEX idx_model_versions_is_active ON model_versions(is_active);

-- Model deployments table
CREATE TABLE IF NOT EXISTS model_deployments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    version_id UUID NOT NULL REFERENCES model_versions(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    strategy VARCHAR(50) NOT NULL DEFAULT 'rolling',
    traffic_percentage INTEGER DEFAULT 100,
    rollout_config JSONB DEFAULT '{}',
    previous_version_id UUID REFERENCES model_versions(id),
    deployed_at TIMESTAMP,
    rolled_back_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT deployments_status_check CHECK (status IN ('pending', 'deploying', 'active', 'rolling_back', 'rolled_back', 'failed')),
    CONSTRAINT deployments_strategy_check CHECK (strategy IN ('rolling', 'blue_green', 'canary', 'shadow'))
);

CREATE INDEX idx_model_deployments_model_id ON model_deployments(model_id);
CREATE INDEX idx_model_deployments_status ON model_deployments(status);

-- Model metrics table
CREATE TABLE IF NOT EXISTS model_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    request_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    avg_latency_ms NUMERIC DEFAULT 0,
    p50_latency_ms NUMERIC DEFAULT 0,
    p95_latency_ms NUMERIC DEFAULT 0,
    p99_latency_ms NUMERIC DEFAULT 0,
    cost NUMERIC DEFAULT 0,
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_model_metrics_model_id ON model_metrics(model_id);
CREATE INDEX idx_model_metrics_timestamp ON model_metrics(timestamp);

-- A/B Tests table
CREATE TABLE IF NOT EXISTS ab_tests (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    variants JSONB NOT NULL DEFAULT '[]',
    targeting JSONB DEFAULT '{}',
    primary_metric VARCHAR(100) NOT NULL,
    secondary_metrics TEXT[] DEFAULT '{}',
    confidence_level NUMERIC DEFAULT 0.95,
    minimum_sample_size INTEGER DEFAULT 1000,
    results JSONB DEFAULT '{}',
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT ab_tests_status_check CHECK (status IN ('draft', 'running', 'paused', 'completed', 'cancelled'))
);

CREATE INDEX idx_ab_tests_model_id ON ab_tests(model_id);
CREATE INDEX idx_ab_tests_status ON ab_tests(status);

-- A/B Test samples table
CREATE TABLE IF NOT EXISTS ab_test_samples (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    test_id UUID NOT NULL REFERENCES ab_tests(id) ON DELETE CASCADE,
    variant_id VARCHAR(255) NOT NULL,
    metrics JSONB NOT NULL DEFAULT '{}',
    context JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ab_test_samples_test_id ON ab_test_samples(test_id);
CREATE INDEX idx_ab_test_samples_variant_id ON ab_test_samples(variant_id);

-- Fine-tune jobs table
CREATE TABLE IF NOT EXISTS fine_tune_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    base_model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    training_data JSONB NOT NULL DEFAULT '{}',
    hyperparameters JSONB NOT NULL DEFAULT '{}',
    progress JSONB DEFAULT '{}',
    result_model_id UUID REFERENCES models(id),
    provider_job_id VARCHAR(255),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    estimated_completion TIMESTAMP,
    estimated_cost NUMERIC,
    actual_cost NUMERIC,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT fine_tune_jobs_status_check CHECK (status IN ('pending', 'preparing', 'training', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX idx_fine_tune_jobs_base_model_id ON fine_tune_jobs(base_model_id);
CREATE INDEX idx_fine_tune_jobs_status ON fine_tune_jobs(status);

-- ===========================================
-- Agent Management Tables
-- ===========================================

-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL DEFAULT 'assistant',
    model_config JSONB NOT NULL DEFAULT '{}',
    system_prompt TEXT,
    tools TEXT[] DEFAULT '{}',
    capabilities JSONB DEFAULT '{}',
    memory JSONB DEFAULT '{}',
    behavior JSONB DEFAULT '{}',
    constraints JSONB DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT agents_type_check CHECK (type IN ('assistant', 'researcher', 'coder', 'reviewer', 'planner', 'executor', 'supervisor', 'custom')),
    CONSTRAINT agents_status_check CHECK (status IN ('active', 'inactive', 'archived'))
);

CREATE INDEX idx_agents_type ON agents(type);
CREATE INDEX idx_agents_status ON agents(status);

-- Agent executions table
CREATE TABLE IF NOT EXISTS agent_executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'running',
    input JSONB NOT NULL,
    output JSONB,
    error TEXT,
    steps JSONB DEFAULT '[]',
    tokens_used JSONB DEFAULT '{}',
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT agent_executions_status_check CHECK (status IN ('running', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX idx_agent_executions_agent_id ON agent_executions(agent_id);
CREATE INDEX idx_agent_executions_status ON agent_executions(status);
CREATE INDEX idx_agent_executions_started_at ON agent_executions(started_at);

-- Tools table
CREATE TABLE IF NOT EXISTS tools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    type VARCHAR(50) NOT NULL DEFAULT 'function',
    parameters JSONB NOT NULL DEFAULT '{}',
    execution JSONB DEFAULT '{}',
    permissions JSONB DEFAULT '{}',
    rate_limit JSONB DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT tools_type_check CHECK (type IN ('function', 'api', 'database', 'file_system', 'code_execution', 'search', 'browser', 'custom')),
    CONSTRAINT tools_status_check CHECK (status IN ('active', 'inactive', 'deprecated'))
);

CREATE INDEX idx_tools_type ON tools(type);
CREATE INDEX idx_tools_status ON tools(status);

-- Tool calls table
CREATE TABLE IF NOT EXISTS tool_calls (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    execution_id UUID REFERENCES agent_executions(id) ON DELETE SET NULL,
    args JSONB NOT NULL DEFAULT '{}',
    result JSONB,
    error TEXT,
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    latency_ms INTEGER
);

CREATE INDEX idx_tool_calls_tool_id ON tool_calls(tool_id);
CREATE INDEX idx_tool_calls_execution_id ON tool_calls(execution_id);
CREATE INDEX idx_tool_calls_started_at ON tool_calls(started_at);

-- Agent teams table
CREATE TABLE IF NOT EXISTS agent_teams (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    members JSONB NOT NULL DEFAULT '[]',
    collaboration_pattern VARCHAR(50) NOT NULL DEFAULT 'sequential',
    coordination_config JSONB DEFAULT '{}',
    shared_memory JSONB DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT agent_teams_pattern_check CHECK (collaboration_pattern IN ('sequential', 'parallel', 'hierarchical', 'debate', 'consensus', 'supervisor')),
    CONSTRAINT agent_teams_status_check CHECK (status IN ('active', 'inactive', 'archived'))
);

CREATE INDEX idx_agent_teams_status ON agent_teams(status);
CREATE INDEX idx_agent_teams_pattern ON agent_teams(collaboration_pattern);

-- Team executions table
CREATE TABLE IF NOT EXISTS team_executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES agent_teams(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'running',
    input JSONB NOT NULL,
    output JSONB,
    error TEXT,
    agent_executions TEXT[] DEFAULT '{}',
    messages JSONB DEFAULT '[]',
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT team_executions_status_check CHECK (status IN ('running', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX idx_team_executions_team_id ON team_executions(team_id);
CREATE INDEX idx_team_executions_status ON team_executions(status);

-- ===========================================
-- RAG Tables
-- ===========================================

-- RAG collections table
CREATE TABLE IF NOT EXISTS rag_collections (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    embedding_config JSONB NOT NULL DEFAULT '{}',
    vector_store_config JSONB NOT NULL DEFAULT '{}',
    chunking_config JSONB NOT NULL DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    document_count INTEGER DEFAULT 0,
    total_chunks INTEGER DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT rag_collections_status_check CHECK (status IN ('active', 'inactive', 'archived'))
);

CREATE INDEX idx_rag_collections_status ON rag_collections(status);

-- RAG documents table
CREATE TABLE IF NOT EXISTS rag_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    collection_id UUID NOT NULL REFERENCES rag_collections(id) ON DELETE CASCADE,
    title VARCHAR(500),
    content TEXT NOT NULL,
    content_type VARCHAR(100) DEFAULT 'text/plain',
    source JSONB DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    content_hash VARCHAR(64) NOT NULL,
    chunk_count INTEGER DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'processing',
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT rag_documents_status_check CHECK (status IN ('processing', 'ready', 'failed'))
);

CREATE INDEX idx_rag_documents_collection_id ON rag_documents(collection_id);
CREATE INDEX idx_rag_documents_status ON rag_documents(status);
CREATE INDEX idx_rag_documents_content_hash ON rag_documents(content_hash);

-- RAG document chunks table
CREATE TABLE IF NOT EXISTS rag_document_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES rag_documents(id) ON DELETE CASCADE,
    collection_id UUID NOT NULL REFERENCES rag_collections(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    embedding JSONB, -- Store as JSON for portability, or use vector type if pgvector
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rag_document_chunks_document_id ON rag_document_chunks(document_id);
CREATE INDEX idx_rag_document_chunks_collection_id ON rag_document_chunks(collection_id);

-- Full-text search index on chunks
CREATE INDEX idx_rag_document_chunks_content_fts ON rag_document_chunks
    USING GIN(to_tsvector('english', content));

-- RAG pipelines table
CREATE TABLE IF NOT EXISTS rag_pipelines (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    collection_ids TEXT[] NOT NULL DEFAULT '{}',
    retrieval_config JSONB NOT NULL DEFAULT '{}',
    generation_config JSONB NOT NULL DEFAULT '{}',
    attribution JSONB DEFAULT '{}',
    guardrails JSONB DEFAULT '{}',
    caching JSONB DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    CONSTRAINT rag_pipelines_status_check CHECK (status IN ('active', 'inactive', 'archived'))
);

CREATE INDEX idx_rag_pipelines_status ON rag_pipelines(status);

-- ===========================================
-- Audit and Logging
-- ===========================================

-- Audit log table
CREATE TABLE IF NOT EXISTS ai_platform_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID,
    user_id VARCHAR(255) NOT NULL,
    details JSONB DEFAULT '{}',
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_platform_audit_log_action ON ai_platform_audit_log(action);
CREATE INDEX idx_ai_platform_audit_log_resource_type ON ai_platform_audit_log(resource_type);
CREATE INDEX idx_ai_platform_audit_log_user_id ON ai_platform_audit_log(user_id);
CREATE INDEX idx_ai_platform_audit_log_created_at ON ai_platform_audit_log(created_at);

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
$$ language 'plpgsql';

-- Apply triggers to tables with updated_at
CREATE TRIGGER update_models_updated_at BEFORE UPDATE ON models
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_agents_updated_at BEFORE UPDATE ON agents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_tools_updated_at BEFORE UPDATE ON tools
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_ab_tests_updated_at BEFORE UPDATE ON ab_tests
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_fine_tune_jobs_updated_at BEFORE UPDATE ON fine_tune_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_agent_teams_updated_at BEFORE UPDATE ON agent_teams
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_rag_collections_updated_at BEFORE UPDATE ON rag_collections
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_rag_documents_updated_at BEFORE UPDATE ON rag_documents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_rag_pipelines_updated_at BEFORE UPDATE ON rag_pipelines
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comment on tables
COMMENT ON TABLE models IS 'Model configurations for various AI providers';
COMMENT ON TABLE model_versions IS 'Version history for models';
COMMENT ON TABLE model_deployments IS 'Model deployment tracking';
COMMENT ON TABLE model_metrics IS 'Performance and usage metrics for models';
COMMENT ON TABLE ab_tests IS 'A/B testing configurations';
COMMENT ON TABLE fine_tune_jobs IS 'Fine-tuning job management';
COMMENT ON TABLE agents IS 'AI agent configurations';
COMMENT ON TABLE agent_executions IS 'Agent execution history';
COMMENT ON TABLE tools IS 'Tool/function definitions for agents';
COMMENT ON TABLE tool_calls IS 'Tool execution history';
COMMENT ON TABLE agent_teams IS 'Multi-agent team configurations';
COMMENT ON TABLE team_executions IS 'Team execution history';
COMMENT ON TABLE rag_collections IS 'Document collections for RAG';
COMMENT ON TABLE rag_documents IS 'Documents in RAG collections';
COMMENT ON TABLE rag_document_chunks IS 'Chunked and embedded document segments';
COMMENT ON TABLE rag_pipelines IS 'RAG pipeline configurations';
