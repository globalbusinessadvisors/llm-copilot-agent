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
var context_default = ContextClient;

exports.ContextClient = ContextClient;
exports.default = context_default;
//# sourceMappingURL=index.js.map
//# sourceMappingURL=index.js.map