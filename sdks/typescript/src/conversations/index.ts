/**
 * Conversations API client
 */

import { HttpClient, CopilotError } from '../client';
import type {
  Conversation,
  CreateConversationInput,
  Message,
  MessageInput,
  SendMessageOptions,
  StreamOptions,
  PaginatedResponse,
  PaginationParams,
  ContentDelta,
  StreamEvent,
} from '../types';

/**
 * Conversations API client
 */
export class ConversationsClient {
  constructor(private readonly client: HttpClient) {}

  /**
   * Create a new conversation
   */
  async create(input: CreateConversationInput = {}): Promise<Conversation> {
    const response = await this.client.post<Conversation>(
      '/api/v1/conversations',
      input
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to create conversation', 'CREATE_FAILED');
    }

    return this.parseConversation(response.data);
  }

  /**
   * Get a conversation by ID
   */
  async get(conversationId: string): Promise<Conversation> {
    const response = await this.client.get<Conversation>(
      `/api/v1/conversations/${conversationId}`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Conversation not found', 'NOT_FOUND', 404);
    }

    return this.parseConversation(response.data);
  }

  /**
   * List conversations
   */
  async list(
    params?: PaginationParams & { status?: string }
  ): Promise<PaginatedResponse<Conversation>> {
    const response = await this.client.paginate<Conversation>(
      '/api/v1/conversations',
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list conversations', 'LIST_FAILED');
    }

    return {
      ...response.data,
      items: response.data.items.map((c) => this.parseConversation(c)),
    };
  }

  /**
   * Update a conversation
   */
  async update(
    conversationId: string,
    updates: { title?: string; metadata?: Record<string, unknown> }
  ): Promise<Conversation> {
    const response = await this.client.patch<Conversation>(
      `/api/v1/conversations/${conversationId}`,
      updates
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to update conversation', 'UPDATE_FAILED');
    }

    return this.parseConversation(response.data);
  }

  /**
   * Delete a conversation
   */
  async delete(conversationId: string): Promise<void> {
    const response = await this.client.delete(
      `/api/v1/conversations/${conversationId}`
    );

    if (!response.success) {
      throw new CopilotError('Failed to delete conversation', 'DELETE_FAILED');
    }
  }

  /**
   * Archive a conversation
   */
  async archive(conversationId: string): Promise<Conversation> {
    const response = await this.client.post<Conversation>(
      `/api/v1/conversations/${conversationId}/archive`
    );

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to archive conversation',
        'ARCHIVE_FAILED'
      );
    }

    return this.parseConversation(response.data);
  }

  /**
   * Send a message to a conversation
   */
  async sendMessage(
    conversationId: string,
    content: string,
    options: SendMessageOptions = {}
  ): Promise<Message> {
    const { stream, ...rest } = options;

    if (stream) {
      throw new CopilotError(
        'Use streamMessage() for streaming responses',
        'USE_STREAM_METHOD'
      );
    }

    const response = await this.client.post<Message>(
      `/api/v1/conversations/${conversationId}/messages`,
      { content, ...rest }
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to send message', 'SEND_FAILED');
    }

    return this.parseMessage(response.data);
  }

  /**
   * Stream a message response
   */
  async streamMessage(
    conversationId: string,
    content: string,
    options: StreamOptions & Omit<SendMessageOptions, 'stream'> = {}
  ): Promise<Message> {
    const { onChunk, onEvent, onError, onComplete, signal, ...rest } = options;

    let fullContent = '';
    let messageId = '';

    return new Promise<Message>((resolve, reject) => {
      this.client
        .stream(
          `/api/v1/conversations/${conversationId}/messages/stream`,
          { content, ...rest },
          {
            onChunk: (data) => {
              try {
                const event = JSON.parse(data) as StreamEvent;

                switch (event.type) {
                  case 'message_start':
                    messageId = (event.data as { id: string }).id;
                    break;

                  case 'content_delta':
                    const delta = event.data as ContentDelta;
                    fullContent += delta.text;
                    onChunk?.(delta.text);
                    break;

                  case 'message_stop':
                    break;
                }

                onEvent?.(event);
              } catch (error) {
                // Not JSON, treat as raw text
                fullContent += data;
                onChunk?.(data);
              }
            },
            onError: (error) => {
              onError?.(error);
              reject(error);
            },
            onComplete: () => {
              const message: Message = {
                id: messageId || 'streamed',
                role: 'assistant',
                content: fullContent,
                createdAt: new Date(),
              };
              onComplete?.(message);
              resolve(message);
            },
          },
          signal
        )
        .catch(reject);
    });
  }

  /**
   * Get messages from a conversation
   */
  async getMessages(
    conversationId: string,
    params?: PaginationParams
  ): Promise<PaginatedResponse<Message>> {
    const response = await this.client.paginate<Message>(
      `/api/v1/conversations/${conversationId}/messages`,
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to get messages', 'GET_MESSAGES_FAILED');
    }

    return {
      ...response.data,
      items: response.data.items.map((m) => this.parseMessage(m)),
    };
  }

  /**
   * Add a message without generating a response
   */
  async addMessage(
    conversationId: string,
    message: MessageInput
  ): Promise<Message> {
    const response = await this.client.post<Message>(
      `/api/v1/conversations/${conversationId}/messages/add`,
      message
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to add message', 'ADD_MESSAGE_FAILED');
    }

    return this.parseMessage(response.data);
  }

  /**
   * Fork a conversation from a specific message
   */
  async fork(conversationId: string, messageId: string): Promise<Conversation> {
    const response = await this.client.post<Conversation>(
      `/api/v1/conversations/${conversationId}/fork`,
      { messageId }
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to fork conversation', 'FORK_FAILED');
    }

    return this.parseConversation(response.data);
  }

  /**
   * Get conversation summary
   */
  async getSummary(conversationId: string): Promise<string> {
    const response = await this.client.get<{ summary: string }>(
      `/api/v1/conversations/${conversationId}/summary`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to get summary', 'SUMMARY_FAILED');
    }

    return response.data.summary;
  }

  /**
   * Export conversation
   */
  async export(
    conversationId: string,
    format: 'json' | 'markdown' = 'json'
  ): Promise<string> {
    const response = await this.client.get<{ content: string }>(
      `/api/v1/conversations/${conversationId}/export`,
      { format }
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to export conversation', 'EXPORT_FAILED');
    }

    return response.data.content;
  }

  /**
   * Parse conversation response to ensure proper types
   */
  private parseConversation(data: Conversation): Conversation {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
      messages: data.messages?.map((m) => this.parseMessage(m)) ?? [],
    };
  }

  /**
   * Parse message response to ensure proper types
   */
  private parseMessage(data: Message): Message {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
    };
  }
}

export default ConversationsClient;
