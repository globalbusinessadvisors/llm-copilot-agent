import { describe, it, expect, vi, beforeEach } from 'vitest';
import { CopilotClient, CopilotError } from '../src';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('CopilotClient', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  describe('constructor', () => {
    it('should create client with config', () => {
      const client = new CopilotClient({
        baseUrl: 'https://api.example.com',
        apiKey: 'test-key',
      });

      expect(client).toBeInstanceOf(CopilotClient);
      expect(client.conversations).toBeDefined();
      expect(client.workflows).toBeDefined();
      expect(client.context).toBeDefined();
    });
  });

  describe('fromEnv', () => {
    it('should create client from environment variables', () => {
      const originalEnv = process.env;
      process.env = {
        ...originalEnv,
        COPILOT_API_URL: 'https://api.example.com',
        COPILOT_API_KEY: 'env-api-key',
        COPILOT_TENANT_ID: 'env-tenant',
      };

      const client = CopilotClient.fromEnv();

      expect(client).toBeInstanceOf(CopilotClient);

      process.env = originalEnv;
    });

    it('should throw if API URL is missing', () => {
      const originalEnv = process.env;
      process.env = {
        ...originalEnv,
        COPILOT_API_URL: undefined,
        COPILOT_BASE_URL: undefined,
        COPILOT_API_KEY: 'test-key',
      };

      expect(() => CopilotClient.fromEnv()).toThrow(CopilotError);

      process.env = originalEnv;
    });

    it('should throw if API key is missing', () => {
      const originalEnv = process.env;
      process.env = {
        ...originalEnv,
        COPILOT_API_URL: 'https://api.example.com',
        COPILOT_API_KEY: undefined,
      };

      expect(() => CopilotClient.fromEnv()).toThrow(CopilotError);

      process.env = originalEnv;
    });

    it('should allow overrides', () => {
      const originalEnv = process.env;
      process.env = {
        ...originalEnv,
        COPILOT_API_URL: 'https://api.example.com',
        COPILOT_API_KEY: 'env-key',
      };

      const client = CopilotClient.fromEnv({
        apiKey: 'override-key',
      });

      expect(client).toBeInstanceOf(CopilotClient);

      process.env = originalEnv;
    });
  });

  describe('version', () => {
    it('should return SDK version', () => {
      expect(CopilotClient.version).toBe('0.1.0');
    });
  });

  describe('healthCheck', () => {
    it('should return health status', async () => {
      const client = new CopilotClient({
        baseUrl: 'https://api.example.com',
        apiKey: 'test-key',
      });

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            status: 'healthy',
            version: '1.0.0',
            services: { api: true, db: true },
          }),
        headers: new Headers(),
      });

      const health = await client.healthCheck();

      expect(health.status).toBe('healthy');
      expect(health.version).toBe('1.0.0');
      expect(health.services.api).toBe(true);
    });
  });

  describe('getApiInfo', () => {
    it('should return API info', async () => {
      const client = new CopilotClient({
        baseUrl: 'https://api.example.com',
        apiKey: 'test-key',
      });

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            name: 'LLM-CoPilot API',
            version: '1.0.0',
            description: 'Enterprise LLM Agent API',
            documentation: 'https://docs.example.com',
          }),
        headers: new Headers(),
      });

      const info = await client.getApiInfo();

      expect(info.name).toBe('LLM-CoPilot API');
      expect(info.version).toBe('1.0.0');
    });
  });
});

describe('ConversationsClient', () => {
  let client: CopilotClient;

  beforeEach(() => {
    mockFetch.mockReset();
    client = new CopilotClient({
      baseUrl: 'https://api.example.com',
      apiKey: 'test-key',
    });
  });

  describe('create', () => {
    it('should create a conversation', async () => {
      const conversationData = {
        id: 'conv-123',
        title: 'Test Conversation',
        status: 'active',
        messages: [],
        tokenCount: 0,
        messageCount: 0,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(conversationData),
        headers: new Headers(),
      });

      const conversation = await client.conversations.create({
        title: 'Test Conversation',
      });

      expect(conversation.id).toBe('conv-123');
      expect(conversation.title).toBe('Test Conversation');
    });
  });

  describe('sendMessage', () => {
    it('should send a message and get response', async () => {
      const messageData = {
        id: 'msg-123',
        role: 'assistant',
        content: 'Hello! How can I help you?',
        createdAt: new Date().toISOString(),
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(messageData),
        headers: new Headers(),
      });

      const message = await client.conversations.sendMessage(
        'conv-123',
        'Hello!'
      );

      expect(message.id).toBe('msg-123');
      expect(message.role).toBe('assistant');
      expect(message.content).toContain('Hello');
    });
  });
});

describe('WorkflowsClient', () => {
  let client: CopilotClient;

  beforeEach(() => {
    mockFetch.mockReset();
    client = new CopilotClient({
      baseUrl: 'https://api.example.com',
      apiKey: 'test-key',
    });
  });

  describe('execute', () => {
    it('should execute a workflow', async () => {
      const executionData = {
        id: 'exec-123',
        workflowId: 'wf-123',
        status: 'running',
        stepResults: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(executionData),
        headers: new Headers(),
      });

      const execution = await client.workflows.execute({
        workflowId: 'wf-123',
        input: { data: 'test' },
      });

      expect(execution.id).toBe('exec-123');
      expect(execution.status).toBe('running');
    });
  });
});

describe('ContextClient', () => {
  let client: CopilotClient;

  beforeEach(() => {
    mockFetch.mockReset();
    client = new CopilotClient({
      baseUrl: 'https://api.example.com',
      apiKey: 'test-key',
    });
  });

  describe('search', () => {
    it('should search context', async () => {
      const searchResult = {
        items: [
          {
            id: 'ctx-1',
            type: 'document',
            content: 'Relevant content',
            relevanceScore: 0.95,
          },
        ],
        totalMatches: 1,
        searchTimeMs: 50,
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(searchResult),
        headers: new Headers(),
      });

      const result = await client.context.search({
        query: 'test query',
        limit: 10,
      });

      expect(result.items).toHaveLength(1);
      expect(result.items[0]?.relevanceScore).toBe(0.95);
    });
  });

  describe('ingestDocument', () => {
    it('should ingest a document', async () => {
      const documentData = {
        id: 'doc-123',
        filename: 'test.txt',
        contentType: 'text/plain',
        size: 1024,
        chunkCount: 5,
        status: 'completed',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(documentData),
        headers: new Headers(),
      });

      const document = await client.context.ingestDocument({
        content: 'Test content',
        filename: 'test.txt',
      });

      expect(document.id).toBe('doc-123');
      expect(document.status).toBe('completed');
    });
  });
});
