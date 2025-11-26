/**
 * Model Management Types
 *
 * Types for model versioning, deployment, and monitoring.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum ModelProvider {
  OPENAI = 'openai',
  ANTHROPIC = 'anthropic',
  GROQ = 'groq',
  COHERE = 'cohere',
  HUGGINGFACE = 'huggingface',
  AZURE_OPENAI = 'azure_openai',
  BEDROCK = 'bedrock',
  CUSTOM = 'custom',
}

export enum ModelType {
  CHAT = 'chat',
  COMPLETION = 'completion',
  EMBEDDING = 'embedding',
  IMAGE = 'image',
  AUDIO = 'audio',
  MULTIMODAL = 'multimodal',
}

export enum ModelStatus {
  DRAFT = 'draft',
  TESTING = 'testing',
  STAGING = 'staging',
  PRODUCTION = 'production',
  DEPRECATED = 'deprecated',
  DISABLED = 'disabled',
}

export enum DeploymentStrategy {
  ROLLING = 'rolling',
  BLUE_GREEN = 'blue_green',
  CANARY = 'canary',
  SHADOW = 'shadow',
}

// ===========================================
// Schemas
// ===========================================

export const ModelConfigSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  displayName: z.string().max(255),
  description: z.string().optional(),
  provider: z.nativeEnum(ModelProvider),
  type: z.nativeEnum(ModelType),
  modelId: z.string(), // Provider's model ID (e.g., "gpt-4-turbo")
  version: z.string(),
  status: z.nativeEnum(ModelStatus),

  // Capabilities
  capabilities: z.object({
    maxTokens: z.number(),
    contextWindow: z.number(),
    supportsStreaming: z.boolean(),
    supportsTools: z.boolean(),
    supportsVision: z.boolean(),
    supportsFunctionCalling: z.boolean(),
    supportsJson: z.boolean(),
  }),

  // Configuration
  config: z.object({
    temperature: z.number().min(0).max(2).default(0.7),
    topP: z.number().min(0).max(1).optional(),
    topK: z.number().optional(),
    frequencyPenalty: z.number().min(-2).max(2).optional(),
    presencePenalty: z.number().min(-2).max(2).optional(),
    stopSequences: z.array(z.string()).optional(),
    systemPrompt: z.string().optional(),
  }),

  // Pricing (per 1M tokens)
  pricing: z.object({
    inputCostPer1M: z.number(),
    outputCostPer1M: z.number(),
    currency: z.string().default('USD'),
  }),

  // Rate limits
  rateLimits: z.object({
    requestsPerMinute: z.number(),
    tokensPerMinute: z.number(),
    requestsPerDay: z.number().optional(),
  }),

  // Metadata
  tags: z.array(z.string()).default([]),
  metadata: z.record(z.unknown()).default({}),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export const ModelVersionSchema = z.object({
  id: z.string().uuid(),
  modelId: z.string().uuid(),
  version: z.string(),
  parentVersion: z.string().optional(),
  status: z.nativeEnum(ModelStatus),

  // Version-specific config overrides
  configOverrides: z.record(z.unknown()).default({}),

  // Performance metrics at deployment
  baselineMetrics: z.object({
    avgLatency: z.number().optional(),
    p95Latency: z.number().optional(),
    p99Latency: z.number().optional(),
    avgTokensPerSecond: z.number().optional(),
    errorRate: z.number().optional(),
  }).optional(),

  // Deployment info
  deployedAt: z.date().optional(),
  deployedBy: z.string().optional(),
  rollbackVersion: z.string().optional(),

  // Changelog
  changelog: z.string().optional(),

  createdAt: z.date(),
  updatedAt: z.date(),
});

export const ModelDeploymentSchema = z.object({
  id: z.string().uuid(),
  modelId: z.string().uuid(),
  versionId: z.string().uuid(),
  environment: z.enum(['development', 'staging', 'production']),
  strategy: z.nativeEnum(DeploymentStrategy),

  // Traffic allocation
  trafficPercent: z.number().min(0).max(100),

  // Canary config
  canaryConfig: z.object({
    targetPercent: z.number(),
    incrementPercent: z.number(),
    intervalMinutes: z.number(),
    errorThreshold: z.number(),
    latencyThreshold: z.number(),
  }).optional(),

  status: z.enum(['pending', 'in_progress', 'completed', 'failed', 'rolled_back']),
  startedAt: z.date().optional(),
  completedAt: z.date().optional(),

  createdAt: z.date(),
  createdBy: z.string(),
});

// ===========================================
// A/B Testing Types
// ===========================================

export const ABTestSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),
  status: z.enum(['draft', 'running', 'paused', 'completed', 'cancelled']),

  // Variants
  variants: z.array(z.object({
    id: z.string().uuid(),
    name: z.string(),
    modelId: z.string().uuid(),
    versionId: z.string().uuid().optional(),
    trafficPercent: z.number().min(0).max(100),
    isControl: z.boolean().default(false),
  })),

  // Targeting
  targeting: z.object({
    tenantIds: z.array(z.string().uuid()).optional(),
    userIds: z.array(z.string().uuid()).optional(),
    userPercentage: z.number().min(0).max(100).optional(),
    conditions: z.array(z.object({
      field: z.string(),
      operator: z.enum(['eq', 'neq', 'gt', 'gte', 'lt', 'lte', 'in', 'nin', 'contains']),
      value: z.unknown(),
    })).optional(),
  }).default({}),

  // Success metrics
  primaryMetric: z.enum(['latency', 'quality_score', 'error_rate', 'cost', 'user_rating', 'custom']),
  secondaryMetrics: z.array(z.string()).default([]),
  minimumSampleSize: z.number().default(100),
  confidenceLevel: z.number().min(0.8).max(0.99).default(0.95),

  // Schedule
  startAt: z.date().optional(),
  endAt: z.date().optional(),

  // Results
  results: z.object({
    totalSamples: z.number(),
    variantResults: z.record(z.object({
      samples: z.number(),
      metrics: z.record(z.number()),
    })),
    winner: z.string().optional(),
    statisticalSignificance: z.number().optional(),
    conclusionAt: z.date().optional(),
  }).optional(),

  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

// ===========================================
// Model Performance Monitoring
// ===========================================

export const ModelMetricsSchema = z.object({
  id: z.string().uuid(),
  modelId: z.string().uuid(),
  versionId: z.string().uuid().optional(),
  timestamp: z.date(),
  period: z.enum(['minute', 'hour', 'day']),

  // Request metrics
  requests: z.object({
    total: z.number(),
    successful: z.number(),
    failed: z.number(),
    cached: z.number(),
  }),

  // Latency metrics (ms)
  latency: z.object({
    avg: z.number(),
    p50: z.number(),
    p90: z.number(),
    p95: z.number(),
    p99: z.number(),
    max: z.number(),
  }),

  // Token metrics
  tokens: z.object({
    inputTotal: z.number(),
    outputTotal: z.number(),
    avgInputPerRequest: z.number(),
    avgOutputPerRequest: z.number(),
  }),

  // Cost metrics
  cost: z.object({
    total: z.number(),
    inputCost: z.number(),
    outputCost: z.number(),
  }),

  // Quality metrics (if available)
  quality: z.object({
    avgScore: z.number().optional(),
    userRatings: z.number().optional(),
    avgRating: z.number().optional(),
  }).optional(),

  // Error breakdown
  errors: z.record(z.number()).default({}),
});

// ===========================================
// Fine-tuning Types
// ===========================================

export const FineTuneJobSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  baseModelId: z.string().uuid(),
  status: z.enum(['pending', 'preparing', 'training', 'validating', 'completed', 'failed', 'cancelled']),

  // Training data
  trainingData: z.object({
    fileId: z.string(),
    format: z.enum(['jsonl', 'csv', 'parquet']),
    samples: z.number(),
    validationSplit: z.number().min(0).max(0.5).default(0.1),
  }),

  // Hyperparameters
  hyperparameters: z.object({
    epochs: z.number().min(1).max(100).default(3),
    batchSize: z.number().optional(),
    learningRate: z.number().optional(),
    warmupSteps: z.number().optional(),
  }),

  // Progress
  progress: z.object({
    currentEpoch: z.number().optional(),
    totalSteps: z.number().optional(),
    completedSteps: z.number().optional(),
    trainingLoss: z.number().optional(),
    validationLoss: z.number().optional(),
  }).optional(),

  // Result
  resultModelId: z.string().uuid().optional(),
  providerJobId: z.string().optional(),

  // Timing
  startedAt: z.date().optional(),
  completedAt: z.date().optional(),
  estimatedCompletion: z.date().optional(),

  // Cost
  estimatedCost: z.number().optional(),
  actualCost: z.number().optional(),

  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

// ===========================================
// Types
// ===========================================

export type ModelConfig = z.infer<typeof ModelConfigSchema>;
export type ModelVersion = z.infer<typeof ModelVersionSchema>;
export type ModelDeployment = z.infer<typeof ModelDeploymentSchema>;
export type ABTest = z.infer<typeof ABTestSchema>;
export type ModelMetrics = z.infer<typeof ModelMetricsSchema>;
export type FineTuneJob = z.infer<typeof FineTuneJobSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateModelInput {
  name: string;
  displayName: string;
  description?: string;
  provider: ModelProvider;
  type: ModelType;
  modelId: string;
  version: string;
  capabilities: ModelConfig['capabilities'];
  config?: Partial<ModelConfig['config']>;
  pricing: ModelConfig['pricing'];
  rateLimits: ModelConfig['rateLimits'];
  tags?: string[];
  metadata?: Record<string, unknown>;
}

export interface CreateABTestInput {
  name: string;
  description?: string;
  variants: Array<{
    name: string;
    modelId: string;
    versionId?: string;
    trafficPercent: number;
    isControl?: boolean;
  }>;
  targeting?: ABTest['targeting'];
  primaryMetric: ABTest['primaryMetric'];
  secondaryMetrics?: string[];
  minimumSampleSize?: number;
  confidenceLevel?: number;
  startAt?: Date;
  endAt?: Date;
}

export interface CreateFineTuneJobInput {
  name: string;
  baseModelId: string;
  trainingData: FineTuneJob['trainingData'];
  hyperparameters?: Partial<FineTuneJob['hyperparameters']>;
}
