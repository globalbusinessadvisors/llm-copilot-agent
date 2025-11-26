/**
 * Alert Models
 *
 * Models for alerts, rules, escalation policies, and on-call schedules.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum AlertSeverity {
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  CRITICAL = 'critical',
}

export enum AlertStatus {
  TRIGGERED = 'triggered',
  ACKNOWLEDGED = 'acknowledged',
  RESOLVED = 'resolved',
  SUPPRESSED = 'suppressed',
}

export enum AlertChannel {
  EMAIL = 'email',
  SLACK = 'slack',
  PAGERDUTY = 'pagerduty',
  WEBHOOK = 'webhook',
  SMS = 'sms',
}

export enum AlertConditionType {
  THRESHOLD = 'threshold',
  RATE_OF_CHANGE = 'rate_of_change',
  ABSENCE = 'absence',
  ANOMALY = 'anomaly',
}

// ===========================================
// Schemas
// ===========================================

export const AlertSchema = z.object({
  id: z.string().uuid(),
  ruleId: z.string().uuid(),
  title: z.string(),
  description: z.string(),
  severity: z.nativeEnum(AlertSeverity),
  status: z.nativeEnum(AlertStatus),
  source: z.string(),
  tags: z.record(z.string()).default({}),
  metadata: z.record(z.unknown()).default({}),
  triggeredAt: z.date(),
  acknowledgedAt: z.date().optional(),
  acknowledgedBy: z.string().optional(),
  resolvedAt: z.date().optional(),
  resolvedBy: z.string().optional(),
  escalationLevel: z.number().default(0),
  notificationsSent: z.array(z.object({
    channel: z.nativeEnum(AlertChannel),
    recipient: z.string(),
    sentAt: z.date(),
    success: z.boolean(),
  })).default([]),
});

export const AlertRuleSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  enabled: z.boolean().default(true),
  conditionType: z.nativeEnum(AlertConditionType),
  condition: z.object({
    metric: z.string(),
    operator: z.enum(['gt', 'gte', 'lt', 'lte', 'eq', 'neq']),
    threshold: z.number(),
    duration: z.number().optional(), // seconds
    aggregation: z.enum(['avg', 'sum', 'min', 'max', 'count']).optional(),
  }),
  severity: z.nativeEnum(AlertSeverity),
  tags: z.record(z.string()).default({}),
  escalationPolicyId: z.string().uuid().optional(),
  notificationChannels: z.array(z.nativeEnum(AlertChannel)),
  cooldownPeriod: z.number().default(300), // seconds
  lastTriggeredAt: z.date().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const EscalationPolicySchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  steps: z.array(z.object({
    order: z.number(),
    delayMinutes: z.number(),
    targets: z.array(z.object({
      type: z.enum(['user', 'schedule', 'webhook']),
      id: z.string(),
      channels: z.array(z.nativeEnum(AlertChannel)),
    })),
  })),
  repeatAfterMinutes: z.number().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const OnCallScheduleSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  timezone: z.string().default('UTC'),
  rotations: z.array(z.object({
    id: z.string().uuid(),
    name: z.string(),
    type: z.enum(['daily', 'weekly', 'custom']),
    startTime: z.string(), // HH:mm format
    handoffTime: z.string(), // HH:mm format
    users: z.array(z.string().uuid()),
    restrictions: z.array(z.object({
      type: z.enum(['time_of_day', 'day_of_week']),
      startTime: z.string().optional(),
      endTime: z.string().optional(),
      daysOfWeek: z.array(z.number()).optional(), // 0-6, Sunday = 0
    })).optional(),
  })),
  overrides: z.array(z.object({
    id: z.string().uuid(),
    userId: z.string().uuid(),
    startAt: z.date(),
    endAt: z.date(),
  })).default([]),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const OnCallUserSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  email: z.string().email(),
  phone: z.string().optional(),
  slackUserId: z.string().optional(),
  notificationPreferences: z.object({
    email: z.boolean().default(true),
    slack: z.boolean().default(true),
    sms: z.boolean().default(false),
    phone: z.boolean().default(false),
  }),
  createdAt: z.date(),
  updatedAt: z.date(),
});

// ===========================================
// Types
// ===========================================

export type Alert = z.infer<typeof AlertSchema>;
export type AlertRule = z.infer<typeof AlertRuleSchema>;
export type EscalationPolicy = z.infer<typeof EscalationPolicySchema>;
export type OnCallSchedule = z.infer<typeof OnCallScheduleSchema>;
export type OnCallUser = z.infer<typeof OnCallUserSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateAlertInput {
  ruleId: string;
  title: string;
  description: string;
  severity: AlertSeverity;
  source: string;
  tags?: Record<string, string>;
  metadata?: Record<string, unknown>;
}

export interface CreateAlertRuleInput {
  name: string;
  description?: string;
  conditionType: AlertConditionType;
  condition: {
    metric: string;
    operator: 'gt' | 'gte' | 'lt' | 'lte' | 'eq' | 'neq';
    threshold: number;
    duration?: number;
    aggregation?: 'avg' | 'sum' | 'min' | 'max' | 'count';
  };
  severity: AlertSeverity;
  tags?: Record<string, string>;
  escalationPolicyId?: string;
  notificationChannels: AlertChannel[];
  cooldownPeriod?: number;
}

export interface CreateEscalationPolicyInput {
  name: string;
  description?: string;
  steps: Array<{
    order: number;
    delayMinutes: number;
    targets: Array<{
      type: 'user' | 'schedule' | 'webhook';
      id: string;
      channels: AlertChannel[];
    }>;
  }>;
  repeatAfterMinutes?: number;
}

// ===========================================
// Alert Summary Types
// ===========================================

export interface AlertSummary {
  total: number;
  byStatus: Record<AlertStatus, number>;
  bySeverity: Record<AlertSeverity, number>;
  mttr: number; // Mean time to resolve (seconds)
  mtta: number; // Mean time to acknowledge (seconds)
}

export interface CurrentOnCall {
  scheduleId: string;
  scheduleName: string;
  user: OnCallUser;
  startedAt: Date;
  endsAt: Date;
}
