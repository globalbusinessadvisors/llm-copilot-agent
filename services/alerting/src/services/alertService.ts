/**
 * Alert Service
 *
 * Handles alert creation, notification, and escalation.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  Alert,
  AlertRule,
  AlertSeverity,
  AlertStatus,
  AlertChannel,
  EscalationPolicy,
  CreateAlertInput,
  CreateAlertRuleInput,
  AlertSummary,
} from '../models/alert';
import { NotificationService } from './notificationService';

export class AlertService {
  private db: Pool;
  private redis: RedisClientType;
  private notificationService: NotificationService;

  constructor(db: Pool, redis: RedisClientType, notificationService: NotificationService) {
    this.db = db;
    this.redis = redis;
    this.notificationService = notificationService;
  }

  // ===========================================
  // Alert Management
  // ===========================================

  /**
   * Create and trigger a new alert
   */
  async createAlert(input: CreateAlertInput): Promise<Alert> {
    const rule = await this.getAlertRule(input.ruleId);
    if (!rule) {
      throw new Error(`Alert rule not found: ${input.ruleId}`);
    }

    // Check cooldown
    if (rule.lastTriggeredAt) {
      const cooldownEnd = new Date(rule.lastTriggeredAt.getTime() + rule.cooldownPeriod * 1000);
      if (new Date() < cooldownEnd) {
        throw new Error('Alert is in cooldown period');
      }
    }

    const alert: Alert = {
      id: uuidv4(),
      ruleId: input.ruleId,
      title: input.title,
      description: input.description,
      severity: input.severity,
      status: AlertStatus.TRIGGERED,
      source: input.source,
      tags: input.tags || {},
      metadata: input.metadata || {},
      triggeredAt: new Date(),
      escalationLevel: 0,
      notificationsSent: [],
    };

    await this.db.query(
      `INSERT INTO alerts (
        id, rule_id, title, description, severity, status, source,
        tags, metadata, triggered_at, escalation_level
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)`,
      [
        alert.id, alert.ruleId, alert.title, alert.description,
        alert.severity, alert.status, alert.source,
        JSON.stringify(alert.tags), JSON.stringify(alert.metadata),
        alert.triggeredAt, alert.escalationLevel,
      ]
    );

    // Update rule last triggered
    await this.db.query(
      `UPDATE alert_rules SET last_triggered_at = NOW() WHERE id = $1`,
      [input.ruleId]
    );

    // Send initial notifications
    await this.sendNotifications(alert, rule);

    // Schedule escalation check
    await this.scheduleEscalation(alert.id);

    return alert;
  }

  /**
   * Acknowledge an alert
   */
  async acknowledgeAlert(alertId: string, userId: string): Promise<Alert | null> {
    const result = await this.db.query(
      `UPDATE alerts SET
        status = $1,
        acknowledged_at = NOW(),
        acknowledged_by = $2
      WHERE id = $3 AND status = $4
      RETURNING *`,
      [AlertStatus.ACKNOWLEDGED, userId, alertId, AlertStatus.TRIGGERED]
    );

    if (result.rows.length === 0) return null;

    // Cancel escalation
    await this.redis.del(`alert:escalation:${alertId}`);

    return this.mapAlertRow(result.rows[0]);
  }

  /**
   * Resolve an alert
   */
  async resolveAlert(alertId: string, userId: string): Promise<Alert | null> {
    const result = await this.db.query(
      `UPDATE alerts SET
        status = $1,
        resolved_at = NOW(),
        resolved_by = $2
      WHERE id = $3 AND status IN ($4, $5)
      RETURNING *`,
      [AlertStatus.RESOLVED, userId, alertId, AlertStatus.TRIGGERED, AlertStatus.ACKNOWLEDGED]
    );

    if (result.rows.length === 0) return null;

    // Cancel escalation
    await this.redis.del(`alert:escalation:${alertId}`);

    return this.mapAlertRow(result.rows[0]);
  }

  /**
   * Get alert by ID
   */
  async getAlert(alertId: string): Promise<Alert | null> {
    const result = await this.db.query(
      `SELECT * FROM alerts WHERE id = $1`,
      [alertId]
    );

    if (result.rows.length === 0) return null;

    return this.mapAlertRow(result.rows[0]);
  }

  /**
   * Get active alerts
   */
  async getActiveAlerts(): Promise<Alert[]> {
    const result = await this.db.query(
      `SELECT * FROM alerts
       WHERE status IN ('triggered', 'acknowledged')
       ORDER BY severity DESC, triggered_at DESC`
    );
    return result.rows.map(this.mapAlertRow);
  }

  /**
   * Get alert history
   */
  async getAlertHistory(limit: number = 100): Promise<Alert[]> {
    const result = await this.db.query(
      `SELECT * FROM alerts ORDER BY triggered_at DESC LIMIT $1`,
      [limit]
    );
    return result.rows.map(this.mapAlertRow);
  }

  /**
   * Get alert summary
   */
  async getAlertSummary(): Promise<AlertSummary> {
    const result = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE status = 'triggered') as triggered,
        COUNT(*) FILTER (WHERE status = 'acknowledged') as acknowledged,
        COUNT(*) FILTER (WHERE status = 'resolved') as resolved,
        COUNT(*) FILTER (WHERE status = 'suppressed') as suppressed,
        COUNT(*) FILTER (WHERE severity = 'critical') as critical,
        COUNT(*) FILTER (WHERE severity = 'error') as error,
        COUNT(*) FILTER (WHERE severity = 'warning') as warning,
        COUNT(*) FILTER (WHERE severity = 'info') as info,
        AVG(EXTRACT(EPOCH FROM (resolved_at - triggered_at))) FILTER (WHERE resolved_at IS NOT NULL) as mttr,
        AVG(EXTRACT(EPOCH FROM (acknowledged_at - triggered_at))) FILTER (WHERE acknowledged_at IS NOT NULL) as mtta
      FROM alerts
      WHERE triggered_at > NOW() - INTERVAL '30 days'
    `);

    const row = result.rows[0];
    return {
      total: parseInt(row.total, 10),
      byStatus: {
        [AlertStatus.TRIGGERED]: parseInt(row.triggered, 10),
        [AlertStatus.ACKNOWLEDGED]: parseInt(row.acknowledged, 10),
        [AlertStatus.RESOLVED]: parseInt(row.resolved, 10),
        [AlertStatus.SUPPRESSED]: parseInt(row.suppressed, 10),
      },
      bySeverity: {
        [AlertSeverity.CRITICAL]: parseInt(row.critical, 10),
        [AlertSeverity.ERROR]: parseInt(row.error, 10),
        [AlertSeverity.WARNING]: parseInt(row.warning, 10),
        [AlertSeverity.INFO]: parseInt(row.info, 10),
      },
      mttr: parseFloat(row.mttr) || 0,
      mtta: parseFloat(row.mtta) || 0,
    };
  }

  // ===========================================
  // Alert Rules
  // ===========================================

  /**
   * Create an alert rule
   */
  async createAlertRule(input: CreateAlertRuleInput): Promise<AlertRule> {
    const rule: AlertRule = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      enabled: true,
      conditionType: input.conditionType,
      condition: input.condition,
      severity: input.severity,
      tags: input.tags || {},
      escalationPolicyId: input.escalationPolicyId,
      notificationChannels: input.notificationChannels,
      cooldownPeriod: input.cooldownPeriod || 300,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO alert_rules (
        id, name, description, enabled, condition_type, condition,
        severity, tags, escalation_policy_id, notification_channels,
        cooldown_period, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
      [
        rule.id, rule.name, rule.description, rule.enabled,
        rule.conditionType, JSON.stringify(rule.condition),
        rule.severity, JSON.stringify(rule.tags), rule.escalationPolicyId,
        JSON.stringify(rule.notificationChannels), rule.cooldownPeriod,
        rule.createdAt, rule.updatedAt,
      ]
    );

    return rule;
  }

  /**
   * Get alert rule by ID
   */
  async getAlertRule(ruleId: string): Promise<AlertRule | null> {
    const result = await this.db.query(
      `SELECT * FROM alert_rules WHERE id = $1`,
      [ruleId]
    );

    if (result.rows.length === 0) return null;

    return this.mapAlertRuleRow(result.rows[0]);
  }

  /**
   * Get all alert rules
   */
  async getAlertRules(): Promise<AlertRule[]> {
    const result = await this.db.query(
      `SELECT * FROM alert_rules ORDER BY name`
    );
    return result.rows.map(this.mapAlertRuleRow);
  }

  /**
   * Enable/disable an alert rule
   */
  async setRuleEnabled(ruleId: string, enabled: boolean): Promise<void> {
    await this.db.query(
      `UPDATE alert_rules SET enabled = $1, updated_at = NOW() WHERE id = $2`,
      [enabled, ruleId]
    );
  }

  // ===========================================
  // Escalation
  // ===========================================

  /**
   * Process escalation for an alert
   */
  async processEscalation(alertId: string): Promise<void> {
    const alert = await this.getAlert(alertId);
    if (!alert || alert.status !== AlertStatus.TRIGGERED) {
      return;
    }

    const rule = await this.getAlertRule(alert.ruleId);
    if (!rule || !rule.escalationPolicyId) {
      return;
    }

    const policy = await this.getEscalationPolicy(rule.escalationPolicyId);
    if (!policy) {
      return;
    }

    // Find current escalation step
    const currentStep = policy.steps.find(s => s.order === alert.escalationLevel);
    const nextStep = policy.steps.find(s => s.order === alert.escalationLevel + 1);

    if (nextStep) {
      // Escalate to next level
      await this.db.query(
        `UPDATE alerts SET escalation_level = $1 WHERE id = $2`,
        [nextStep.order, alertId]
      );

      // Send notifications to next level
      for (const target of nextStep.targets) {
        for (const channel of target.channels) {
          await this.notificationService.sendNotification({
            channel,
            recipientId: target.id,
            recipientType: target.type,
            alert,
            escalationLevel: nextStep.order,
          });
        }
      }

      // Schedule next escalation
      await this.scheduleEscalation(alertId, nextStep.delayMinutes * 60);
    } else if (policy.repeatAfterMinutes) {
      // Reset and repeat from first step
      await this.db.query(
        `UPDATE alerts SET escalation_level = 0 WHERE id = $1`,
        [alertId]
      );
      await this.scheduleEscalation(alertId, policy.repeatAfterMinutes * 60);
    }
  }

  private async scheduleEscalation(alertId: string, delaySeconds: number = 300): Promise<void> {
    // Store escalation schedule in Redis with TTL
    await this.redis.set(
      `alert:escalation:${alertId}`,
      Date.now().toString(),
      { EX: delaySeconds }
    );
  }

  /**
   * Get escalation policy by ID
   */
  async getEscalationPolicy(policyId: string): Promise<EscalationPolicy | null> {
    const result = await this.db.query(
      `SELECT * FROM escalation_policies WHERE id = $1`,
      [policyId]
    );

    if (result.rows.length === 0) return null;

    return this.mapEscalationPolicyRow(result.rows[0]);
  }

  // ===========================================
  // Notifications
  // ===========================================

  private async sendNotifications(alert: Alert, rule: AlertRule): Promise<void> {
    for (const channel of rule.notificationChannels) {
      try {
        await this.notificationService.sendNotification({
          channel,
          alert,
          escalationLevel: 0,
        });

        // Record notification
        await this.db.query(
          `UPDATE alerts SET notifications_sent = notifications_sent || $1::jsonb WHERE id = $2`,
          [
            JSON.stringify([{
              channel,
              recipient: 'default',
              sentAt: new Date().toISOString(),
              success: true,
            }]),
            alert.id,
          ]
        );
      } catch (error) {
        console.error(`Failed to send ${channel} notification:`, error);
      }
    }
  }

  // ===========================================
  // Private Helpers
  // ===========================================

  private mapAlertRow(row: any): Alert {
    return {
      id: row.id,
      ruleId: row.rule_id,
      title: row.title,
      description: row.description,
      severity: row.severity,
      status: row.status,
      source: row.source,
      tags: row.tags,
      metadata: row.metadata,
      triggeredAt: row.triggered_at,
      acknowledgedAt: row.acknowledged_at,
      acknowledgedBy: row.acknowledged_by,
      resolvedAt: row.resolved_at,
      resolvedBy: row.resolved_by,
      escalationLevel: row.escalation_level,
      notificationsSent: row.notifications_sent || [],
    };
  }

  private mapAlertRuleRow(row: any): AlertRule {
    return {
      id: row.id,
      name: row.name,
      description: row.description,
      enabled: row.enabled,
      conditionType: row.condition_type,
      condition: row.condition,
      severity: row.severity,
      tags: row.tags,
      escalationPolicyId: row.escalation_policy_id,
      notificationChannels: row.notification_channels,
      cooldownPeriod: row.cooldown_period,
      lastTriggeredAt: row.last_triggered_at,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private mapEscalationPolicyRow(row: any): EscalationPolicy {
    return {
      id: row.id,
      name: row.name,
      description: row.description,
      steps: row.steps,
      repeatAfterMinutes: row.repeat_after_minutes,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
