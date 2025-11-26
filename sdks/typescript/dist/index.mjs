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
var HttpClient = class {
  config;
  constructor(config) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/+$/, ""),
      apiKey: config.apiKey,
      timeout: config.timeout ?? 3e4,
      maxRetries: config.maxRetries ?? 3,
      tenantId: config.tenantId,
      headers: config.headers,
      debug: config.debug
    };
  }
  /**
   * Build full URL with query parameters
   */
  buildUrl(path, query) {
    const url = new URL(path, this.config.baseUrl);
    if (query) {
      Object.entries(query).forEach(([key, value]) => {
        if (value !== void 0) {
          url.searchParams.append(key, String(value));
        }
      });
    }
    return url.toString();
  }
  /**
   * Build request headers
   */
  buildHeaders(customHeaders) {
    const headers = new Headers({
      "Content-Type": "application/json",
      Authorization: `Bearer ${this.config.apiKey}`,
      "User-Agent": "@llm-copilot/sdk",
      ...this.config.headers,
      ...customHeaders
    });
    if (this.config.tenantId) {
      headers.set("X-Tenant-ID", this.config.tenantId);
    }
    return headers;
  }
  /**
   * Log debug information
   */
  debug(message, data) {
    if (this.config.debug) {
      console.log(`[CopilotSDK] ${message}`, data ?? "");
    }
  }
  /**
   * Execute a request with retry logic
   */
  async executeWithRetry(fn, retries = this.config.maxRetries) {
    let lastError;
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        return await fn();
      } catch (error) {
        lastError = error;
        if (error instanceof CopilotError) {
          if (error.status && error.status >= 400 && error.status < 500 && error.status !== 429) {
            throw error;
          }
        }
        if (attempt < retries) {
          const delay = Math.min(1e3 * Math.pow(2, attempt), 1e4);
          this.debug(`Retry attempt ${attempt + 1} after ${delay}ms`);
          await new Promise((resolve) => setTimeout(resolve, delay));
        }
      }
    }
    throw lastError;
  }
  /**
   * Make an HTTP request
   */
  async request(path, options = {}) {
    const { method = "GET", body, query, headers, signal, timeout } = options;
    const url = this.buildUrl(path, query);
    const requestHeaders = this.buildHeaders(headers);
    this.debug(`${method} ${url}`);
    const controller = new AbortController();
    const timeoutId = setTimeout(
      () => controller.abort(),
      timeout ?? this.config.timeout
    );
    try {
      const response = await this.executeWithRetry(async () => {
        const res = await fetch(url, {
          method,
          headers: requestHeaders,
          body: body ? JSON.stringify(body) : void 0,
          signal: signal ?? controller.signal
        });
        if (!res.ok) {
          const errorBody = await res.json().catch(() => ({}));
          const apiError = {
            code: errorBody.code ?? "UNKNOWN_ERROR",
            message: errorBody.message ?? res.statusText,
            details: errorBody.details,
            requestId: res.headers.get("X-Request-ID") ?? void 0
          };
          throw CopilotError.fromApiError(apiError, res.status);
        }
        return res;
      });
      const data = await response.json();
      return {
        success: true,
        data,
        metadata: {
          requestId: response.headers.get("X-Request-ID") ?? "",
          processingTimeMs: parseInt(
            response.headers.get("X-Processing-Time") ?? "0",
            10
          ),
          rateLimit: this.parseRateLimitHeaders(response.headers)
        }
      };
    } catch (error) {
      if (error instanceof CopilotError) {
        throw error;
      }
      if (error instanceof Error && error.name === "AbortError") {
        throw new CopilotError("Request timeout", "TIMEOUT", 408);
      }
      throw new CopilotError(
        error instanceof Error ? error.message : "Unknown error",
        "NETWORK_ERROR"
      );
    } finally {
      clearTimeout(timeoutId);
    }
  }
  /**
   * Parse rate limit headers from response
   */
  parseRateLimitHeaders(headers) {
    const limit = headers.get("X-RateLimit-Limit");
    const remaining = headers.get("X-RateLimit-Remaining");
    const reset = headers.get("X-RateLimit-Reset");
    if (limit && remaining && reset) {
      return {
        limit: parseInt(limit, 10),
        remaining: parseInt(remaining, 10),
        resetAt: new Date(parseInt(reset, 10) * 1e3)
      };
    }
    return void 0;
  }
  /**
   * GET request
   */
  async get(path, query, options) {
    return this.request(path, { ...options, method: "GET", query });
  }
  /**
   * POST request
   */
  async post(path, body, options) {
    return this.request(path, { ...options, method: "POST", body });
  }
  /**
   * PUT request
   */
  async put(path, body, options) {
    return this.request(path, { ...options, method: "PUT", body });
  }
  /**
   * PATCH request
   */
  async patch(path, body, options) {
    return this.request(path, { ...options, method: "PATCH", body });
  }
  /**
   * DELETE request
   */
  async delete(path, options) {
    return this.request(path, { ...options, method: "DELETE" });
  }
  /**
   * Stream request using Server-Sent Events
   */
  async stream(path, body, handler, signal) {
    const url = this.buildUrl(path);
    const headers = this.buildHeaders({ Accept: "text/event-stream" });
    this.debug(`STREAM ${url}`);
    try {
      const response = await fetch(url, {
        method: "POST",
        headers,
        body: JSON.stringify(body),
        signal
      });
      if (!response.ok) {
        const errorBody = await response.json().catch(() => ({}));
        throw CopilotError.fromApiError(
          {
            code: errorBody.code ?? "STREAM_ERROR",
            message: errorBody.message ?? response.statusText
          },
          response.status
        );
      }
      if (!response.body) {
        throw new CopilotError("No response body", "NO_BODY", 500);
      }
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) {
            handler.onComplete?.();
            break;
          }
          const chunk = decoder.decode(value, { stream: true });
          const lines = chunk.split("\n");
          for (const line of lines) {
            if (line.startsWith("data: ")) {
              const data = line.slice(6);
              if (data === "[DONE]") {
                handler.onComplete?.();
                return;
              }
              handler.onChunk(data);
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      if (error instanceof CopilotError) {
        handler.onError?.(error);
        throw error;
      }
      const copilotError = new CopilotError(
        error instanceof Error ? error.message : "Stream error",
        "STREAM_ERROR"
      );
      handler.onError?.(copilotError);
      throw copilotError;
    }
  }
  /**
   * Paginated GET request
   */
  async paginate(path, params) {
    const { page = 1, pageSize = 20, cursor, ...rest } = params ?? {};
    return this.get(path, {
      page,
      page_size: pageSize,
      cursor,
      ...rest
    });
  }
  /**
   * Iterate through all pages
   */
  async *paginateAll(path, params) {
    let cursor;
    let hasMore = true;
    while (hasMore) {
      const response = await this.paginate(path, { ...params, cursor });
      if (!response.success || !response.data) {
        throw new CopilotError("Pagination failed", "PAGINATION_ERROR");
      }
      for (const item of response.data.items) {
        yield item;
      }
      hasMore = response.data.hasMore;
      cursor = response.data.nextCursor;
    }
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

// src/workflows/index.ts
var WorkflowsClient = class {
  constructor(client) {
    this.client = client;
  }
  // ============================================================================
  // Workflow Definitions
  // ============================================================================
  /**
   * Create a new workflow
   */
  async create(input) {
    const response = await this.client.post(
      "/api/v1/workflows",
      input
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to create workflow", "CREATE_FAILED");
    }
    return response.data;
  }
  /**
   * Get a workflow by ID
   */
  async get(workflowId) {
    const response = await this.client.get(
      `/api/v1/workflows/${workflowId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Workflow not found", "NOT_FOUND", 404);
    }
    return response.data;
  }
  /**
   * List workflows
   */
  async list(params) {
    const response = await this.client.paginate(
      "/api/v1/workflows",
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list workflows", "LIST_FAILED");
    }
    return response.data;
  }
  /**
   * Update a workflow
   */
  async update(workflowId, updates) {
    const response = await this.client.patch(
      `/api/v1/workflows/${workflowId}`,
      updates
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to update workflow", "UPDATE_FAILED");
    }
    return response.data;
  }
  /**
   * Delete a workflow
   */
  async delete(workflowId) {
    const response = await this.client.delete(
      `/api/v1/workflows/${workflowId}`
    );
    if (!response.success) {
      throw new CopilotError("Failed to delete workflow", "DELETE_FAILED");
    }
  }
  /**
   * Create a new version of a workflow
   */
  async createVersion(workflowId, versionType = "minor") {
    const response = await this.client.post(
      `/api/v1/workflows/${workflowId}/versions`,
      { versionType }
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to create workflow version",
        "VERSION_FAILED"
      );
    }
    return response.data;
  }
  /**
   * List workflow versions
   */
  async listVersions(workflowId) {
    const response = await this.client.paginate(
      `/api/v1/workflows/${workflowId}/versions`
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to list workflow versions",
        "LIST_VERSIONS_FAILED"
      );
    }
    return response.data;
  }
  // ============================================================================
  // Workflow Execution
  // ============================================================================
  /**
   * Execute a workflow
   */
  async execute(input) {
    const { workflowId, ...rest } = input;
    const response = await this.client.post(
      `/api/v1/workflows/${workflowId}/execute`,
      rest
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to execute workflow", "EXECUTE_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Execute a workflow and stream progress
   */
  async executeWithStream(input, options = {}) {
    const { workflowId, ...rest } = input;
    const { onChunk, onEvent, onError, onComplete, onStep, signal } = options;
    let execution = {};
    return new Promise((resolve, reject) => {
      this.client.stream(
        `/api/v1/workflows/${workflowId}/execute/stream`,
        { ...rest, stream: true },
        {
          onChunk: (data) => {
            try {
              const event = JSON.parse(data);
              if (event.type === "message_start") {
                execution = event.data;
              }
              const workflowEvent = event.data;
              switch (workflowEvent.type) {
                case "step_started":
                  onChunk?.(`Step started: ${workflowEvent.stepId}`);
                  break;
                case "step_completed":
                  if (workflowEvent.stepId && workflowEvent.result) {
                    onStep?.(workflowEvent.stepId, workflowEvent.result);
                  }
                  onChunk?.(`Step completed: ${workflowEvent.stepId}`);
                  break;
                case "step_failed":
                  onChunk?.(
                    `Step failed: ${workflowEvent.stepId} - ${workflowEvent.error}`
                  );
                  break;
                case "workflow_completed":
                  break;
              }
              onEvent?.(event);
            } catch {
              onChunk?.(data);
            }
          },
          onError: (error) => {
            onError?.(error);
            reject(error);
          },
          onComplete: () => {
            const finalExecution = this.parseExecution(
              execution
            );
            onComplete?.(finalExecution);
            resolve(finalExecution);
          }
        },
        signal
      ).catch(reject);
    });
  }
  /**
   * Get execution status
   */
  async getExecution(executionId) {
    const response = await this.client.get(
      `/api/v1/executions/${executionId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Execution not found", "NOT_FOUND", 404);
    }
    return this.parseExecution(response.data);
  }
  /**
   * List executions for a workflow
   */
  async listExecutions(workflowId, params) {
    const response = await this.client.paginate(
      `/api/v1/workflows/${workflowId}/executions`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list executions", "LIST_FAILED");
    }
    return {
      ...response.data,
      items: response.data.items.map((e) => this.parseExecution(e))
    };
  }
  /**
   * Cancel a running execution
   */
  async cancelExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/cancel`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to cancel execution", "CANCEL_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Retry a failed execution
   */
  async retryExecution(executionId, fromStep) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/retry`,
      { fromStep }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to retry execution", "RETRY_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Pause a running execution
   */
  async pauseExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/pause`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to pause execution", "PAUSE_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Resume a paused execution
   */
  async resumeExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/resume`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to resume execution", "RESUME_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Poll execution until completion
   */
  async waitForCompletion(executionId, options = {}) {
    const { pollInterval = 1e3, timeout = 3e5, onProgress } = options;
    const startTime = Date.now();
    while (true) {
      const execution = await this.getExecution(executionId);
      onProgress?.(execution);
      if (execution.status === "completed" || execution.status === "failed" || execution.status === "cancelled") {
        return execution;
      }
      if (Date.now() - startTime > timeout) {
        throw new CopilotError(
          "Workflow execution timeout",
          "EXECUTION_TIMEOUT"
        );
      }
      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }
  }
  // ============================================================================
  // Templates
  // ============================================================================
  /**
   * Create workflow from template
   */
  async createFromTemplate(templateId, params) {
    const response = await this.client.post(
      `/api/v1/workflows/templates/${templateId}/instantiate`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to create workflow from template",
        "TEMPLATE_FAILED"
      );
    }
    return response.data;
  }
  /**
   * List available templates
   */
  async listTemplates() {
    const response = await this.client.paginate("/api/v1/workflows/templates");
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list templates", "LIST_TEMPLATES_FAILED");
    }
    return response.data;
  }
  /**
   * Parse execution response to ensure proper types
   */
  parseExecution(data) {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
      startedAt: data.startedAt ? new Date(data.startedAt) : void 0,
      completedAt: data.completedAt ? new Date(data.completedAt) : void 0
    };
  }
};

