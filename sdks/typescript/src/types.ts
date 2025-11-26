/**
 * Core type definitions for the LLM-CoPilot SDK
 */

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * SDK Configuration options
 */
export interface CopilotConfig {
  /** API base URL */
  baseUrl: string;
  /** API key for authentication */
  apiKey: string;
  /** Tenant ID for multi-tenant deployments */
  tenantId?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Maximum number of retries for failed requests */
  maxRetries?: number;
  /** Custom headers to include in all requests */
  headers?: Record<string, string>;
  /** Enable debug logging */
  debug?: boolean;
}

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG: Partial<CopilotConfig> = {
  timeout: 30000,
  maxRetries: 3,
  debug: false,
};

// ============================================================================
// API Response Types
// ============================================================================

/**
 * Standard API response wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: ApiError;
  metadata?: ResponseMetadata;
}

/**
 * API error structure
 */
export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  requestId?: string;
}

/**
 * Response metadata
 */
export interface ResponseMetadata {
  requestId: string;
  processingTimeMs: number;
  rateLimit?: RateLimitInfo;
}

/**
 * Rate limit information
 */
export interface RateLimitInfo {
  limit: number;
  remaining: number;
  resetAt: Date;
}

// ============================================================================
// Pagination Types
// ============================================================================

/**
 * Pagination parameters
 */
export interface PaginationParams {
  page?: number;
  pageSize?: number;
  cursor?: string;
  [key: string]: unknown;
}

/**
 * Paginated response
 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
  nextCursor?: string;
}

// ============================================================================
// Common Entity Types
// ============================================================================

/**
 * Base entity with common fields
 */
export interface BaseEntity {
  id: string;
  createdAt: Date;
  updatedAt: Date;
}

/**
 * Entity with soft delete support
 */
export interface SoftDeletableEntity extends BaseEntity {
  deletedAt?: Date;
  isDeleted: boolean;
}

// ============================================================================
// Message Types
// ============================================================================

/**
 * Role of a message sender
 */
export type MessageRole = 'user' | 'assistant' | 'system' | 'function';

/**
 * A chat message
 */
export interface Message {
  id: string;
  role: MessageRole;
  content: string;
  metadata?: Record<string, unknown>;
  createdAt: Date;
  tokens?: number;
}

/**
 * Message input for creating new messages
 */
export interface MessageInput {
  role: MessageRole;
  content: string;
  metadata?: Record<string, unknown>;
}

// ============================================================================
// Conversation Types
// ============================================================================

/**
 * Conversation status
 */
export type ConversationStatus = 'active' | 'paused' | 'completed' | 'archived';

/**
 * A conversation thread
 */
export interface Conversation extends BaseEntity {
  title?: string;
  status: ConversationStatus;
  messages: Message[];
  metadata?: Record<string, unknown>;
  tokenCount: number;
  messageCount: number;
}

/**
 * Input for creating a conversation
 */
export interface CreateConversationInput {
  title?: string;
  metadata?: Record<string, unknown>;
  initialMessages?: MessageInput[];
}

/**
 * Options for sending a message
 */
export interface SendMessageOptions {
  /** Enable streaming response */
  stream?: boolean;
  /** Temperature for generation */
  temperature?: number;
  /** Maximum tokens to generate */
  maxTokens?: number;
  /** Stop sequences */
  stopSequences?: string[];
  /** Additional context to include */
  context?: ContextItem[];
}

// ============================================================================
// Context Types
// ============================================================================

/**
 * Context item for providing additional information
 */
export interface ContextItem {
  id: string;
  type: 'document' | 'memory' | 'snippet' | 'custom';
  content: string;
  metadata?: Record<string, unknown>;
  relevanceScore?: number;
}

/**
 * Context search parameters
 */
export interface ContextSearchParams {
  query: string;
  limit?: number;
  threshold?: number;
  filters?: Record<string, unknown>;
}

/**
 * Context search result
 */
export interface ContextSearchResult {
  items: ContextItem[];
  totalMatches: number;
  searchTimeMs: number;
}

// ============================================================================
// Workflow Types
// ============================================================================

/**
 * Workflow status
 */
export type WorkflowStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'paused';

/**
 * Workflow step action type
 */
export type StepActionType =
  | 'prompt'
  | 'tool'
  | 'condition'
  | 'parallel'
  | 'loop'
  | 'wait'
  | 'custom';

/**
 * Workflow step definition
 */
export interface WorkflowStep {
  id: string;
  name: string;
  description?: string;
  action: StepActionType;
  config: Record<string, unknown>;
  dependsOn?: string[];
  retryConfig?: RetryConfig;
  timeoutSeconds?: number;
}

