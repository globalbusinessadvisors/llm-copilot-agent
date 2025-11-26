-- Billing Service - Usage Events Partitioning
-- Migration: 003_usage_partitioning
-- Created: 2024-01-01
--
-- This migration sets up table partitioning for usage_events
-- for better query performance with large datasets

-- Only run this if you need partitioning (high volume usage)
-- This requires PostgreSQL 11+

-- Note: This is a sample for setting up range partitioning by month
-- You'll need to create partitions for each month as needed

-- First, we need to recreate the table with partitioning
-- Backup existing data first!

-- Step 1: Rename old table
-- ALTER TABLE usage_events RENAME TO usage_events_old;

-- Step 2: Create new partitioned table
-- CREATE TABLE usage_events (
--     id UUID NOT NULL DEFAULT uuid_generate_v4(),
--     tenant_id UUID NOT NULL,
--     user_id UUID,
--     type VARCHAR(50) NOT NULL,
--     unit VARCHAR(50) NOT NULL,
--     quantity DECIMAL(20, 6) NOT NULL,
--     metadata JSONB DEFAULT '{}',
--     resource_id VARCHAR(255),
--     resource_type VARCHAR(100),
--     model VARCHAR(100),
--     endpoint VARCHAR(255),
--     status_code INTEGER,
--     timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     billing_period_start TIMESTAMPTZ NOT NULL,
--     billing_period_end TIMESTAMPTZ NOT NULL,
--     PRIMARY KEY (id, timestamp)
-- ) PARTITION BY RANGE (timestamp);

-- Step 3: Create partitions for each month
-- Example for 2024:

-- CREATE TABLE usage_events_2024_01 PARTITION OF usage_events
--     FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- CREATE TABLE usage_events_2024_02 PARTITION OF usage_events
--     FOR VALUES FROM ('2024-02-01') TO ('2024-03-01');

-- ... continue for each month

-- Step 4: Copy data from old table
-- INSERT INTO usage_events SELECT * FROM usage_events_old;

-- Step 5: Drop old table
-- DROP TABLE usage_events_old;

-- Alternative: Create a function to automatically create partitions
CREATE OR REPLACE FUNCTION create_usage_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'usage_events_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    -- Check if partition already exists
    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = partition_name
    ) THEN
        EXECUTE FORMAT(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF usage_events
             FOR VALUES FROM (%L) TO (%L)',
            partition_name, start_date, end_date
        );

        -- Create indexes on the partition
        EXECUTE FORMAT(
            'CREATE INDEX IF NOT EXISTS %I ON %I (tenant_id)',
            partition_name || '_tenant_idx', partition_name
        );
        EXECUTE FORMAT(
            'CREATE INDEX IF NOT EXISTS %I ON %I (type)',
            partition_name || '_type_idx', partition_name
        );
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Function to ensure partitions exist before inserts
CREATE OR REPLACE FUNCTION ensure_usage_partition()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM create_usage_partition(NEW.timestamp::DATE);
    RETURN NEW;
EXCEPTION
    WHEN duplicate_table THEN
        RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- For non-partitioned tables, add materialized views for reporting
CREATE MATERIALIZED VIEW IF NOT EXISTS usage_daily_summary AS
SELECT
    tenant_id,
    DATE(timestamp) as date,
    type,
    unit,
    SUM(quantity) as total_quantity,
    COUNT(*) as event_count,
    AVG(quantity) as avg_quantity,
    MAX(quantity) as max_quantity
FROM usage_events
GROUP BY tenant_id, DATE(timestamp), type, unit
WITH DATA;

-- Create unique index for concurrent refresh
CREATE UNIQUE INDEX IF NOT EXISTS idx_usage_daily_summary_unique
ON usage_daily_summary (tenant_id, date, type, unit);

-- Create regular indexes
CREATE INDEX IF NOT EXISTS idx_usage_daily_summary_tenant_date
ON usage_daily_summary (tenant_id, date);

-- Function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_usage_daily_summary()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY usage_daily_summary;
END;
$$ LANGUAGE plpgsql;

-- You can set up a cron job or scheduled task to call this function periodically
-- Example using pg_cron (if available):
-- SELECT cron.schedule('0 * * * *', $$SELECT refresh_usage_daily_summary()$$);
