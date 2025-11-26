/**
 * Usage Tracking Models
 *
 * Models for tracking API usage, token consumption, storage, and compute time.
 */

import { z } from 'zod';

// ===========================================
// Usage Event Types
// ===========================================

export enum UsageType {
  API_CALL = 'api_call',
  TOKEN_INPUT = 'token_input',
  TOKEN_OUTPUT = 'token_output',
  STORAGE = 'storage',
  COMPUTE = 'compute',
  EMBEDDING = 'embedding',
  WORKFLOW_RUN = 'workflow_run',
  CONTEXT_SEARCH = 'context_search',
}

export enum UsageUnit {
  COUNT = 'count',
  TOKENS = 'tokens',
  BYTES = 'bytes',
  SECONDS = 'seconds',
  MILLISECONDS = 'milliseconds',
}

// ===========================================
// Schemas
// ===========================================

export const UsageEventSchema = z.object({
  id: z.string().uuid(),
  tenantId: z.string().uuid(),
  userId: z.string().uuid().optional(),
  type: z.nativeEnum(UsageType),
  unit: z.nativeEnum(UsageUnit),
  quantity: z.number().positive(),
  metadata: z.record(z.unknown()).optional(),
  resourceId: z.string().optional(),
  resourceType: z.string().optional(),
  model: z.string().optional(),
  endpoint: z.string().optional(),
  statusCode: z.number().optional(),
  timestamp: z.date(),
  billingPeriodStart: z.date(),
  billingPeriodEnd: z.date(),
});

export const UsageSummarySchema = z.object({
  tenantId: z.string().uuid(),
  periodStart: z.date(),
  periodEnd: z.date(),
  apiCalls: z.number(),
  inputTokens: z.number(),
  outputTokens: z.number(),
  storageBytes: z.number(),
  computeSeconds: z.number(),
  embeddingTokens: z.number(),
  workflowRuns: z.number(),
  contextSearches: z.number(),
});

export const UsageQuotaSchema = z.object({
  tenantId: z.string().uuid(),
  apiCallsLimit: z.number().nullable(),
  inputTokensLimit: z.number().nullable(),
  outputTokensLimit: z.number().nullable(),
  storageBytesLimit: z.number().nullable(),
  computeSecondsLimit: z.number().nullable(),
  periodStart: z.date(),
  periodEnd: z.date(),
});

// ===========================================
// Types
// ===========================================

export type UsageEvent = z.infer<typeof UsageEventSchema>;
export type UsageSummary = z.infer<typeof UsageSummarySchema>;
export type UsageQuota = z.infer<typeof UsageQuotaSchema>;

// ===========================================
// Usage Event Creation
// ===========================================

export interface CreateUsageEventInput {
  tenantId: string;
  userId?: string;
  type: UsageType;
  unit: UsageUnit;
  quantity: number;
  metadata?: Record<string, unknown>;
  resourceId?: string;
  resourceType?: string;
  model?: string;
  endpoint?: string;
  statusCode?: number;
}

// ===========================================
// Usage Query Parameters
// ===========================================

export interface UsageQueryParams {
  tenantId: string;
  userId?: string;
  type?: UsageType;
  startDate: Date;
  endDate: Date;
  groupBy?: 'hour' | 'day' | 'week' | 'month';
}

// ===========================================
// Usage Aggregation Results
// ===========================================

export interface UsageAggregation {
  period: string;
  type: UsageType;
  totalQuantity: number;
  count: number;
  avgQuantity: number;
  maxQuantity: number;
  minQuantity: number;
}

export interface TenantUsageReport {
  tenantId: string;
  periodStart: Date;
  periodEnd: Date;
  summary: UsageSummary;
  breakdown: UsageAggregation[];
  quotaUsage: {
    apiCalls: { used: number; limit: number | null; percentage: number | null };
    inputTokens: { used: number; limit: number | null; percentage: number | null };
    outputTokens: { used: number; limit: number | null; percentage: number | null };
    storage: { used: number; limit: number | null; percentage: number | null };
    compute: { used: number; limit: number | null; percentage: number | null };
  };
  estimatedCost: number;
}

// ===========================================
// Pricing Configuration
// ===========================================

export interface PricingTier {
  name: string;
  minQuantity: number;
  maxQuantity: number | null;
  pricePerUnit: number;
}

export interface PricingConfig {
  apiCallPrice: number;
  inputTokenPrice: number;
  outputTokenPrice: number;
  storagePricePerGb: number;
  computePricePerHour: number;
  embeddingTokenPrice: number;
  workflowRunPrice: number;
  contextSearchPrice: number;
  tiers?: {
    apiCalls?: PricingTier[];
    tokens?: PricingTier[];
  };
}

export const DEFAULT_PRICING: PricingConfig = {
  apiCallPrice: 0.0001,           // $0.0001 per API call
  inputTokenPrice: 0.000003,      // $3 per 1M input tokens
  outputTokenPrice: 0.000015,     // $15 per 1M output tokens
  storagePricePerGb: 0.02,        // $0.02 per GB per month
  computePricePerHour: 0.10,      // $0.10 per compute hour
  embeddingTokenPrice: 0.0000001, // $0.10 per 1M embedding tokens
  workflowRunPrice: 0.001,        // $0.001 per workflow run
  contextSearchPrice: 0.0001,     // $0.0001 per context search
};
