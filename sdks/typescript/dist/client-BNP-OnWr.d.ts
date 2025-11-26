/**
 * Core type definitions for the LLM-CoPilot SDK
 */
/**
 * SDK Configuration options
 */
interface CopilotConfig {
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
declare const DEFAULT_CONFIG: Partial<CopilotConfig>;
/**
 * Standard API response wrapper
 */
interface ApiResponse<T> {
    success: boolean;
    data?: T;
    error?: ApiError;
    metadata?: ResponseMetadata;
}
/**
 * API error structure
 */
interface ApiError {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    requestId?: string;
}
/**
 * Response metadata
 */
interface ResponseMetadata {
    requestId: string;
    processingTimeMs: number;
    rateLimit?: RateLimitInfo;
}
/**
 * Rate limit information
 */
interface RateLimitInfo {
    limit: number;
    remaining: number;
    resetAt: Date;
}
/**
 * Pagination parameters
 */
interface PaginationParams {
    page?: number;
    pageSize?: number;
    cursor?: string;
    [key: string]: unknown;
}
/**
 * Paginated response
 */
interface PaginatedResponse<T> {
    items: T[];
    total: number;
    page: number;
    pageSize: number;
    hasMore: boolean;
    nextCursor?: string;
}
/**
 * Base entity with common fields
 */
interface BaseEntity {
    id: string;
    createdAt: Date;
    updatedAt: Date;
}
/**
 * Entity with soft delete support
 */
interface SoftDeletableEntity extends BaseEntity {
    deletedAt?: Date;
    isDeleted: boolean;
}
/**
 * Role of a message sender
 */
type MessageRole = 'user' | 'assistant' | 'system' | 'function';
/**
 * A chat message
 */
interface Message {
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
interface MessageInput {
    role: MessageRole;
    content: string;
    metadata?: Record<string, unknown>;
}
/**
 * Conversation status
 */
type ConversationStatus = 'active' | 'paused' | 'completed' | 'archived';
/**
 * A conversation thread
 */
interface Conversation extends BaseEntity {
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
interface CreateConversationInput {
    title?: string;
    metadata?: Record<string, unknown>;
    initialMessages?: MessageInput[];
}
/**
 * Options for sending a message
 */
interface SendMessageOptions {
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
/**
 * Context item for providing additional information
 */
interface ContextItem {
    id: string;
    type: 'document' | 'memory' | 'snippet' | 'custom';
    content: string;
    metadata?: Record<string, unknown>;
    relevanceScore?: number;
}
/**
 * Context search parameters
 */
interface ContextSearchParams {
    query: string;
    limit?: number;
    threshold?: number;
    filters?: Record<string, unknown>;
}
/**
 * Context search result
 */
interface ContextSearchResult {
    items: ContextItem[];
    totalMatches: number;
    searchTimeMs: number;
}
/**
 * Workflow status
 */
type WorkflowStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'paused';
/**
 * Workflow step action type
 */
type StepActionType = 'prompt' | 'tool' | 'condition' | 'parallel' | 'loop' | 'wait' | 'custom';
/**
 * Workflow step definition
 */
interface WorkflowStep {
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
interface RetryConfig {
    maxRetries: number;
    initialDelayMs: number;
    maxDelayMs: number;
    backoffMultiplier: number;
}
/**
 * Workflow definition
 */
interface WorkflowDefinition {
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
interface WorkflowTrigger {
    type: 'manual' | 'schedule' | 'event' | 'webhook';
    config: Record<string, unknown>;
}
/**
 * Workflow execution
 */
interface WorkflowExecution extends BaseEntity {
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
interface StepResult {
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
interface CreateWorkflowInput {
    name: string;
    description?: string;
    steps: Omit<WorkflowStep, 'id'>[];
    triggers?: WorkflowTrigger[];
    metadata?: Record<string, unknown>;
}
/**
 * Input for executing a workflow
 */
interface ExecuteWorkflowInput {
    workflowId: string;
    input?: Record<string, unknown>;
    async?: boolean;
}
/**
 * Streaming event types
 */
type StreamEventType = 'message_start' | 'content_delta' | 'content_stop' | 'message_stop' | 'error' | 'tool_use' | 'tool_result';
/**
 * Base streaming event
 */
interface StreamEvent<T = unknown> {
    type: StreamEventType;
    data: T;
    timestamp: Date;
}
/**
 * Content delta in streaming
 */
interface ContentDelta {
    text: string;
    index: number;
}
/**
 * Streaming options
 */
interface StreamOptions {
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
/**
 * Supported document types
 */
type DocumentType = 'text' | 'markdown' | 'pdf' | 'html' | 'json' | 'code';
/**
 * Document for ingestion
 */
interface DocumentInput {
    content: string | Buffer;
    filename?: string;
    contentType?: string;
    metadata?: Record<string, unknown>;
}
/**
 * Ingested document
 */
interface IngestedDocument extends BaseEntity {
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
interface DocumentChunk {
    id: string;
    documentId: string;
    content: string;
    index: number;
    tokenCount: number;
    metadata?: Record<string, unknown>;
}
/**
 * Webhook event types
 */
type WebhookEventType = 'conversation.created' | 'conversation.updated' | 'message.created' | 'workflow.started' | 'workflow.completed' | 'workflow.failed' | 'document.ingested';
/**
 * Webhook configuration
 */
interface WebhookConfig {
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
interface WebhookPayload<T = unknown> {
    id: string;
    type: WebhookEventType;
    timestamp: Date;
    data: T;
}
/**
 * Make all properties of T optional recursively
 */
type DeepPartial<T> = {
    [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};
/**
 * Extract the resolved type from a Promise
 */
type Awaited<T> = T extends Promise<infer U> ? U : T;
/**
 * Callback type for event handlers
 */
type EventHandler<T = void> = (data: T) => void | Promise<void>;

/**
 * HTTP Client for LLM-CoPilot SDK
 */

/**
 * HTTP method types
 */
type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
/**
 * Request options
 */
interface RequestOptions<T = unknown> {
    method?: HttpMethod;
    body?: T;
    query?: Record<string, string | number | boolean | undefined>;
    headers?: Record<string, string>;
    signal?: AbortSignal;
    timeout?: number;
}
/**
 * Stream response handler
 */
interface StreamHandler {
    onChunk: (chunk: string) => void;
    onError?: (error: Error) => void;
    onComplete?: () => void;
}
/**
 * SDK Error class
 */
declare class CopilotError extends Error {
    readonly code: string;
    readonly status?: number | undefined;
    readonly details?: Record<string, unknown> | undefined;
    readonly requestId?: string | undefined;
    constructor(message: string, code: string, status?: number | undefined, details?: Record<string, unknown> | undefined, requestId?: string | undefined);
    static fromApiError(error: ApiError, status?: number): CopilotError;
}
/**
 * HTTP Client for making API requests
 */
declare class HttpClient {
    private readonly config;
    constructor(config: CopilotConfig);
    /**
     * Build full URL with query parameters
     */
    private buildUrl;
    /**
     * Build request headers
     */
    private buildHeaders;
    /**
     * Log debug information
     */
    private debug;
    /**
     * Execute a request with retry logic
     */
    private executeWithRetry;
    /**
     * Make an HTTP request
     */
    request<T, B = unknown>(path: string, options?: RequestOptions<B>): Promise<ApiResponse<T>>;
    /**
     * Parse rate limit headers from response
     */
    private parseRateLimitHeaders;
    /**
     * GET request
     */
    get<T>(path: string, query?: Record<string, string | number | boolean | undefined>, options?: Omit<RequestOptions, 'method' | 'body' | 'query'>): Promise<ApiResponse<T>>;
    /**
     * POST request
     */
    post<T, B = unknown>(path: string, body?: B, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ApiResponse<T>>;
    /**
     * PUT request
     */
    put<T, B = unknown>(path: string, body?: B, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ApiResponse<T>>;
    /**
     * PATCH request
     */
    patch<T, B = unknown>(path: string, body?: B, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ApiResponse<T>>;
    /**
     * DELETE request
     */
    delete<T>(path: string, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ApiResponse<T>>;
    /**
     * Stream request using Server-Sent Events
     */
    stream(path: string, body: unknown, handler: StreamHandler, signal?: AbortSignal): Promise<void>;
    /**
     * Paginated GET request
     */
    paginate<T>(path: string, params?: PaginationParams & Record<string, unknown>): Promise<ApiResponse<PaginatedResponse<T>>>;
    /**
     * Iterate through all pages
     */
    paginateAll<T>(path: string, params?: Omit<PaginationParams, 'page' | 'cursor'> & Record<string, unknown>): AsyncGenerator<T, void, unknown>;
}

export { type ApiResponse as A, type BaseEntity as B, type CopilotConfig as C, DEFAULT_CONFIG as D, type ExecuteWorkflowInput as E, type DocumentInput as F, type DocumentChunk as G, HttpClient as H, type IngestedDocument as I, type WebhookEventType as J, type WebhookConfig as K, type WebhookPayload as L, type MessageRole as M, type DeepPartial as N, type Awaited as O, type PaginationParams as P, type EventHandler as Q, type ResponseMetadata as R, type SoftDeletableEntity as S, type WorkflowStatus as W, CopilotError as a, type ApiError as b, type RateLimitInfo as c, type PaginatedResponse as d, type Message as e, type MessageInput as f, type ConversationStatus as g, type Conversation as h, type CreateConversationInput as i, type SendMessageOptions as j, type ContextItem as k, type ContextSearchParams as l, type ContextSearchResult as m, type StepActionType as n, type WorkflowStep as o, type RetryConfig as p, type WorkflowDefinition as q, type WorkflowTrigger as r, type WorkflowExecution as s, type StepResult as t, type CreateWorkflowInput as u, type StreamEventType as v, type StreamEvent as w, type ContentDelta as x, type StreamOptions as y, type DocumentType as z };
