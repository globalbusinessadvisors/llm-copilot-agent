/**
 * Usage Tracking Service
 *
 * Handles recording and querying usage metrics for billing purposes.
 */

import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import { createClient, RedisClientType } from 'redis';
import {
  UsageEvent,
  UsageType,
  UsageUnit,
  CreateUsageEventInput,
  UsageQueryParams,
  UsageSummary,
  UsageAggregation,
  TenantUsageReport,
  PricingConfig,
  DEFAULT_PRICING,
  UsageQuota,
} from '../models/usage';
import { logger } from '../utils/logger';

export class UsageService {
  private db: Pool;
  private redis: RedisClientType;
  private pricing: PricingConfig;

  constructor(db: Pool, redis: RedisClientType, pricing?: PricingConfig) {
    this.db = db;
    this.redis = redis;
    this.pricing = pricing || DEFAULT_PRICING;
  }

  /**
   * Record a usage event
   */
  async recordUsage(input: CreateUsageEventInput): Promise<UsageEvent> {
    const now = new Date();
    const periodStart = this.getBillingPeriodStart(now);
    const periodEnd = this.getBillingPeriodEnd(now);

    const event: UsageEvent = {
      id: uuidv4(),
      tenantId: input.tenantId,
      userId: input.userId,
      type: input.type,
      unit: input.unit,
      quantity: input.quantity,
      metadata: input.metadata,
      resourceId: input.resourceId,
      resourceType: input.resourceType,
      model: input.model,
      endpoint: input.endpoint,
      statusCode: input.statusCode,
      timestamp: now,
      billingPeriodStart: periodStart,
      billingPeriodEnd: periodEnd,
    };

    // Store in database
    await this.db.query(
      `INSERT INTO usage_events (
        id, tenant_id, user_id, type, unit, quantity, metadata,
        resource_id, resource_type, model, endpoint, status_code,
        timestamp, billing_period_start, billing_period_end
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        event.id,
        event.tenantId,
        event.userId,
        event.type,
        event.unit,
        event.quantity,
        JSON.stringify(event.metadata || {}),
        event.resourceId,
        event.resourceType,
        event.model,
        event.endpoint,
        event.statusCode,
        event.timestamp,
        event.billingPeriodStart,
        event.billingPeriodEnd,
      ]
    );

    // Update real-time counters in Redis
    await this.updateRealtimeCounters(event);

    // Check quota warnings
    await this.checkQuotaWarnings(event.tenantId);

    logger.info('Usage event recorded', { eventId: event.id, type: event.type });

    return event;
  }

  /**
   * Record multiple usage events in batch
   */
  async recordUsageBatch(events: CreateUsageEventInput[]): Promise<UsageEvent[]> {
    const results: UsageEvent[] = [];

    const client = await this.db.connect();
    try {
      await client.query('BEGIN');

      for (const input of events) {
        const event = await this.recordUsage(input);
        results.push(event);
      }

      await client.query('COMMIT');
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }

    return results;
  }

  /**
   * Get usage summary for a tenant
   */
  async getUsageSummary(
    tenantId: string,
    startDate: Date,
    endDate: Date
  ): Promise<UsageSummary> {
    const result = await this.db.query(
      `SELECT
        COUNT(*) FILTER (WHERE type = 'api_call') as api_calls,
        COALESCE(SUM(quantity) FILTER (WHERE type = 'token_input'), 0) as input_tokens,
        COALESCE(SUM(quantity) FILTER (WHERE type = 'token_output'), 0) as output_tokens,
        COALESCE(SUM(quantity) FILTER (WHERE type = 'storage'), 0) as storage_bytes,
        COALESCE(SUM(quantity) FILTER (WHERE type = 'compute'), 0) as compute_seconds,
        COALESCE(SUM(quantity) FILTER (WHERE type = 'embedding'), 0) as embedding_tokens,
        COUNT(*) FILTER (WHERE type = 'workflow_run') as workflow_runs,
        COUNT(*) FILTER (WHERE type = 'context_search') as context_searches
      FROM usage_events
      WHERE tenant_id = $1
        AND timestamp >= $2
        AND timestamp < $3`,
      [tenantId, startDate, endDate]
    );

    const row = result.rows[0];
    return {
      tenantId,
      periodStart: startDate,
      periodEnd: endDate,
      apiCalls: parseInt(row.api_calls, 10),
      inputTokens: parseInt(row.input_tokens, 10),
      outputTokens: parseInt(row.output_tokens, 10),
      storageBytes: parseInt(row.storage_bytes, 10),
      computeSeconds: parseFloat(row.compute_seconds),
      embeddingTokens: parseInt(row.embedding_tokens, 10),
      workflowRuns: parseInt(row.workflow_runs, 10),
      contextSearches: parseInt(row.context_searches, 10),
    };
  }

  /**
   * Get usage aggregations with grouping
   */
  async getUsageAggregations(params: UsageQueryParams): Promise<UsageAggregation[]> {
    const { tenantId, userId, type, startDate, endDate, groupBy = 'day' } = params;

    const dateFormat = {
      hour: "YYYY-MM-DD HH24:00",
      day: "YYYY-MM-DD",
      week: "YYYY-WW",
      month: "YYYY-MM",
    }[groupBy];

    let query = `
      SELECT
        TO_CHAR(timestamp, '${dateFormat}') as period,
        type,
        SUM(quantity) as total_quantity,
        COUNT(*) as count,
        AVG(quantity) as avg_quantity,
        MAX(quantity) as max_quantity,
        MIN(quantity) as min_quantity
      FROM usage_events
      WHERE tenant_id = $1
        AND timestamp >= $2
        AND timestamp < $3
    `;
    const queryParams: (string | Date)[] = [tenantId, startDate, endDate];

    if (userId) {
      query += ` AND user_id = $${queryParams.length + 1}`;
      queryParams.push(userId);
    }

    if (type) {
      query += ` AND type = $${queryParams.length + 1}`;
      queryParams.push(type);
    }

    query += ` GROUP BY period, type ORDER BY period, type`;

    const result = await this.db.query(query, queryParams);

    return result.rows.map((row) => ({
      period: row.period,
      type: row.type as UsageType,
      totalQuantity: parseFloat(row.total_quantity),
      count: parseInt(row.count, 10),
      avgQuantity: parseFloat(row.avg_quantity),
      maxQuantity: parseFloat(row.max_quantity),
      minQuantity: parseFloat(row.min_quantity),
    }));
  }

  /**
   * Get detailed usage report for a tenant
   */
  async getTenantUsageReport(
    tenantId: string,
    startDate: Date,
    endDate: Date
  ): Promise<TenantUsageReport> {
    const summary = await this.getUsageSummary(tenantId, startDate, endDate);
    const breakdown = await this.getUsageAggregations({
      tenantId,
      startDate,
      endDate,
      groupBy: 'day',
    });
    const quota = await this.getQuota(tenantId);

    const quotaUsage = {
      apiCalls: this.calculateQuotaUsage(summary.apiCalls, quota?.apiCallsLimit),
      inputTokens: this.calculateQuotaUsage(summary.inputTokens, quota?.inputTokensLimit),
      outputTokens: this.calculateQuotaUsage(summary.outputTokens, quota?.outputTokensLimit),
      storage: this.calculateQuotaUsage(summary.storageBytes, quota?.storageBytesLimit),
      compute: this.calculateQuotaUsage(summary.computeSeconds, quota?.computeSecondsLimit),
    };

    const estimatedCost = this.calculateCost(summary);

    return {
      tenantId,
      periodStart: startDate,
      periodEnd: endDate,
      summary,
      breakdown,
      quotaUsage,
      estimatedCost,
    };
  }

  /**
   * Get current quota for a tenant
   */
  async getQuota(tenantId: string): Promise<UsageQuota | null> {
    const result = await this.db.query(
      `SELECT * FROM usage_quotas
       WHERE tenant_id = $1
         AND period_start <= NOW()
         AND period_end > NOW()
       ORDER BY period_start DESC
       LIMIT 1`,
      [tenantId]
    );

    if (result.rows.length === 0) return null;

    const row = result.rows[0];
    return {
      tenantId: row.tenant_id,
      apiCallsLimit: row.api_calls_limit,
      inputTokensLimit: row.input_tokens_limit,
      outputTokensLimit: row.output_tokens_limit,
      storageBytesLimit: row.storage_bytes_limit,
      computeSecondsLimit: row.compute_seconds_limit,
      periodStart: row.period_start,
      periodEnd: row.period_end,
    };
  }

  /**
   * Set quota for a tenant
   */
  async setQuota(quota: UsageQuota): Promise<void> {
    await this.db.query(
      `INSERT INTO usage_quotas (
        tenant_id, api_calls_limit, input_tokens_limit, output_tokens_limit,
        storage_bytes_limit, compute_seconds_limit, period_start, period_end
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      ON CONFLICT (tenant_id, period_start) DO UPDATE SET
        api_calls_limit = EXCLUDED.api_calls_limit,
        input_tokens_limit = EXCLUDED.input_tokens_limit,
        output_tokens_limit = EXCLUDED.output_tokens_limit,
        storage_bytes_limit = EXCLUDED.storage_bytes_limit,
        compute_seconds_limit = EXCLUDED.compute_seconds_limit`,
      [
        quota.tenantId,
        quota.apiCallsLimit,
        quota.inputTokensLimit,
        quota.outputTokensLimit,
        quota.storageBytesLimit,
        quota.computeSecondsLimit,
        quota.periodStart,
        quota.periodEnd,
      ]
    );
  }

  /**
   * Check if tenant has exceeded quota
   */
  async checkQuotaExceeded(tenantId: string): Promise<{
    exceeded: boolean;
    details: Record<string, { used: number; limit: number | null; exceeded: boolean }>;
  }> {
    const now = new Date();
    const periodStart = this.getBillingPeriodStart(now);
    const periodEnd = this.getBillingPeriodEnd(now);

    const summary = await this.getUsageSummary(tenantId, periodStart, periodEnd);
    const quota = await this.getQuota(tenantId);

    const details: Record<string, { used: number; limit: number | null; exceeded: boolean }> = {
      apiCalls: {
        used: summary.apiCalls,
        limit: quota?.apiCallsLimit ?? null,
        exceeded: quota?.apiCallsLimit !== null && summary.apiCalls > quota.apiCallsLimit,
      },
      inputTokens: {
        used: summary.inputTokens,
        limit: quota?.inputTokensLimit ?? null,
        exceeded: quota?.inputTokensLimit !== null && summary.inputTokens > quota.inputTokensLimit,
      },
      outputTokens: {
        used: summary.outputTokens,
        limit: quota?.outputTokensLimit ?? null,
        exceeded: quota?.outputTokensLimit !== null && summary.outputTokens > quota.outputTokensLimit,
      },
      storage: {
        used: summary.storageBytes,
        limit: quota?.storageBytesLimit ?? null,
        exceeded: quota?.storageBytesLimit !== null && summary.storageBytes > quota.storageBytesLimit,
      },
      compute: {
        used: summary.computeSeconds,
        limit: quota?.computeSecondsLimit ?? null,
        exceeded: quota?.computeSecondsLimit !== null && summary.computeSeconds > quota.computeSecondsLimit,
      },
    };

    const exceeded = Object.values(details).some((d) => d.exceeded);

    return { exceeded, details };
  }

  /**
   * Get real-time usage from Redis
   */
  async getRealtimeUsage(tenantId: string): Promise<Record<UsageType, number>> {
    const keys = Object.values(UsageType).map(
      (type) => `usage:${tenantId}:${type}:${this.getCurrentPeriodKey()}`
    );

    const values = await this.redis.mGet(keys);

    const result: Record<string, number> = {};
    Object.values(UsageType).forEach((type, index) => {
      result[type] = parseInt(values[index] || '0', 10);
    });

    return result as Record<UsageType, number>;
  }

  // ===========================================
  // Private Methods
  // ===========================================

  private async updateRealtimeCounters(event: UsageEvent): Promise<void> {
    const key = `usage:${event.tenantId}:${event.type}:${this.getCurrentPeriodKey()}`;
    await this.redis.incrByFloat(key, event.quantity);
    await this.redis.expire(key, 86400 * 35); // 35 days TTL
  }

  private async checkQuotaWarnings(tenantId: string): Promise<void> {
    const { exceeded, details } = await this.checkQuotaExceeded(tenantId);

    for (const [metric, info] of Object.entries(details)) {
      if (info.limit !== null) {
        const percentage = (info.used / info.limit) * 100;

        // Check for 80% warning
        if (percentage >= 80 && percentage < 100) {
          await this.emitQuotaWarning(tenantId, metric, percentage);
        }

        // Check for exceeded
        if (info.exceeded) {
          await this.emitQuotaExceeded(tenantId, metric);
        }
      }
    }
  }

  private async emitQuotaWarning(
    tenantId: string,
    metric: string,
    percentage: number
  ): Promise<void> {
    const cacheKey = `quota_warning:${tenantId}:${metric}:${this.getCurrentPeriodKey()}`;
    const alreadyWarned = await this.redis.get(cacheKey);

    if (!alreadyWarned) {
      logger.warn('Quota warning', { tenantId, metric, percentage });
      await this.redis.set(cacheKey, '1', { EX: 86400 });
      // TODO: Emit webhook/notification
    }
  }

  private async emitQuotaExceeded(tenantId: string, metric: string): Promise<void> {
    logger.warn('Quota exceeded', { tenantId, metric });
    // TODO: Emit webhook/notification
  }

  private calculateQuotaUsage(
    used: number,
    limit: number | null
  ): { used: number; limit: number | null; percentage: number | null } {
    return {
      used,
      limit,
      percentage: limit !== null ? Math.min((used / limit) * 100, 100) : null,
    };
  }

  private calculateCost(summary: UsageSummary): number {
    const p = this.pricing;
    return (
      summary.apiCalls * p.apiCallPrice +
      summary.inputTokens * p.inputTokenPrice +
      summary.outputTokens * p.outputTokenPrice +
      (summary.storageBytes / (1024 * 1024 * 1024)) * p.storagePricePerGb +
      (summary.computeSeconds / 3600) * p.computePricePerHour +
      summary.embeddingTokens * p.embeddingTokenPrice +
      summary.workflowRuns * p.workflowRunPrice +
      summary.contextSearches * p.contextSearchPrice
    );
  }

  private getBillingPeriodStart(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), 1);
  }

  private getBillingPeriodEnd(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth() + 1, 1);
  }

  private getCurrentPeriodKey(): string {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
  }
}