// src/context/index.ts
var ContextClient = class {
  constructor(client) {
    this.client = client;
  }
  // ============================================================================
  // Context Search
  // ============================================================================
  /**
   * Search context
   */
  async search(options) {
    const response = await this.client.post(
      "/api/v1/context/search",
      options
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Search failed", "SEARCH_FAILED");
    }
    return response.data;
  }
  /**
   * Vector similarity search
   */
  async vectorSearch(query, options = {}) {
    const result = await this.search({
      query,
      ...options,
      hybrid: false
    });
    return result.items;
  }
  /**
   * Hybrid search (vector + keyword)
   */
  async hybridSearch(query, options = {}) {
    const result = await this.search({
      query,
      limit: options.limit,
      threshold: options.threshold,
      hybrid: true,
      filters: {
        vectorWeight: options.vectorWeight ?? 0.7,
        keywordWeight: options.keywordWeight ?? 0.3
      }
    });
    return result.items;
  }
  /**
   * Search with reranking
   */
  async searchWithRerank(query, options = {}) {
    const result = await this.search({
      query,
      limit: options.limit ?? 10,
      filters: {
        initialLimit: options.initialLimit ?? 50
      },
      rerank: true
    });
    return result.items;
  }
  // ============================================================================
  // Memory Management
  // ============================================================================
  /**
   * Store a memory item
   */
  async storeMemory(content, options = {}) {
    const response = await this.client.post("/api/v1/context/memory", {
      content,
      ...options
    });
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to store memory", "STORE_MEMORY_FAILED");
    }
    return response.data;
  }
  /**
   * Get memory by ID
   */
  async getMemory(memoryId) {
    const response = await this.client.get(
      `/api/v1/context/memory/${memoryId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Memory not found", "NOT_FOUND", 404);
    }
    return response.data;
  }
  /**
   * List memories
   */
  async listMemories(params) {
    const response = await this.client.paginate(
      "/api/v1/context/memory",
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list memories", "LIST_MEMORIES_FAILED");
    }
    return response.data;
  }
  /**
   * Delete a memory
   */
  async deleteMemory(memoryId) {
    const response = await this.client.delete(`/api/v1/context/memory/${memoryId}`);
    if (!response.success) {
      throw new CopilotError("Failed to delete memory", "DELETE_MEMORY_FAILED");
    }
  }
  /**
   * Consolidate memories
   */
  async consolidateMemories() {
    const response = await this.client.post("/api/v1/context/memory/consolidate");
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to consolidate memories",
        "CONSOLIDATE_FAILED"
      );
    }
    return response.data;
  }
  // ============================================================================
  // Document Ingestion
  // ============================================================================
  /**
   * Ingest a document
   */
  async ingestDocument(input, options = {}) {
    const { waitForCompletion = false, ...ingestionOptions } = options;
    const formData = new FormData();
    if (typeof input.content === "string") {
      formData.append("content", input.content);
    } else {
      const arrayBuffer = input.content instanceof ArrayBuffer ? input.content : new Uint8Array(input.content);
      formData.append(
        "file",
        new Blob([arrayBuffer]),
        input.filename ?? "document"
      );
    }
    if (input.filename) {
      formData.append("filename", input.filename);
    }
    if (input.contentType) {
      formData.append("contentType", input.contentType);
    }
    if (input.metadata) {
      formData.append("metadata", JSON.stringify(input.metadata));
    }
    Object.entries(ingestionOptions).forEach(([key, value]) => {
      if (value !== void 0) {
        formData.append(key, String(value));
      }
    });
    const response = await this.client.post(
      "/api/v1/context/documents",
      { ...input, ...ingestionOptions }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to ingest document", "INGESTION_FAILED");
    }
    if (waitForCompletion) {
      return this.waitForIngestion(response.data.id);
    }
    return this.parseDocument(response.data);
  }
  /**
   * Ingest multiple documents
   */
  async ingestDocuments(inputs, options = {}) {
    const results = await Promise.all(
      inputs.map((input) => this.ingestDocument(input, options))
    );
    return results;
  }
  /**
   * Get document by ID
   */
  async getDocument(documentId) {
    const response = await this.client.get(
      `/api/v1/context/documents/${documentId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Document not found", "NOT_FOUND", 404);
    }
    return this.parseDocument(response.data);
  }
  /**
   * List documents
   */
  async listDocuments(params) {
    const response = await this.client.paginate(
      "/api/v1/context/documents",
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list documents", "LIST_DOCUMENTS_FAILED");
    }
    return {
      ...response.data,
      items: response.data.items.map((d) => this.parseDocument(d))
    };
  }
  /**
   * Delete a document
   */
  async deleteDocument(documentId) {
    const response = await this.client.delete(
      `/api/v1/context/documents/${documentId}`
    );
    if (!response.success) {
      throw new CopilotError("Failed to delete document", "DELETE_DOCUMENT_FAILED");
    }
  }
  /**
   * Get document chunks
   */
  async getDocumentChunks(documentId, params) {
    const response = await this.client.paginate(
      `/api/v1/context/documents/${documentId}/chunks`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to get document chunks", "GET_CHUNKS_FAILED");
    }
    return response.data;
  }
  /**
   * Wait for document ingestion to complete
   */
  async waitForIngestion(documentId, options = {}) {
    const { pollInterval = 1e3, timeout = 3e5 } = options;
    const startTime = Date.now();
    while (true) {
      const document = await this.getDocument(documentId);
      if (document.status === "completed" || document.status === "failed") {
        return document;
      }
      if (Date.now() - startTime > timeout) {
        throw new CopilotError("Document ingestion timeout", "INGESTION_TIMEOUT");
      }
      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }
  }
  // ============================================================================
  // Context Window Management
  // ============================================================================
  /**
   * Build context window for a query
   */
  async buildContextWindow(query, options = {}) {
    const response = await this.client.post("/api/v1/context/window", {
      query,
      ...options
    });
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to build context window",
        "BUILD_WINDOW_FAILED"
      );
    }
    return response.data;
  }
  /**
   * Compress context to fit token budget
   */
  async compressContext(items, maxTokens) {
    const response = await this.client.post("/api/v1/context/compress", {
      items,
      maxTokens
    });
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to compress context", "COMPRESS_FAILED");
    }
    return response.data;
  }
  /**
   * Parse document response to ensure proper types
   */
  parseDocument(data) {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt)
    };
  }
};