/**
 * Retry configuration
 */
export interface RetryConfig {
  maxRetries: number;
  initialDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
}

/**
 * Workflow definition
 */
export interface WorkflowDefinition {
  id: string;
  name: string;
  description?: string;
  version: string;
  steps: WorkflowStep[];
  triggers?: WorkflowTrigger[];
  metadata?: Record<string, unknown>;
}

/**
 * Workflow trigger
 */
export interface WorkflowTrigger {
  type: 'manual' | 'schedule' | 'event' | 'webhook';
  config: Record<string, unknown>;
}

/**
 * Workflow execution
 */
export interface WorkflowExecution extends BaseEntity {
  workflowId: string;
  status: WorkflowStatus;
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
  currentStep?: string;
  stepResults: Record<string, StepResult>;
  error?: string;
  startedAt?: Date;
  completedAt?: Date;
  durationMs?: number;
}

/**
 * Result of a workflow step execution
 */
export interface StepResult {
  stepId: string;
  status: WorkflowStatus;
  output?: unknown;
  error?: string;
  startedAt: Date;
  completedAt?: Date;
  durationMs?: number;
  retryCount?: number;
}

/**
 * Input for creating a workflow
 */
export interface CreateWorkflowInput {
  name: string;
  description?: string;
  steps: Omit<WorkflowStep, 'id'>[];
  triggers?: WorkflowTrigger[];
  metadata?: Record<string, unknown>;
}

/**
 * Input for executing a workflow
 */
export interface ExecuteWorkflowInput {
  workflowId: string;
  input?: Record<string, unknown>;
  async?: boolean;
}

// ============================================================================
// Streaming Types
// ============================================================================

/**
 * Streaming event types
 */
export type StreamEventType =
  | 'message_start'
  | 'content_delta'
  | 'content_stop'
  | 'message_stop'
  | 'error'
  | 'tool_use'
  | 'tool_result';

/**
 * Base streaming event
 */
export interface StreamEvent<T = unknown> {
  type: StreamEventType;
  data: T;
  timestamp: Date;
}

/**
 * Content delta in streaming
 */
export interface ContentDelta {
  text: string;
  index: number;
}

/**
 * Streaming options
 */
export interface StreamOptions {
  /** Callback for each chunk */
  onChunk?: (chunk: string) => void;
  /** Callback for events */
  onEvent?: (event: StreamEvent) => void;
  /** Callback for errors */
  onError?: (error: Error) => void;
  /** Callback when stream completes */
  onComplete?: (message: Message) => void;
  /** Signal for aborting the stream */
  signal?: AbortSignal;
}

// ============================================================================
// Document Ingestion Types
// ============================================================================

/**
 * Supported document types
 */
export type DocumentType =
  | 'text'
  | 'markdown'
  | 'pdf'
  | 'html'
  | 'json'
  | 'code';

/**
 * Document for ingestion
 */
export interface DocumentInput {
  content: string | Buffer;
  filename?: string;
  contentType?: string;
  metadata?: Record<string, unknown>;
}

/**
 * Ingested document
 */
export interface IngestedDocument extends BaseEntity {
  filename?: string;
  contentType: string;
  size: number;
  chunkCount: number;
  status: 'processing' | 'completed' | 'failed';
  error?: string;
  metadata?: Record<string, unknown>;
}

/**
 * Document chunk
 */
export interface DocumentChunk {
  id: string;
  documentId: string;
  content: string;
  index: number;
  tokenCount: number;
  metadata?: Record<string, unknown>;
}

// ============================================================================
// Webhook Types
// ============================================================================

/**
 * Webhook event types
 */
export type WebhookEventType =
  | 'conversation.created'
  | 'conversation.updated'
  | 'message.created'
  | 'workflow.started'
  | 'workflow.completed'
  | 'workflow.failed'
  | 'document.ingested';

/**
 * Webhook configuration
 */
export interface WebhookConfig {
  id: string;
  url: string;
  events: WebhookEventType[];
  secret: string;
  isActive: boolean;
  createdAt: Date;
}

/**
 * Webhook payload
 */
export interface WebhookPayload<T = unknown> {
  id: string;
  type: WebhookEventType;
  timestamp: Date;
  data: T;
}

// ============================================================================
// Utility Types
// ============================================================================

/**
 * Make all properties of T optional recursively
 */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

/**
 * Extract the resolved type from a Promise
 */
export type Awaited<T> = T extends Promise<infer U> ? U : T;

/**
 * Callback type for event handlers
 */
export type EventHandler<T = void> = (data: T) => void | Promise<void>;
