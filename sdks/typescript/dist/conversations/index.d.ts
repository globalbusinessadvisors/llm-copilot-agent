import { H as HttpClient, i as CreateConversationInput, h as Conversation, P as PaginationParams, d as PaginatedResponse, j as SendMessageOptions, e as Message, y as StreamOptions, f as MessageInput } from '../client-BNP-OnWr.js';

/**
 * Conversations API client
 */

/**
 * Conversations API client
 */
declare class ConversationsClient {
    private readonly client;
    constructor(client: HttpClient);
    /**
     * Create a new conversation
     */
    create(input?: CreateConversationInput): Promise<Conversation>;
    /**
     * Get a conversation by ID
     */
    get(conversationId: string): Promise<Conversation>;
    /**
     * List conversations
     */
    list(params?: PaginationParams & {
        status?: string;
    }): Promise<PaginatedResponse<Conversation>>;
    /**
     * Update a conversation
     */
    update(conversationId: string, updates: {
        title?: string;
        metadata?: Record<string, unknown>;
    }): Promise<Conversation>;
    /**
     * Delete a conversation
     */
    delete(conversationId: string): Promise<void>;
    /**
     * Archive a conversation
     */
    archive(conversationId: string): Promise<Conversation>;
    /**
     * Send a message to a conversation
     */
    sendMessage(conversationId: string, content: string, options?: SendMessageOptions): Promise<Message>;
    /**
     * Stream a message response
     */
    streamMessage(conversationId: string, content: string, options?: StreamOptions & Omit<SendMessageOptions, 'stream'>): Promise<Message>;
    /**
     * Get messages from a conversation
     */
    getMessages(conversationId: string, params?: PaginationParams): Promise<PaginatedResponse<Message>>;
    /**
     * Add a message without generating a response
     */
    addMessage(conversationId: string, message: MessageInput): Promise<Message>;
    /**
     * Fork a conversation from a specific message
     */
    fork(conversationId: string, messageId: string): Promise<Conversation>;
    /**
     * Get conversation summary
     */
    getSummary(conversationId: string): Promise<string>;
    /**
     * Export conversation
     */
    export(conversationId: string, format?: 'json' | 'markdown'): Promise<string>;
    /**
     * Parse conversation response to ensure proper types
     */
    private parseConversation;
    /**
     * Parse message response to ensure proper types
     */
    private parseMessage;
}

export { ConversationsClient, ConversationsClient as default };