// src/types.ts
var DEFAULT_CONFIG = {
  timeout: 3e4,
  maxRetries: 3,
  debug: false
};

// src/index.ts
var CopilotClient = class _CopilotClient {
  httpClient;
  /** Conversations API */
  conversations;
  /** Workflows API */
  workflows;
  /** Context API */
  context;
  constructor(config) {
    this.httpClient = new HttpClient(config);
    this.conversations = new ConversationsClient(this.httpClient);
    this.workflows = new WorkflowsClient(this.httpClient);
    this.context = new ContextClient(this.httpClient);
  }
  /**
   * Create a client from environment variables
   */
  static fromEnv(overrides) {
    const baseUrl = overrides?.baseUrl ?? process.env["COPILOT_API_URL"] ?? process.env["COPILOT_BASE_URL"];
    const apiKey = overrides?.apiKey ?? process.env["COPILOT_API_KEY"];
    const tenantId = overrides?.tenantId ?? process.env["COPILOT_TENANT_ID"];
    if (!baseUrl) {
      throw new CopilotError(
        "Missing API URL. Set COPILOT_API_URL environment variable or pass baseUrl in config.",
        "CONFIG_ERROR"
      );
    }
    if (!apiKey) {
      throw new CopilotError(
        "Missing API key. Set COPILOT_API_KEY environment variable or pass apiKey in config.",
        "CONFIG_ERROR"
      );
    }
    return new _CopilotClient({
      baseUrl,
      apiKey,
      tenantId,
      ...overrides
    });
  }
  /**
   * Get SDK version
   */
  static get version() {
    return "0.1.0";
  }
  /**
   * Health check
   */
  async healthCheck() {
    const response = await this.httpClient.get("/health");
    if (!response.success || !response.data) {
      throw new CopilotError("Health check failed", "HEALTH_CHECK_FAILED");
    }
    return response.data;
  }
  /**
   * Get API info
   */
  async getApiInfo() {
    const response = await this.httpClient.get("/api/v1");
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to get API info", "API_INFO_FAILED");
    }
    return response.data;
  }
};
var src_default = CopilotClient;

export { ContextClient, ConversationsClient, CopilotClient, CopilotError, DEFAULT_CONFIG, HttpClient, WorkflowsClient, src_default as default };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map