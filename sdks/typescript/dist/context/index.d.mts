import { H as HttpClient, l as ContextSearchParams, m as ContextSearchResult, k as ContextItem, P as PaginationParams, d as PaginatedResponse, F as DocumentInput, I as IngestedDocument, G as DocumentChunk } from '../client-BNP-OnWr.mjs';

/**
 * Context API client
 */

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
declare class ContextClient {
    private readonly client;
    constructor(client: HttpClient);
    /**
     * Search context
     */
    search(options: SearchOptions): Promise<ContextSearchResult>;
    /**
     * Vector similarity search
     */
    vectorSearch(query: string, options?: {
        limit?: number;
        threshold?: number;
    }): Promise<ContextItem[]>;
    /**
     * Hybrid search (vector + keyword)
     */
    hybridSearch(query: string, options?: {
        limit?: number;
        threshold?: number;
        vectorWeight?: number;
        keywordWeight?: number;
    }): Promise<ContextItem[]>;
    /**
     * Search with reranking
     */
    searchWithRerank(query: string, options?: {
        limit?: number;
        initialLimit?: number;
    }): Promise<ContextItem[]>;
    /**
     * Store a memory item
     */
    storeMemory(content: string, options?: {
        type?: 'short_term' | 'long_term' | 'episodic';
        importance?: number;
        metadata?: Record<string, unknown>;
    }): Promise<ContextItem>;
    /**
     * Get memory by ID
     */
    getMemory(memoryId: string): Promise<ContextItem>;
    /**
     * List memories
     */
    listMemories(params?: PaginationParams & {
        type?: string;
    }): Promise<PaginatedResponse<ContextItem>>;
    /**
     * Delete a memory
     */
    deleteMemory(memoryId: string): Promise<void>;
    /**
     * Consolidate memories
     */
    consolidateMemories(): Promise<{
        consolidated: number;
        remaining: number;
    }>;
    /**
     * Ingest a document
     */
    ingestDocument(input: DocumentInput, options?: IngestionOptions): Promise<IngestedDocument>;
    /**
     * Ingest multiple documents
     */
    ingestDocuments(inputs: DocumentInput[], options?: IngestionOptions): Promise<IngestedDocument[]>;
    /**
     * Get document by ID
     */
    getDocument(documentId: string): Promise<IngestedDocument>;
    /**
     * List documents
     */
    listDocuments(params?: PaginationParams & {
        status?: string;
    }): Promise<PaginatedResponse<IngestedDocument>>;
    /**
     * Delete a document
     */
    deleteDocument(documentId: string): Promise<void>;
    /**
     * Get document chunks
     */
    getDocumentChunks(documentId: string, params?: PaginationParams): Promise<PaginatedResponse<DocumentChunk>>;
    /**
     * Wait for document ingestion to complete
     */
    waitForIngestion(documentId: string, options?: {
        pollInterval?: number;
        timeout?: number;
    }): Promise<IngestedDocument>;
    /**
     * Build context window for a query
     */
    buildContextWindow(query: string, options?: {
        maxTokens?: number;
        includeMemory?: boolean;
        includeDocuments?: boolean;
        filters?: Record<string, unknown>;
    }): Promise<{
        items: ContextItem[];
        totalTokens: number;
    }>;
    /**
     * Compress context to fit token budget
     */
    compressContext(items: ContextItem[], maxTokens: number): Promise<{
        items: ContextItem[];
        totalTokens: number;
    }>;
    /**
     * Parse document response to ensure proper types
     */
    private parseDocument;
}

export { ContextClient, ContextClient as default };
