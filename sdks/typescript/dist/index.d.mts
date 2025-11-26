import ConversationsClient from './conversations/index.mjs';
import WorkflowsClient from './workflows/index.mjs';
import ContextClient from './context/index.mjs';
import { C as CopilotConfig } from './client-BNP-OnWr.mjs';
export { b as ApiError, A as ApiResponse, O as Awaited, B as BaseEntity, x as ContentDelta, k as ContextItem, l as ContextSearchParams, m as ContextSearchResult, h as Conversation, g as ConversationStatus, a as CopilotError, i as CreateConversationInput, u as CreateWorkflowInput, D as DEFAULT_CONFIG, N as DeepPartial, G as DocumentChunk, F as DocumentInput, z as DocumentType, Q as EventHandler, E as ExecuteWorkflowInput, H as HttpClient, I as IngestedDocument, e as Message, f as MessageInput, M as MessageRole, d as PaginatedResponse, P as PaginationParams, c as RateLimitInfo, R as ResponseMetadata, p as RetryConfig, j as SendMessageOptions, S as SoftDeletableEntity, n as StepActionType, t as StepResult, w as StreamEvent, v as StreamEventType, y as StreamOptions, K as WebhookConfig, J as WebhookEventType, L as WebhookPayload, q as WorkflowDefinition, s as WorkflowExecution, W as WorkflowStatus, o as WorkflowStep, r as WorkflowTrigger } from './client-BNP-OnWr.mjs';

/**
 * LLM-CoPilot TypeScript SDK
 *
 * A comprehensive SDK for interacting with the LLM-CoPilot-Agent API.
 *
 * @example
 * ```typescript
 * import { CopilotClient } from '@llm-copilot/sdk';
 *
 * const client = new CopilotClient({
 *   baseUrl: 'https://api.copilot.example.com',
 *   apiKey: 'your-api-key',
 * });
 *
 * // Create a conversation and send messages
 * const conversation = await client.conversations.create();
 * const response = await client.conversations.sendMessage(
 *   conversation.id,
 *   'Hello, world!'
 * );
 *
 * // Stream responses
 * await client.conversations.streamMessage(
 *   conversation.id,
 *   'Tell me a story',
 *   {
 *     onChunk: (chunk) => process.stdout.write(chunk),
 *   }
 * );
 *
 * // Search context
 * const results = await client.context.search({
 *   query: 'important topic',
 *   limit: 10,
 * });
 *
 * // Execute workflows
 * const execution = await client.workflows.execute({
 *   workflowId: 'my-workflow',
 *   input: { data: 'value' },
 * });
 * ```
 */

/**
 * Main SDK client
 */
declare class CopilotClient {
    private readonly httpClient;
    /** Conversations API */
    readonly conversations: ConversationsClient;
    /** Workflows API */
    readonly workflows: WorkflowsClient;
    /** Context API */
    readonly context: ContextClient;
    constructor(config: CopilotConfig);
    /**
     * Create a client from environment variables
     */
    static fromEnv(overrides?: Partial<CopilotConfig>): CopilotClient;
    /**
     * Get SDK version
     */
    static get version(): string;
    /**
     * Health check
     */
    healthCheck(): Promise<{
        status: 'healthy' | 'degraded' | 'unhealthy';
        version: string;
        services: Record<string, boolean>;
    }>;
    /**
     * Get API info
     */
    getApiInfo(): Promise<{
        name: string;
        version: string;
        description: string;
        documentation: string;
    }>;
}

export { ContextClient, ConversationsClient, CopilotClient, CopilotConfig, WorkflowsClient, CopilotClient as default };
