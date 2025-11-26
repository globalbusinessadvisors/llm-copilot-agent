/**
 * Audit Service
 *
 * Manages comprehensive audit trail for all platform activities.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  AuditEvent,
  AuditEventType,
  AuditSeverity,
  CreateAuditEventInput,
} from '../models/governance';

interface AuditSearchOptions {
  types?: AuditEventType[];
  severity?: AuditSeverity[];
  actorId?: string;
  actorType?: AuditEvent['actor']['type'];
  resourceType?: string;
  resourceId?: string;
  outcome?: AuditEvent['outcome'];
  startDate?: Date;
  endDate?: Date;
  search?: string;
  limit?: number;
  offset?: number;
}

export class AuditService {
  private db: Pool;
  private redis: RedisClientType;
  private eventBuffer: AuditEvent[] = [];
  private flushInterval: NodeJS.Timeout | null = null;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;

    // Start buffer flush interval
    this.startFlushInterval();
  }

  /**
   * Start buffer flush interval for batch inserts
   */
  private startFlushInterval(): void {
    this.flushInterval = setInterval(() => {
      this.flushBuffer().catch(console.error);
    }, 5000); // Flush every 5 seconds
  }

  /**
   * Stop buffer flush interval
   */
  stopFlushInterval(): void {
    if (this.flushInterval) {
      clearInterval(this.flushInterval);
      this.flushInterval = null;
    }
    // Final flush
    this.flushBuffer().catch(console.error);
  }

  /**
   * Flush event buffer to database
   */
  private async flushBuffer(): Promise<void> {
    if (this.eventBuffer.length === 0) return;

    const events = [...this.eventBuffer];
    this.eventBuffer = [];

    // Batch insert
    const values: string[] = [];
    const params: unknown[] = [];
    let paramIndex = 1;

    for (const event of events) {
      values.push(`($${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++}, $${paramIndex++})`);
      params.push(
        event.id,
        event.type,
        event.severity,
        JSON.stringify(event.actor),
        event.action,
        event.resource ? JSON.stringify(event.resource) : null,
        event.outcome,
        JSON.stringify({ ...event.details, ...event.metadata }),
        event.timestamp
      );
    }

    await this.db.query(
      `INSERT INTO audit_events (id, type, severity, actor, action, resource, outcome, details, timestamp)
       VALUES ${values.join(', ')}`,
      params
    );
  }

  // ===========================================
  // Event Recording
  // ===========================================

  /**
   * Record an audit event
   */
  async recordEvent(input: CreateAuditEventInput): Promise<AuditEvent> {
    const event: AuditEvent = {
      id: uuidv4(),
      type: input.type,
      severity: input.severity || AuditSeverity.INFO,
      actor: input.actor,
      action: input.action,
      resource: input.resource,
      outcome: input.outcome,
      details: input.details,
      metadata: input.metadata,
      timestamp: new Date(),
    };

    // Add to buffer for batch insert
    this.eventBuffer.push(event);

    // For critical events, also publish to Redis for real-time monitoring
    if (event.severity === AuditSeverity.CRITICAL || event.severity === AuditSeverity.ERROR) {
      await this.publishEvent(event);
    }

    return event;
  }

  /**
   * Record multiple events
   */
  async recordEvents(inputs: CreateAuditEventInput[]): Promise<AuditEvent[]> {
    const events: AuditEvent[] = inputs.map(input => ({
      id: uuidv4(),
      type: input.type,
      severity: input.severity || AuditSeverity.INFO,
      actor: input.actor,
      action: input.action,
      resource: input.resource,
      outcome: input.outcome,
      details: input.details,
      metadata: input.metadata,
      timestamp: new Date(),
    }));

    this.eventBuffer.push(...events);

    return events;
  }

  /**
   * Publish event to Redis for real-time subscribers
   */
  private async publishEvent(event: AuditEvent): Promise<void> {
    const channel = `audit:${event.type}`;
    await this.redis.publish(channel, JSON.stringify(event));

    // Also publish to severity-specific channel
    await this.redis.publish(`audit:severity:${event.severity}`, JSON.stringify(event));
  }

  // ===========================================
  // Event Retrieval
  // ===========================================

  /**
   * Get event by ID
   */
  async getEvent(eventId: string): Promise<AuditEvent | null> {
    const result = await this.db.query(
      `SELECT * FROM audit_events WHERE id = $1`,
      [eventId]
    );

    if (result.rows.length === 0) return null;

    return this.mapEventRow(result.rows[0]);
  }

  /**
   * Search audit events
   */
  async searchEvents(options: AuditSearchOptions): Promise<{
    events: AuditEvent[];
    total: number;
    hasMore: boolean;
  }> {
    let query = `SELECT * FROM audit_events WHERE 1=1`;
    let countQuery = `SELECT COUNT(*) FROM audit_events WHERE 1=1`;
    const values: unknown[] = [];
    const countValues: unknown[] = [];
    let paramIndex = 1;

    // Build filters
    if (options.types && options.types.length > 0) {
      query += ` AND type = ANY($${paramIndex})`;
      countQuery += ` AND type = ANY($${paramIndex++})`;
      values.push(options.types);
      countValues.push(options.types);
    }
    if (options.severity && options.severity.length > 0) {
      query += ` AND severity = ANY($${paramIndex})`;
      countQuery += ` AND severity = ANY($${paramIndex++})`;
      values.push(options.severity);
      countValues.push(options.severity);
    }
    if (options.actorId) {
      query += ` AND actor->>'id' = $${paramIndex}`;
      countQuery += ` AND actor->>'id' = $${paramIndex++}`;
      values.push(options.actorId);
      countValues.push(options.actorId);
    }
    if (options.actorType) {
      query += ` AND actor->>'type' = $${paramIndex}`;
      countQuery += ` AND actor->>'type' = $${paramIndex++}`;
      values.push(options.actorType);
      countValues.push(options.actorType);
    }
    if (options.resourceType) {
      query += ` AND resource->>'type' = $${paramIndex}`;
      countQuery += ` AND resource->>'type' = $${paramIndex++}`;
      values.push(options.resourceType);
      countValues.push(options.resourceType);
    }
    if (options.resourceId) {
      query += ` AND resource->>'id' = $${paramIndex}`;
      countQuery += ` AND resource->>'id' = $${paramIndex++}`;
      values.push(options.resourceId);
      countValues.push(options.resourceId);
    }
    if (options.outcome) {
      query += ` AND outcome = $${paramIndex}`;
      countQuery += ` AND outcome = $${paramIndex++}`;
      values.push(options.outcome);
      countValues.push(options.outcome);
    }
    if (options.startDate) {
      query += ` AND timestamp >= $${paramIndex}`;
      countQuery += ` AND timestamp >= $${paramIndex++}`;
      values.push(options.startDate);
      countValues.push(options.startDate);
    }
    if (options.endDate) {
      query += ` AND timestamp <= $${paramIndex}`;
      countQuery += ` AND timestamp <= $${paramIndex++}`;
      values.push(options.endDate);
      countValues.push(options.endDate);
    }
    if (options.search) {
      query += ` AND (action ILIKE $${paramIndex} OR details::text ILIKE $${paramIndex})`;
      countQuery += ` AND (action ILIKE $${paramIndex} OR details::text ILIKE $${paramIndex++})`;
      const searchPattern = `%${options.search}%`;
      values.push(searchPattern);
      countValues.push(searchPattern);
    }

    // Add ordering and pagination
    const limit = options.limit || 50;
    const offset = options.offset || 0;

    query += ` ORDER BY timestamp DESC LIMIT $${paramIndex++} OFFSET $${paramIndex}`;
    values.push(limit + 1); // Request one extra to check hasMore
    values.push(offset);

    // Execute queries
    const [eventsResult, countResult] = await Promise.all([
      this.db.query(query, values),
      this.db.query(countQuery, countValues),
    ]);

    const events = eventsResult.rows.slice(0, limit).map(this.mapEventRow);
    const total = parseInt(countResult.rows[0]?.count || '0', 10);
    const hasMore = eventsResult.rows.length > limit;

    return { events, total, hasMore };
  }

  /**
   * Get events for a specific resource
   */
  async getResourceHistory(
    resourceType: string,
    resourceId: string,
    limit = 100
  ): Promise<AuditEvent[]> {
    const result = await this.db.query(
      `SELECT * FROM audit_events
       WHERE resource->>'type' = $1 AND resource->>'id' = $2
       ORDER BY timestamp DESC LIMIT $3`,
      [resourceType, resourceId, limit]
    );

    return result.rows.map(this.mapEventRow);
  }

  /**
   * Get events for a specific actor
   */
  async getActorHistory(
    actorId: string,
    limit = 100
  ): Promise<AuditEvent[]> {
    const result = await this.db.query(
      `SELECT * FROM audit_events
       WHERE actor->>'id' = $1
       ORDER BY timestamp DESC LIMIT $2`,
      [actorId, limit]
    );

    return result.rows.map(this.mapEventRow);
  }

  // ===========================================
  // Analytics
  // ===========================================

  /**
   * Get audit statistics
   */
  async getStatistics(options?: {
    startDate?: Date;
    endDate?: Date;
    groupBy?: 'hour' | 'day' | 'week';
  }): Promise<{
    totalEvents: number;
    byType: Record<AuditEventType, number>;
    bySeverity: Record<AuditSeverity, number>;
    byOutcome: Record<string, number>;
    topActors: Array<{ actorId: string; count: number }>;
    topResources: Array<{ resourceType: string; count: number }>;
    timeline: Array<{ period: string; count: number }>;
  }> {
    let dateFilter = '';
    const values: unknown[] = [];
    let paramIndex = 1;

    if (options?.startDate) {
      dateFilter += ` AND timestamp >= $${paramIndex++}`;
      values.push(options.startDate);
    }
    if (options?.endDate) {
      dateFilter += ` AND timestamp <= $${paramIndex++}`;
      values.push(options.endDate);
    }

    // Total events
    const totalResult = await this.db.query(
      `SELECT COUNT(*) FROM audit_events WHERE 1=1 ${dateFilter}`,
      values
    );
    const totalEvents = parseInt(totalResult.rows[0]?.count || '0', 10);

    // By type
    const typeResult = await this.db.query(
      `SELECT type, COUNT(*) as count FROM audit_events WHERE 1=1 ${dateFilter} GROUP BY type`,
      values
    );
    const byType: Record<string, number> = {};
    typeResult.rows.forEach(row => {
      byType[row.type] = parseInt(row.count, 10);
    });

    // By severity
    const severityResult = await this.db.query(
      `SELECT severity, COUNT(*) as count FROM audit_events WHERE 1=1 ${dateFilter} GROUP BY severity`,
      values
    );
    const bySeverity: Record<string, number> = {};
    severityResult.rows.forEach(row => {
      bySeverity[row.severity] = parseInt(row.count, 10);
    });

    // By outcome
    const outcomeResult = await this.db.query(
      `SELECT outcome, COUNT(*) as count FROM audit_events WHERE 1=1 ${dateFilter} GROUP BY outcome`,
      values
    );
    const byOutcome: Record<string, number> = {};
    outcomeResult.rows.forEach(row => {
      byOutcome[row.outcome] = parseInt(row.count, 10);
    });

    // Top actors
    const actorsResult = await this.db.query(
      `SELECT actor->>'id' as actor_id, COUNT(*) as count
       FROM audit_events WHERE 1=1 ${dateFilter}
       GROUP BY actor->>'id' ORDER BY count DESC LIMIT 10`,
      values
    );
    const topActors = actorsResult.rows.map(row => ({
      actorId: row.actor_id,
      count: parseInt(row.count, 10),
    }));

    // Top resources
    const resourcesResult = await this.db.query(
      `SELECT resource->>'type' as resource_type, COUNT(*) as count
       FROM audit_events WHERE resource IS NOT NULL ${dateFilter}
       GROUP BY resource->>'type' ORDER BY count DESC LIMIT 10`,
      values
    );
    const topResources = resourcesResult.rows.map(row => ({
      resourceType: row.resource_type,
      count: parseInt(row.count, 10),
    }));

    // Timeline
    const truncate = options?.groupBy === 'hour' ? 'hour'
      : options?.groupBy === 'week' ? 'week'
      : 'day';
    const timelineResult = await this.db.query(
      `SELECT DATE_TRUNC('${truncate}', timestamp) as period, COUNT(*) as count
       FROM audit_events WHERE 1=1 ${dateFilter}
       GROUP BY period ORDER BY period`,
      values
    );
    const timeline = timelineResult.rows.map(row => ({
      period: row.period.toISOString(),
      count: parseInt(row.count, 10),
    }));

    return {
      totalEvents,
      byType: byType as Record<AuditEventType, number>,
      bySeverity: bySeverity as Record<AuditSeverity, number>,
      byOutcome,
      topActors,
      topResources,
      timeline,
    };
  }

  /**
   * Detect anomalies in audit patterns
   */
  async detectAnomalies(options?: {
    windowMinutes?: number;
  }): Promise<Array<{
    type: string;
    description: string;
    severity: AuditSeverity;
    details: Record<string, unknown>;
  }>> {
    const anomalies: Array<{
      type: string;
      description: string;
      severity: AuditSeverity;
      details: Record<string, unknown>;
    }> = [];

    const windowMinutes = options?.windowMinutes || 60;
    const windowStart = new Date();
    windowStart.setMinutes(windowStart.getMinutes() - windowMinutes);

    // Check for unusual failure rates
    const failureResult = await this.db.query(
      `SELECT
        COUNT(*) as total,
        SUM(CASE WHEN outcome = 'failure' THEN 1 ELSE 0 END) as failures
       FROM audit_events WHERE timestamp >= $1`,
      [windowStart]
    );

    const total = parseInt(failureResult.rows[0]?.total || '0', 10);
    const failures = parseInt(failureResult.rows[0]?.failures || '0', 10);
    const failureRate = total > 0 ? failures / total : 0;

    if (failureRate > 0.3 && total > 100) {
      anomalies.push({
        type: 'high_failure_rate',
        description: `High failure rate detected: ${(failureRate * 100).toFixed(1)}%`,
        severity: AuditSeverity.WARNING,
        details: { total, failures, failureRate },
      });
    }

    // Check for unusual activity from single actor
    const actorResult = await this.db.query(
      `SELECT actor->>'id' as actor_id, COUNT(*) as count
       FROM audit_events WHERE timestamp >= $1
       GROUP BY actor->>'id' HAVING COUNT(*) > 1000
       ORDER BY count DESC LIMIT 5`,
      [windowStart]
    );

    for (const row of actorResult.rows) {
      anomalies.push({
        type: 'excessive_activity',
        description: `Excessive activity from actor ${row.actor_id}`,
        severity: AuditSeverity.WARNING,
        details: { actorId: row.actor_id, count: parseInt(row.count, 10) },
      });
    }

    // Check for security-related events
    const securityResult = await this.db.query(
      `SELECT type, COUNT(*) as count
       FROM audit_events
       WHERE timestamp >= $1 AND type IN ('login_failed', 'access_denied', 'breach_attempt')
       GROUP BY type HAVING COUNT(*) > 50`,
      [windowStart]
    );

    for (const row of securityResult.rows) {
      anomalies.push({
        type: 'security_concern',
        description: `High number of ${row.type} events: ${row.count}`,
        severity: AuditSeverity.ERROR,
        details: { eventType: row.type, count: parseInt(row.count, 10) },
      });
    }

    return anomalies;
  }

  // ===========================================
  // Retention
  // ===========================================

  /**
   * Archive old audit events
   */
  async archiveEvents(olderThan: Date): Promise<{ archived: number }> {
    // Move to archive table
    const archiveResult = await this.db.query(
      `INSERT INTO audit_events_archive
       SELECT * FROM audit_events WHERE timestamp < $1`,
      [olderThan]
    );

    // Delete from main table
    const deleteResult = await this.db.query(
      `DELETE FROM audit_events WHERE timestamp < $1`,
      [olderThan]
    );

    return { archived: deleteResult.rowCount || 0 };
  }

  /**
   * Purge archived events
   */
  async purgeArchive(olderThan: Date): Promise<{ purged: number }> {
    const result = await this.db.query(
      `DELETE FROM audit_events_archive WHERE timestamp < $1`,
      [olderThan]
    );

    return { purged: result.rowCount || 0 };
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapEventRow(row: Record<string, unknown>): AuditEvent {
    const details = row.details as Record<string, unknown> || {};
    const { sessionId, requestId, correlationId, source, version, ...otherDetails } = details;

    return {
      id: row.id as string,
      type: row.type as AuditEventType,
      severity: row.severity as AuditSeverity,
      actor: row.actor as AuditEvent['actor'],
      action: row.action as string,
      resource: row.resource as AuditEvent['resource'],
      outcome: row.outcome as AuditEvent['outcome'],
      details: otherDetails,
      metadata: {
        sessionId: sessionId as string,
        requestId: requestId as string,
        correlationId: correlationId as string,
        source: source as string,
        version: version as string,
      },
      timestamp: row.timestamp as Date,
    };
  }
}
