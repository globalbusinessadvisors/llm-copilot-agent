/**
 * Agent Orchestration Types
 *
 * Types for multi-agent systems, tool calling, and agent collaboration.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum AgentType {
  ASSISTANT = 'assistant',
  RESEARCHER = 'researcher',
  CODER = 'coder',
  REVIEWER = 'reviewer',
  PLANNER = 'planner',
  EXECUTOR = 'executor',
  SUPERVISOR = 'supervisor',
  CUSTOM = 'custom',
}

export enum AgentStatus {
  IDLE = 'idle',
  RUNNING = 'running',
  WAITING = 'waiting',
  COMPLETED = 'completed',
  FAILED = 'failed',
  CANCELLED = 'cancelled',
}

export enum ToolType {
  FUNCTION = 'function',
  API = 'api',
  DATABASE = 'database',
  FILE_SYSTEM = 'file_system',
  CODE_EXECUTION = 'code_execution',
  SEARCH = 'search',
  BROWSER = 'browser',
  CUSTOM = 'custom',
}

export enum CollaborationPattern {
  SEQUENTIAL = 'sequential',
  PARALLEL = 'parallel',
  HIERARCHICAL = 'hierarchical',
  DEBATE = 'debate',
  CONSENSUS = 'consensus',
  SUPERVISOR = 'supervisor',
}

// ===========================================
// Tool Schemas
// ===========================================

export const ToolParameterSchema = z.object({
  name: z.string(),
  type: z.enum(['string', 'number', 'boolean', 'array', 'object']),
  description: z.string(),
  required: z.boolean().default(false),
  default: z.unknown().optional(),
  enum: z.array(z.unknown()).optional(),
  items: z.lazy((): z.ZodTypeAny => ToolParameterSchema).optional(),
  properties: z.record(z.lazy((): z.ZodTypeAny => ToolParameterSchema)).optional(),
});

export const ToolDefinitionSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(64).regex(/^[a-zA-Z_][a-zA-Z0-9_]*$/),
  displayName: z.string().max(100),
  description: z.string().max(1000),
  type: z.nativeEnum(ToolType),

  // Parameters schema (OpenAI function format)
  parameters: z.object({
    type: z.literal('object'),
    properties: z.record(ToolParameterSchema),
    required: z.array(z.string()).default([]),
  }),

  // Return type description
  returns: z.object({
    type: z.string(),
    description: z.string(),
  }).optional(),

  // Execution config
  execution: z.object({
    endpoint: z.string().optional(), // For API tools
    handler: z.string().optional(),  // For function tools
    timeout: z.number().default(30000),
    retries: z.number().default(3),
    sandboxed: z.boolean().default(true),
  }),

  // Permissions
  permissions: z.object({
    requiresConfirmation: z.boolean().default(false),
    allowedTenants: z.array(z.string().uuid()).optional(),
    deniedTenants: z.array(z.string().uuid()).optional(),
    rateLimit: z.number().optional(),
  }).default({}),

  // Metadata
  version: z.string().default('1.0.0'),
  tags: z.array(z.string()).default([]),
  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const ToolCallSchema = z.object({
  id: z.string().uuid(),
  toolId: z.string().uuid(),
  toolName: z.string(),
  arguments: z.record(z.unknown()),
  status: z.enum(['pending', 'running', 'completed', 'failed', 'cancelled']),
  result: z.unknown().optional(),
  error: z.string().optional(),
  startedAt: z.date().optional(),
  completedAt: z.date().optional(),
  durationMs: z.number().optional(),
});

// ===========================================
// Agent Schemas
// ===========================================

export const AgentConfigSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  displayName: z.string().max(255),
  description: z.string().optional(),
  type: z.nativeEnum(AgentType),

  // Model configuration
  modelId: z.string().uuid(),
  modelConfig: z.object({
    temperature: z.number().min(0).max(2).optional(),
    maxTokens: z.number().optional(),
    systemPrompt: z.string().optional(),
  }).default({}),

  // Tools available to this agent
  tools: z.array(z.string().uuid()).default([]),

  // Agent capabilities
  capabilities: z.object({
    canUseTools: z.boolean().default(true),
    canDelegateToAgents: z.boolean().default(false),
    canAccessMemory: z.boolean().default(true),
    canAccessContext: z.boolean().default(true),
    maxToolCalls: z.number().default(10),
    maxDelegations: z.number().default(5),
  }),

  // Memory configuration
  memory: z.object({
    type: z.enum(['none', 'buffer', 'summary', 'vector']).default('buffer'),
    maxMessages: z.number().default(20),
    includeSystemMessages: z.boolean().default(true),
  }),

  // Behavior
  behavior: z.object({
    maxIterations: z.number().default(10),
    stopOnToolError: z.boolean().default(false),
    returnIntermediateSteps: z.boolean().default(false),
    verbose: z.boolean().default(false),
  }),

  // Metadata
  version: z.string().default('1.0.0'),
  tags: z.array(z.string()).default([]),
  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export const AgentExecutionSchema = z.object({
  id: z.string().uuid(),
  agentId: z.string().uuid(),
  sessionId: z.string().uuid().optional(),
  status: z.nativeEnum(AgentStatus),

  // Input
  input: z.object({
    message: z.string(),
    context: z.record(z.unknown()).optional(),
    tools: z.array(z.string()).optional(),
  }),

  // Execution steps
  steps: z.array(z.object({
    stepNumber: z.number(),
    type: z.enum(['thought', 'tool_call', 'tool_result', 'delegation', 'response']),
    content: z.unknown(),
    timestamp: z.date(),
  })),

  // Output
  output: z.object({
    response: z.string().optional(),
    toolCalls: z.array(ToolCallSchema).optional(),
    delegations: z.array(z.string().uuid()).optional(),
  }).optional(),

  // Metrics
  metrics: z.object({
    inputTokens: z.number(),
    outputTokens: z.number(),
    totalTokens: z.number(),
    toolCalls: z.number(),
    iterations: z.number(),
    durationMs: z.number(),
    cost: z.number().optional(),
  }).optional(),

  // Error info
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.unknown().optional(),
  }).optional(),

  startedAt: z.date(),
  completedAt: z.date().optional(),
  createdBy: z.string(),
});

// ===========================================
// Multi-Agent Collaboration Schemas
// ===========================================

export const AgentTeamSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),

  // Team composition
  agents: z.array(z.object({
    agentId: z.string().uuid(),
    role: z.string(),
    priority: z.number().default(0),
  })),

  // Collaboration pattern
  pattern: z.nativeEnum(CollaborationPattern),

  // Pattern-specific config
  patternConfig: z.object({
    // Sequential/Parallel
    maxConcurrent: z.number().optional(),
    orderStrategy: z.enum(['priority', 'round_robin', 'random']).optional(),

    // Hierarchical
    supervisorAgentId: z.string().uuid().optional(),
    delegationRules: z.array(z.object({
      condition: z.string(),
      targetAgentId: z.string().uuid(),
    })).optional(),

    // Debate/Consensus
    maxRounds: z.number().optional(),
    consensusThreshold: z.number().optional(),
    tieBreaker: z.enum(['supervisor', 'voting', 'random']).optional(),
  }).default({}),

  // Shared context
  sharedContext: z.object({
    enabled: z.boolean().default(true),
    shareToolResults: z.boolean().default(true),
    shareResponses: z.boolean().default(true),
  }),

  // Termination conditions
  termination: z.object({
    maxIterations: z.number().default(20),
    maxTokens: z.number().optional(),
    maxDuration: z.number().optional(), // seconds
    stopPhrases: z.array(z.string()).optional(),
  }),

  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export const TeamExecutionSchema = z.object({
  id: z.string().uuid(),
  teamId: z.string().uuid(),
  sessionId: z.string().uuid().optional(),
  status: z.nativeEnum(AgentStatus),

  // Input
  input: z.object({
    task: z.string(),
    context: z.record(z.unknown()).optional(),
  }),

  // Agent executions within this team execution
  agentExecutions: z.array(z.object({
    executionId: z.string().uuid(),
    agentId: z.string().uuid(),
    role: z.string(),
    order: z.number(),
  })),

  // Collaboration log
  collaborationLog: z.array(z.object({
    timestamp: z.date(),
    type: z.enum(['delegation', 'message', 'tool_share', 'vote', 'consensus', 'intervention']),
    fromAgentId: z.string().uuid().optional(),
    toAgentId: z.string().uuid().optional(),
    content: z.unknown(),
  })),

  // Final output
  output: z.object({
    response: z.string().optional(),
    artifacts: z.array(z.object({
      type: z.string(),
      content: z.unknown(),
      producedBy: z.string().uuid(),
    })).optional(),
    consensus: z.object({
      reached: z.boolean(),
      rounds: z.number(),
      votes: z.record(z.number()),
    }).optional(),
  }).optional(),

  // Aggregate metrics
  metrics: z.object({
    totalTokens: z.number(),
    totalToolCalls: z.number(),
    totalIterations: z.number(),
    totalDurationMs: z.number(),
    totalCost: z.number().optional(),
    agentMetrics: z.record(z.object({
      tokens: z.number(),
      toolCalls: z.number(),
      iterations: z.number(),
    })),
  }).optional(),

  error: z.object({
    code: z.string(),
    message: z.string(),
    agentId: z.string().uuid().optional(),
  }).optional(),

  startedAt: z.date(),
  completedAt: z.date().optional(),
  createdBy: z.string(),
});

// ===========================================
// Types
// ===========================================

export type ToolParameter = z.infer<typeof ToolParameterSchema>;
export type ToolDefinition = z.infer<typeof ToolDefinitionSchema>;
export type ToolCall = z.infer<typeof ToolCallSchema>;
export type AgentConfig = z.infer<typeof AgentConfigSchema>;
export type AgentExecution = z.infer<typeof AgentExecutionSchema>;
export type AgentTeam = z.infer<typeof AgentTeamSchema>;
export type TeamExecution = z.infer<typeof TeamExecutionSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateToolInput {
  name: string;
  displayName: string;
  description: string;
  type: ToolType;
  parameters: ToolDefinition['parameters'];
  returns?: ToolDefinition['returns'];
  execution: ToolDefinition['execution'];
  permissions?: Partial<ToolDefinition['permissions']>;
  tags?: string[];
}

export interface CreateAgentInput {
  name: string;
  displayName: string;
  description?: string;
  type: AgentType;
  modelId: string;
  modelConfig?: AgentConfig['modelConfig'];
  tools?: string[];
  capabilities?: Partial<AgentConfig['capabilities']>;
  memory?: Partial<AgentConfig['memory']>;
  behavior?: Partial<AgentConfig['behavior']>;
  tags?: string[];
}

export interface CreateAgentTeamInput {
  name: string;
  description?: string;
  agents: Array<{
    agentId: string;
    role: string;
    priority?: number;
  }>;
  pattern: CollaborationPattern;
  patternConfig?: Partial<AgentTeam['patternConfig']>;
  sharedContext?: Partial<AgentTeam['sharedContext']>;
  termination?: Partial<AgentTeam['termination']>;
}

export interface ExecuteAgentInput {
  message: string;
  context?: Record<string, unknown>;
  tools?: string[];
  sessionId?: string;
}

export interface ExecuteTeamInput {
  task: string;
  context?: Record<string, unknown>;
  sessionId?: string;
}
