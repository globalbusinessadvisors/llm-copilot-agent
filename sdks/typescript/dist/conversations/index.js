'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

// src/client.ts
var CopilotError = class _CopilotError extends Error {
  constructor(message, code, status, details, requestId) {
    super(message);
    this.code = code;
    this.status = status;
    this.details = details;
    this.requestId = requestId;
    this.name = "CopilotError";
  }
  static fromApiError(error, status) {
    return new _CopilotError(
      error.message,
      error.code,
      status,
      error.details,
      error.requestId
    );
  }
};

// src/conversations/index.ts
var ConversationsClient = class {
  constructor(client) {
    this.client = client;
  }
  /**
   * Create a new conversation
   */
  async create(input = {}) {
    const response = await this.client.post(
      "/api/v1/conversations",
      input
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to create conversation", "CREATE_FAILED");
    }
    return this.parseConversation(response.data);
  }
  /**
   * Get a conversation by ID
   */
  async get(conversationId) {
    const response = await this.client.get(
      `/api/v1/conversations/${conversationId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Conversation not found", "NOT_FOUND", 404);
    }
    return this.parseConversation(response.data);
  }
  /**
   * List conversations
   */
  async list(params) {
    const response = await this.client.paginate(
      "/api/v1/conversations",
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list conversations", "LIST_FAILED");
    }
    return {
      ...response.data,
      items: response.data.items.map((c) => this.parseConversation(c))
    };
  }
  /**
   * Update a conversation
   */
  async update(conversationId, updates) {
    const response = await this.client.patch(
      `/api/v1/conversations/${conversationId}`,
      updates
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to update conversation", "UPDATE_FAILED");
    }
    return this.parseConversation(response.data);
  }
  /**
   * Delete a conversation
   */
  async delete(conversationId) {
    const response = await this.client.delete(
      `/api/v1/conversations/${conversationId}`
    );
    if (!response.success) {
      throw new CopilotError("Failed to delete conversation", "DELETE_FAILED");
    }
  }
  /**
   * Archive a conversation
   */
  async archive(conversationId) {
    const response = await this.client.post(
      `/api/v1/conversations/${conversationId}/archive`
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to archive conversation",
        "ARCHIVE_FAILED"
      );
    }
    return this.parseConversation(response.data);
  }
  /**
   * Send a message to a conversation
   */
  async sendMessage(conversationId, content, options = {}) {
    const { stream, ...rest } = options;
    if (stream) {
      throw new CopilotError(
        "Use streamMessage() for streaming responses",
        "USE_STREAM_METHOD"
      );
    }
    const response = await this.client.post(
      `/api/v1/conversations/${conversationId}/messages`,
      { content, ...rest }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to send message", "SEND_FAILED");
    }
    return this.parseMessage(response.data);
  }
  /**
   * Stream a message response
   */
  async streamMessage(conversationId, content, options = {}) {
    const { onChunk, onEvent, onError, onComplete, signal, ...rest } = options;
    let fullContent = "";
    let messageId = "";
    return new Promise((resolve, reject) => {
      this.client.stream(
        `/api/v1/conversations/${conversationId}/messages/stream`,
        { content, ...rest },
        {
          onChunk: (data) => {
            try {
              const event = JSON.parse(data);
              switch (event.type) {
                case "message_start":
                  messageId = event.data.id;
                  break;
                case "content_delta":
                  const delta = event.data;
                  fullContent += delta.text;
                  onChunk?.(delta.text);
                  break;
                case "message_stop":
                  break;
              }
              onEvent?.(event);
            } catch (error) {
              fullContent += data;
              onChunk?.(data);
            }
          },
          onError: (error) => {
            onError?.(error);
            reject(error);
          },
          onComplete: () => {
            const message = {
              id: messageId || "streamed",
              role: "assistant",
              content: fullContent,
              createdAt: /* @__PURE__ */ new Date()
            };
            onComplete?.(message);
            resolve(message);
          }
        },
        signal
      ).catch(reject);
    });
  }
  /**
   * Get messages from a conversation
   */
  async getMessages(conversationId, params) {
    const response = await this.client.paginate(
      `/api/v1/conversations/${conversationId}/messages`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to get messages", "GET_MESSAGES_FAILED");
    }
    return {
      ...response.data,
      items: response.data.items.map((m) => this.parseMessage(m))
    };
  }
  /**
   * Add a message without generating a response
   */
  async addMessage(conversationId, message) {
    const response = await this.client.post(
      `/api/v1/conversations/${conversationId}/messages/add`,
      message
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to add message", "ADD_MESSAGE_FAILED");
    }
    return this.parseMessage(response.data);
  }
  /**
   * Fork a conversation from a specific message
   */
  async fork(conversationId, messageId) {
    const response = await this.client.post(
      `/api/v1/conversations/${conversationId}/fork`,
      { messageId }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to fork conversation", "FORK_FAILED");
    }
    return this.parseConversation(response.data);
  }
  /**
   * Get conversation summary
   */
  async getSummary(conversationId) {
    const response = await this.client.get(
      `/api/v1/conversations/${conversationId}/summary`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to get summary", "SUMMARY_FAILED");
    }
    return response.data.summary;
  }
  /**
   * Export conversation
   */
  async export(conversationId, format = "json") {
    const response = await this.client.get(
      `/api/v1/conversations/${conversationId}/export`,
      { format }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to export conversation", "EXPORT_FAILED");
    }
    return response.data.content;
  }
  /**
   * Parse conversation response to ensure proper types
   */
  parseConversation(data) {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
      messages: data.messages?.map((m) => this.parseMessage(m)) ?? []
    };
  }
  /**
   * Parse message response to ensure proper types
   */
  parseMessage(data) {
    return {
      ...data,
      createdAt: new Date(data.createdAt)
    };
  }
};
var conversations_default = ConversationsClient;

exports.ConversationsClient = ConversationsClient;
exports.default = conversations_default;
//# sourceMappingURL=index.js.map
//# sourceMappingURL=index.js.map