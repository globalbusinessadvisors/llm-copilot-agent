/**
 * Context API client
 */

import { HttpClient, CopilotError } from '../client';
import type {
  ContextItem,
  ContextSearchParams,
  ContextSearchResult,
  DocumentInput,
  IngestedDocument,
  DocumentChunk,
  PaginatedResponse,
  PaginationParams,
} from '../types';

/**
 * Context search options
 */
interface SearchOptions extends ContextSearchParams {
  /** Include vector scores in results */
  includeScores?: boolean;
  /** Use hybrid search (vector + keyword) */
  hybrid?: boolean;
  /** Rerank results */
  rerank?: boolean;
}

/**
 * Document ingestion options
 */
interface IngestionOptions {
  /** Chunking strategy */
  chunkingStrategy?: 'fixed' | 'sentence' | 'paragraph' | 'recursive';
  /** Target chunk size in tokens */
  chunkSize?: number;
  /** Overlap between chunks */
  chunkOverlap?: number;
  /** Generate embeddings */
  generateEmbeddings?: boolean;
  /** Wait for processing to complete */
  waitForCompletion?: boolean;
}

/**
 * Context API client
 */
export class ContextClient {
  constructor(private readonly client: HttpClient) {}

  // ============================================================================
  // Context Search
  // ============================================================================

  /**
   * Search context
   */
  async search(options: SearchOptions): Promise<ContextSearchResult> {
    const response = await this.client.post<ContextSearchResult>(
      '/api/v1/context/search',
      options
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Search failed', 'SEARCH_FAILED');
    }

    return response.data;
  }

  /**
   * Vector similarity search
   */
  async vectorSearch(
    query: string,
    options: { limit?: number; threshold?: number } = {}
  ): Promise<ContextItem[]> {
    const result = await this.search({
      query,
      ...options,
      hybrid: false,
    });

    return result.items;
  }

  /**
   * Hybrid search (vector + keyword)
   */
  async hybridSearch(
    query: string,
    options: {
      limit?: number;
      threshold?: number;
      vectorWeight?: number;
      keywordWeight?: number;
    } = {}
  ): Promise<ContextItem[]> {
    const result = await this.search({
      query,
      limit: options.limit,
      threshold: options.threshold,
      hybrid: true,
      filters: {
        vectorWeight: options.vectorWeight ?? 0.7,
        keywordWeight: options.keywordWeight ?? 0.3,
      },
    });

    return result.items;
  }

  /**
   * Search with reranking
   */
  async searchWithRerank(
    query: string,
    options: { limit?: number; initialLimit?: number } = {}
  ): Promise<ContextItem[]> {
    const result = await this.search({
      query,
      limit: options.limit ?? 10,
      filters: {
        initialLimit: options.initialLimit ?? 50,
      },
      rerank: true,
    });

    return result.items;
  }

  // ============================================================================
  // Memory Management
  // ============================================================================

  /**
   * Store a memory item
   */
  async storeMemory(
    content: string,
    options: {
      type?: 'short_term' | 'long_term' | 'episodic';
      importance?: number;
      metadata?: Record<string, unknown>;
    } = {}
  ): Promise<ContextItem> {
    const response = await this.client.post<ContextItem>('/api/v1/context/memory', {
      content,
      ...options,
    });

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to store memory', 'STORE_MEMORY_FAILED');
    }

