/**
 * Governance Models
 *
 * Type definitions for content filtering, policies, audit trail, and data lineage.
 */

import { z } from 'zod';

// ===========================================
// Content Filtering Types
// ===========================================

export enum ContentCategory {
  HATE_SPEECH = 'hate_speech',
  VIOLENCE = 'violence',
  SEXUAL = 'sexual',
  SELF_HARM = 'self_harm',
  HARASSMENT = 'harassment',
  DANGEROUS = 'dangerous',
  PII = 'pii',
  CONFIDENTIAL = 'confidential',
  LEGAL = 'legal',
  MEDICAL = 'medical',
  FINANCIAL = 'financial',
  CUSTOM = 'custom',
}

export enum FilterAction {
  BLOCK = 'block',
  WARN = 'warn',
  REDACT = 'redact',
  FLAG = 'flag',
  LOG = 'log',
  ALLOW = 'allow',
}

export enum FilterDirection {
  INPUT = 'input',
  OUTPUT = 'output',
  BOTH = 'both',
}

export const ContentFilterRuleSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  category: z.nativeEnum(ContentCategory),
  direction: z.nativeEnum(FilterDirection),
  action: z.nativeEnum(FilterAction),
  priority: z.number().min(0).max(100),
  conditions: z.object({
    patterns: z.array(z.string()).optional(),
    keywords: z.array(z.string()).optional(),
    threshold: z.number().min(0).max(1).optional(),
    customLogic: z.string().optional(),
  }),
  exceptions: z.object({
    users: z.array(z.string()).optional(),
    roles: z.array(z.string()).optional(),
    contexts: z.array(z.string()).optional(),
  }).optional(),
  redactionConfig: z.object({
    replacement: z.string().optional(),
    maskChar: z.string().optional(),
    preserveLength: z.boolean().optional(),
  }).optional(),
  enabled: z.boolean(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export type ContentFilterRule = z.infer<typeof ContentFilterRuleSchema>;

export const ContentFilterResultSchema = z.object({
  id: z.string().uuid(),
  ruleId: z.string().uuid().optional(),
  content: z.string(),
  direction: z.nativeEnum(FilterDirection),
  category: z.nativeEnum(ContentCategory).optional(),
  action: z.nativeEnum(FilterAction),
  confidence: z.number().min(0).max(1).optional(),
  matches: z.array(z.object({
    pattern: z.string(),
    location: z.object({
      start: z.number(),
      end: z.number(),
    }),
    text: z.string(),
  })).optional(),
  redactedContent: z.string().optional(),
  metadata: z.record(z.unknown()).optional(),
  processedAt: z.date(),
});

export type ContentFilterResult = z.infer<typeof ContentFilterResultSchema>;

// ===========================================
// Policy Management Types
// ===========================================

export enum PolicyType {
  USAGE = 'usage',
  ACCESS = 'access',
  DATA_HANDLING = 'data_handling',
  RETENTION = 'retention',
  SHARING = 'sharing',
  CONTENT = 'content',
  RATE_LIMIT = 'rate_limit',
  COST = 'cost',
  CUSTOM = 'custom',
}

export enum PolicyScope {
  GLOBAL = 'global',
  ORGANIZATION = 'organization',
  TEAM = 'team',
  PROJECT = 'project',
  USER = 'user',
  API = 'api',
  MODEL = 'model',
}

export enum PolicyEnforcement {
  STRICT = 'strict',
  PERMISSIVE = 'permissive',
  AUDIT_ONLY = 'audit_only',
}

export const PolicySchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  type: z.nativeEnum(PolicyType),
  scope: z.nativeEnum(PolicyScope),
  enforcement: z.nativeEnum(PolicyEnforcement),
  rules: z.array(z.object({
    id: z.string(),
    condition: z.string(),
    action: z.string(),
    parameters: z.record(z.unknown()).optional(),
  })),
  targets: z.object({
    organizations: z.array(z.string()).optional(),
    teams: z.array(z.string()).optional(),
    users: z.array(z.string()).optional(),
    projects: z.array(z.string()).optional(),
    apis: z.array(z.string()).optional(),
    models: z.array(z.string()).optional(),
  }).optional(),
  exceptions: z.array(z.object({
    type: z.string(),
    value: z.string(),
    reason: z.string().optional(),
    expiresAt: z.date().optional(),
  })).optional(),
  version: z.number(),
  status: z.enum(['draft', 'active', 'deprecated', 'archived']),
  effectiveDate: z.date().optional(),
  expirationDate: z.date().optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export type Policy = z.infer<typeof PolicySchema>;

export const PolicyViolationSchema = z.object({
  id: z.string().uuid(),
  policyId: z.string().uuid(),
  policyName: z.string(),
  ruleId: z.string(),
  userId: z.string(),
  action: z.string(),
  resource: z.object({
    type: z.string(),
    id: z.string(),
    name: z.string().optional(),
  }),
  violationType: z.enum(['hard', 'soft']),
  blocked: z.boolean(),
  details: z.string(),
  context: z.record(z.unknown()).optional(),
  timestamp: z.date(),
});

export type PolicyViolation = z.infer<typeof PolicyViolationSchema>;

// ===========================================
// Audit Trail Types
// ===========================================

export enum AuditEventType {
  // Authentication
  LOGIN = 'login',
  LOGOUT = 'logout',
  LOGIN_FAILED = 'login_failed',
  PASSWORD_CHANGE = 'password_change',
  MFA_ENABLED = 'mfa_enabled',

  // Authorization
  ACCESS_GRANTED = 'access_granted',
  ACCESS_DENIED = 'access_denied',
  PERMISSION_CHANGE = 'permission_change',
  ROLE_ASSIGNED = 'role_assigned',

  // Data Operations
  CREATE = 'create',
  READ = 'read',
  UPDATE = 'update',
  DELETE = 'delete',
  EXPORT = 'export',
  IMPORT = 'import',

  // AI Operations
  MODEL_INVOCATION = 'model_invocation',
  PROMPT_SUBMITTED = 'prompt_submitted',
  RESPONSE_GENERATED = 'response_generated',
  AGENT_EXECUTION = 'agent_execution',
  TOOL_CALL = 'tool_call',

  // Administrative
  CONFIG_CHANGE = 'config_change',
  POLICY_CHANGE = 'policy_change',
  USER_CREATED = 'user_created',
  USER_DELETED = 'user_deleted',

  // Security
  SECURITY_ALERT = 'security_alert',
  ANOMALY_DETECTED = 'anomaly_detected',
  BREACH_ATTEMPT = 'breach_attempt',

  // Custom
  CUSTOM = 'custom',
}

export enum AuditSeverity {
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  CRITICAL = 'critical',
}

export const AuditEventSchema = z.object({
  id: z.string().uuid(),
  type: z.nativeEnum(AuditEventType),
  severity: z.nativeEnum(AuditSeverity),
  actor: z.object({
    type: z.enum(['user', 'service', 'system', 'api_key']),
    id: z.string(),
    name: z.string().optional(),
    email: z.string().email().optional(),
    ip: z.string().optional(),
    userAgent: z.string().optional(),
  }),
  action: z.string(),
  resource: z.object({
    type: z.string(),
    id: z.string(),
    name: z.string().optional(),
  }).optional(),
  outcome: z.enum(['success', 'failure', 'partial']),
  details: z.record(z.unknown()).optional(),
  metadata: z.object({
    sessionId: z.string().optional(),
    requestId: z.string().optional(),
    correlationId: z.string().optional(),
    source: z.string().optional(),
    version: z.string().optional(),
  }).optional(),
  timestamp: z.date(),
});

export type AuditEvent = z.infer<typeof AuditEventSchema>;

// ===========================================
// Data Lineage Types
// ===========================================

export enum LineageNodeType {
  DATA_SOURCE = 'data_source',
  TRANSFORMATION = 'transformation',
  MODEL = 'model',
  PIPELINE = 'pipeline',
  OUTPUT = 'output',
  STORAGE = 'storage',
  API = 'api',
  USER_INTERACTION = 'user_interaction',
}

export enum LineageEdgeType {
  DERIVES_FROM = 'derives_from',
  TRANSFORMS_TO = 'transforms_to',
  FEEDS_INTO = 'feeds_into',
  GENERATES = 'generates',
  COPIES_TO = 'copies_to',
  REFERENCES = 'references',
}

export const LineageNodeSchema = z.object({
  id: z.string().uuid(),
  type: z.nativeEnum(LineageNodeType),
  name: z.string(),
  description: z.string().optional(),
  source: z.object({
    system: z.string(),
    identifier: z.string(),
    version: z.string().optional(),
  }),
  schema: z.object({
    fields: z.array(z.object({
      name: z.string(),
      type: z.string(),
      description: z.string().optional(),
      sensitive: z.boolean().optional(),
    })).optional(),
  }).optional(),
  metadata: z.record(z.unknown()).optional(),
  tags: z.array(z.string()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export type LineageNode = z.infer<typeof LineageNodeSchema>;

export const LineageEdgeSchema = z.object({
  id: z.string().uuid(),
  type: z.nativeEnum(LineageEdgeType),
  sourceNodeId: z.string().uuid(),
  targetNodeId: z.string().uuid(),
  transformation: z.object({
    type: z.string(),
    logic: z.string().optional(),
    parameters: z.record(z.unknown()).optional(),
  }).optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
});

export type LineageEdge = z.infer<typeof LineageEdgeSchema>;

export const DataLineageGraphSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  nodes: z.array(LineageNodeSchema),
  edges: z.array(LineageEdgeSchema),
  rootNodeId: z.string().uuid().optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export type DataLineageGraph = z.infer<typeof DataLineageGraphSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateContentFilterRuleInput {
  name: string;
  description?: string;
  category: ContentCategory;
  direction: FilterDirection;
  action: FilterAction;
  priority?: number;
  conditions: ContentFilterRule['conditions'];
  exceptions?: ContentFilterRule['exceptions'];
  redactionConfig?: ContentFilterRule['redactionConfig'];
  enabled?: boolean;
  metadata?: Record<string, unknown>;
}

export interface FilterContentInput {
  content: string;
  direction: FilterDirection;
  userId?: string;
  context?: Record<string, unknown>;
}

export interface CreatePolicyInput {
  name: string;
  description?: string;
  type: PolicyType;
  scope: PolicyScope;
  enforcement: PolicyEnforcement;
  rules: Policy['rules'];
  targets?: Policy['targets'];
  exceptions?: Policy['exceptions'];
  effectiveDate?: Date;
  expirationDate?: Date;
  metadata?: Record<string, unknown>;
}

export interface EvaluatePolicyInput {
  userId: string;
  action: string;
  resource: {
    type: string;
    id: string;
    name?: string;
  };
  context?: Record<string, unknown>;
}

export interface CreateAuditEventInput {
  type: AuditEventType;
  severity?: AuditSeverity;
  actor: AuditEvent['actor'];
  action: string;
  resource?: AuditEvent['resource'];
  outcome: AuditEvent['outcome'];
  details?: Record<string, unknown>;
  metadata?: AuditEvent['metadata'];
}

export interface CreateLineageNodeInput {
  type: LineageNodeType;
  name: string;
  description?: string;
  source: LineageNode['source'];
  schema?: LineageNode['schema'];
  metadata?: Record<string, unknown>;
  tags?: string[];
}

export interface CreateLineageEdgeInput {
  type: LineageEdgeType;
  sourceNodeId: string;
  targetNodeId: string;
  transformation?: LineageEdge['transformation'];
  metadata?: Record<string, unknown>;
}
