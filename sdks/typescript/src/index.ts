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

import { HttpClient, CopilotError } from './client';
import { ConversationsClient } from './conversations';
import { WorkflowsClient } from './workflows';
import { ContextClient } from './context';
import type { CopilotConfig } from './types';

// Re-export types
export * from './types';
export { CopilotError } from './client';

/**
 * Main SDK client
 */
export class CopilotClient {
  private readonly httpClient: HttpClient;

  /** Conversations API */
  public readonly conversations: ConversationsClient;

  /** Workflows API */
  public readonly workflows: WorkflowsClient;

  /** Context API */
  public readonly context: ContextClient;

  constructor(config: CopilotConfig) {
    this.httpClient = new HttpClient(config);
    this.conversations = new ConversationsClient(this.httpClient);
    this.workflows = new WorkflowsClient(this.httpClient);
    this.context = new ContextClient(this.httpClient);
  }

  /**
   * Create a client from environment variables
   */
  static fromEnv(overrides?: Partial<CopilotConfig>): CopilotClient {
    const baseUrl =
      overrides?.baseUrl ??
      process.env['COPILOT_API_URL'] ??
      process.env['COPILOT_BASE_URL'];

    const apiKey =
      overrides?.apiKey ??
      process.env['COPILOT_API_KEY'];

    const tenantId =
      overrides?.tenantId ??
      process.env['COPILOT_TENANT_ID'];

    if (!baseUrl) {
      throw new CopilotError(
        'Missing API URL. Set COPILOT_API_URL environment variable or pass baseUrl in config.',
        'CONFIG_ERROR'
      );
    }

    if (!apiKey) {
      throw new CopilotError(
        'Missing API key. Set COPILOT_API_KEY environment variable or pass apiKey in config.',
        'CONFIG_ERROR'
      );
    }

    return new CopilotClient({
      baseUrl,
      apiKey,
      tenantId,
      ...overrides,
    });
  }

  /**
   * Get SDK version
   */
  static get version(): string {
    return '0.1.0';
  }

  /**
   * Health check
   */
  async healthCheck(): Promise<{
    status: 'healthy' | 'degraded' | 'unhealthy';
    version: string;
    services: Record<string, boolean>;
  }> {
    const response = await this.httpClient.get<{
      status: 'healthy' | 'degraded' | 'unhealthy';
      version: string;
      services: Record<string, boolean>;
    }>('/health');

    if (!response.success || !response.data) {
      throw new CopilotError('Health check failed', 'HEALTH_CHECK_FAILED');
    }

    return response.data;
  }

  /**
   * Get API info
   */
  async getApiInfo(): Promise<{
    name: string;
    version: string;
    description: string;
    documentation: string;
  }> {
    const response = await this.httpClient.get<{
      name: string;
      version: string;
      description: string;
      documentation: string;
    }>('/api/v1');

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to get API info', 'API_INFO_FAILED');
    }

    return response.data;
  }
}

// Default export
export default CopilotClient;

// Named exports for tree-shaking
export { ConversationsClient } from './conversations';
export { WorkflowsClient } from './workflows';
export { ContextClient } from './context';
export { HttpClient } from './client';