    return response.data;
  }

  /**
   * Get memory by ID
   */
  async getMemory(memoryId: string): Promise<ContextItem> {
    const response = await this.client.get<ContextItem>(
      `/api/v1/context/memory/${memoryId}`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Memory not found', 'NOT_FOUND', 404);
    }

    return response.data;
  }

  /**
   * List memories
   */
  async listMemories(
    params?: PaginationParams & { type?: string }
  ): Promise<PaginatedResponse<ContextItem>> {
    const response = await this.client.paginate<ContextItem>(
      '/api/v1/context/memory',
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list memories', 'LIST_MEMORIES_FAILED');
    }

    return response.data;
  }

  /**
   * Delete a memory
   */
  async deleteMemory(memoryId: string): Promise<void> {
    const response = await this.client.delete(`/api/v1/context/memory/${memoryId}`);

    if (!response.success) {
      throw new CopilotError('Failed to delete memory', 'DELETE_MEMORY_FAILED');
    }
  }

  /**
   * Consolidate memories
   */
  async consolidateMemories(): Promise<{ consolidated: number; remaining: number }> {
    const response = await this.client.post<{
      consolidated: number;
      remaining: number;
    }>('/api/v1/context/memory/consolidate');

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to consolidate memories',
        'CONSOLIDATE_FAILED'
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
  async ingestDocument(
    input: DocumentInput,
    options: IngestionOptions = {}
  ): Promise<IngestedDocument> {
    const { waitForCompletion = false, ...ingestionOptions } = options;

    // Prepare form data for file upload
    const formData = new FormData();

    if (typeof input.content === 'string') {
      formData.append('content', input.content);
    } else {
      // Convert Buffer to Uint8Array for Blob compatibility
      const arrayBuffer = input.content instanceof ArrayBuffer
        ? input.content
        : new Uint8Array(input.content as Buffer);
      formData.append(
        'file',
        new Blob([arrayBuffer]),
        input.filename ?? 'document'
      );
    }

    if (input.filename) {
      formData.append('filename', input.filename);
    }
    if (input.contentType) {
      formData.append('contentType', input.contentType);
    }
    if (input.metadata) {
      formData.append('metadata', JSON.stringify(input.metadata));
    }

    Object.entries(ingestionOptions).forEach(([key, value]) => {
      if (value !== undefined) {
        formData.append(key, String(value));
      }
    });

    // Note: In a real implementation, we'd need to handle FormData differently
    const response = await this.client.post<IngestedDocument>(
      '/api/v1/context/documents',
      { ...input, ...ingestionOptions }
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to ingest document', 'INGESTION_FAILED');
    }

    if (waitForCompletion) {
      return this.waitForIngestion(response.data.id);
    }

    return this.parseDocument(response.data);
  }

  /**
   * Ingest multiple documents
   */
  async ingestDocuments(
    inputs: DocumentInput[],
    options: IngestionOptions = {}
  ): Promise<IngestedDocument[]> {
    const results = await Promise.all(
      inputs.map((input) => this.ingestDocument(input, options))
    );

    return results;
  }

  /**
   * Get document by ID
   */
  async getDocument(documentId: string): Promise<IngestedDocument> {
    const response = await this.client.get<IngestedDocument>(
      `/api/v1/context/documents/${documentId}`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Document not found', 'NOT_FOUND', 404);
    }

    return this.parseDocument(response.data);
  }

  /**
   * List documents
   */
  async listDocuments(
    params?: PaginationParams & { status?: string }
  ): Promise<PaginatedResponse<IngestedDocument>> {
    const response = await this.client.paginate<IngestedDocument>(
      '/api/v1/context/documents',
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list documents', 'LIST_DOCUMENTS_FAILED');
    }

    return {
      ...response.data,
      items: response.data.items.map((d) => this.parseDocument(d)),
    };
  }

  /**
   * Delete a document
   */
  async deleteDocument(documentId: string): Promise<void> {
    const response = await this.client.delete(
      `/api/v1/context/documents/${documentId}`
    );

    if (!response.success) {
      throw new CopilotError('Failed to delete document', 'DELETE_DOCUMENT_FAILED');
    }
  }

  /**
   * Get document chunks
   */
  async getDocumentChunks(
    documentId: string,
    params?: PaginationParams
  ): Promise<PaginatedResponse<DocumentChunk>> {
    const response = await this.client.paginate<DocumentChunk>(
      `/api/v1/context/documents/${documentId}/chunks`,
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to get document chunks', 'GET_CHUNKS_FAILED');
    }

    return response.data;
  }

  /**
   * Wait for document ingestion to complete
   */
  async waitForIngestion(
    documentId: string,
    options: { pollInterval?: number; timeout?: number } = {}
  ): Promise<IngestedDocument> {
    const { pollInterval = 1000, timeout = 300000 } = options;
    const startTime = Date.now();

    while (true) {
      const document = await this.getDocument(documentId);

      if (document.status === 'completed' || document.status === 'failed') {
        return document;
      }

      if (Date.now() - startTime > timeout) {
        throw new CopilotError('Document ingestion timeout', 'INGESTION_TIMEOUT');
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
  async buildContextWindow(
    query: string,
    options: {
      maxTokens?: number;
      includeMemory?: boolean;
      includeDocuments?: boolean;
      filters?: Record<string, unknown>;
    } = {}
  ): Promise<{ items: ContextItem[]; totalTokens: number }> {
    const response = await this.client.post<{
      items: ContextItem[];
      totalTokens: number;
    }>('/api/v1/context/window', {
      query,
      ...options,
    });

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to build context window',
        'BUILD_WINDOW_FAILED'
      );
    }

    return response.data;
  }

  /**
   * Compress context to fit token budget
   */
  async compressContext(
    items: ContextItem[],
    maxTokens: number
  ): Promise<{ items: ContextItem[]; totalTokens: number }> {
    const response = await this.client.post<{
      items: ContextItem[];
      totalTokens: number;
    }>('/api/v1/context/compress', {
      items,
      maxTokens,
    });

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to compress context', 'COMPRESS_FAILED');
    }

    return response.data;
  }

  /**
   * Parse document response to ensure proper types
   */
  private parseDocument(data: IngestedDocument): IngestedDocument {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
    };
  }
}

export default ContextClient;
