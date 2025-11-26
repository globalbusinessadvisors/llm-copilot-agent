/**
 * RAG (Retrieval Augmented Generation) Types
 *
 * Types for document ingestion, chunking, retrieval, and generation.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum DocumentStatus {
  PENDING = 'pending',
  PROCESSING = 'processing',
  INDEXED = 'indexed',
  FAILED = 'failed',
  ARCHIVED = 'archived',
}

export enum DocumentType {
  TEXT = 'text',
  MARKDOWN = 'markdown',
  HTML = 'html',
  PDF = 'pdf',
  DOCX = 'docx',
  CODE = 'code',
  JSON = 'json',
  CSV = 'csv',
  IMAGE = 'image',
  AUDIO = 'audio',
}

export enum ChunkingStrategy {
  FIXED_SIZE = 'fixed_size',
  SENTENCE = 'sentence',
  PARAGRAPH = 'paragraph',
  SEMANTIC = 'semantic',
  RECURSIVE = 'recursive',
  CODE_AWARE = 'code_aware',
  MARKDOWN_HEADER = 'markdown_header',
}

export enum RetrievalStrategy {
  VECTOR_SIMILARITY = 'vector_similarity',
  KEYWORD = 'keyword',
  HYBRID = 'hybrid',
  RERANKING = 'reranking',
  MULTI_QUERY = 'multi_query',
  CONTEXTUAL = 'contextual',
}

export enum VectorStoreProvider {
  PINECONE = 'pinecone',
  CHROMA = 'chroma',
  WEAVIATE = 'weaviate',
  QDRANT = 'qdrant',
  MILVUS = 'milvus',
  PGVECTOR = 'pgvector',
  MEMORY = 'memory',
}

// ===========================================
// Document Schemas
// ===========================================

export const DocumentSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(255),
  type: z.nativeEnum(DocumentType),
  status: z.nativeEnum(DocumentStatus),

  // Source information
  source: z.object({
    type: z.enum(['upload', 'url', 'api', 'connector']),
    url: z.string().optional(),
    connectorId: z.string().optional(),
    metadata: z.record(z.unknown()).optional(),
  }),

  // Content info
  content: z.object({
    size: z.number(),
    mimeType: z.string(),
    encoding: z.string().optional(),
    language: z.string().optional(),
    hash: z.string(), // For deduplication
  }),

  // Processing info
  processing: z.object({
    chunksCount: z.number().default(0),
    tokensCount: z.number().default(0),
    embeddingModel: z.string().optional(),
    chunkingStrategy: z.nativeEnum(ChunkingStrategy).optional(),
    processedAt: z.date().optional(),
    errorMessage: z.string().optional(),
  }),

  // Organization
  collectionId: z.string().uuid(),
  tags: z.array(z.string()).default([]),
  metadata: z.record(z.unknown()).default({}),

  // Access control
  tenantId: z.string().uuid(),
  visibility: z.enum(['private', 'team', 'public']).default('private'),

  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export const DocumentChunkSchema = z.object({
  id: z.string().uuid(),
  documentId: z.string().uuid(),
  collectionId: z.string().uuid(),

  // Content
  content: z.string(),
  contentHash: z.string(),

  // Position
  position: z.object({
    index: z.number(),
    start: z.number(),
    end: z.number(),
    page: z.number().optional(),
    section: z.string().optional(),
  }),

  // Embedding
  embedding: z.array(z.number()).optional(),
  embeddingModel: z.string(),
  embeddingDimension: z.number(),

  // Metadata for retrieval
  metadata: z.object({
    title: z.string().optional(),
    summary: z.string().optional(),
    keywords: z.array(z.string()).optional(),
    entities: z.array(z.object({
      type: z.string(),
      value: z.string(),
    })).optional(),
    parentChunkId: z.string().uuid().optional(),
    childChunkIds: z.array(z.string().uuid()).optional(),
  }),

  // Token info
  tokenCount: z.number(),

  createdAt: z.date(),
});

// ===========================================
// Collection Schemas
// ===========================================

export const CollectionSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),

  // Vector store configuration
  vectorStore: z.object({
    provider: z.nativeEnum(VectorStoreProvider),
    namespace: z.string(),
    dimension: z.number(),
    metric: z.enum(['cosine', 'euclidean', 'dotProduct']).default('cosine'),
    indexType: z.string().optional(),
  }),

  // Embedding configuration
  embedding: z.object({
    modelId: z.string(),
    modelName: z.string(),
    dimension: z.number(),
    batchSize: z.number().default(100),
  }),

  // Chunking configuration
  chunking: z.object({
    strategy: z.nativeEnum(ChunkingStrategy),
    chunkSize: z.number().default(512),
    chunkOverlap: z.number().default(50),
    separators: z.array(z.string()).optional(),
    minChunkSize: z.number().optional(),
    maxChunkSize: z.number().optional(),
  }),

  // Stats
  stats: z.object({
    documentsCount: z.number().default(0),
    chunksCount: z.number().default(0),
    totalTokens: z.number().default(0),
    lastUpdated: z.date().optional(),
  }),

  // Access control
  tenantId: z.string().uuid(),
  visibility: z.enum(['private', 'team', 'public']).default('private'),

  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

// ===========================================
// Retrieval Schemas
// ===========================================

export const RetrievalConfigSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),

  // Collections to search
  collectionIds: z.array(z.string().uuid()),

  // Retrieval strategy
  strategy: z.nativeEnum(RetrievalStrategy),

  // Strategy-specific config
  strategyConfig: z.object({
    // Vector similarity
    topK: z.number().default(5),
    scoreThreshold: z.number().min(0).max(1).optional(),

    // Hybrid search
    hybridAlpha: z.number().min(0).max(1).default(0.5), // 0 = keyword only, 1 = vector only
    keywordWeight: z.number().optional(),
    vectorWeight: z.number().optional(),

    // Multi-query
    numQueries: z.number().default(3),
    queryPrompt: z.string().optional(),

    // Reranking
    rerankerModel: z.string().optional(),
    rerankerTopK: z.number().optional(),

    // Contextual compression
    compressionEnabled: z.boolean().default(false),
    compressionModel: z.string().optional(),

    // MMR (Maximum Marginal Relevance)
    mmrEnabled: z.boolean().default(false),
    mmrLambda: z.number().min(0).max(1).default(0.5),
  }),

  // Filtering
  filters: z.object({
    metadataFilters: z.array(z.object({
      field: z.string(),
      operator: z.enum(['eq', 'neq', 'gt', 'gte', 'lt', 'lte', 'in', 'nin', 'contains']),
      value: z.unknown(),
    })).optional(),
    documentTypes: z.array(z.nativeEnum(DocumentType)).optional(),
    tags: z.array(z.string()).optional(),
    dateRange: z.object({
      start: z.date().optional(),
      end: z.date().optional(),
    }).optional(),
  }).default({}),

  // Post-processing
  postProcessing: z.object({
    deduplication: z.boolean().default(true),
    maxTokens: z.number().optional(),
    includeMetadata: z.boolean().default(true),
    citationFormat: z.enum(['none', 'inline', 'footnote', 'endnote']).default('inline'),
  }),

  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const RetrievalResultSchema = z.object({
  id: z.string().uuid(),
  query: z.string(),
  configId: z.string().uuid(),

  // Results
  results: z.array(z.object({
    chunkId: z.string().uuid(),
    documentId: z.string().uuid(),
    documentName: z.string(),
    content: z.string(),
    score: z.number(),
    rerankerScore: z.number().optional(),
    metadata: z.record(z.unknown()),
    highlights: z.array(z.object({
      start: z.number(),
      end: z.number(),
      text: z.string(),
    })).optional(),
  })),

  // Metrics
  metrics: z.object({
    totalResults: z.number(),
    returnedResults: z.number(),
    searchTimeMs: z.number(),
    rerankTimeMs: z.number().optional(),
    totalTimeMs: z.number(),
  }),

  // Debug info (if verbose)
  debug: z.object({
    expandedQueries: z.array(z.string()).optional(),
    filterApplied: z.unknown().optional(),
    embeddingTimeMs: z.number().optional(),
  }).optional(),

  createdAt: z.date(),
});

// ===========================================
// RAG Pipeline Schemas
// ===========================================

export const RAGPipelineSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1).max(100),
  description: z.string().optional(),

  // Components
  retrieval: z.object({
    configId: z.string().uuid(),
    enabled: z.boolean().default(true),
  }),

  // Generation config
  generation: z.object({
    modelId: z.string().uuid(),
    systemPrompt: z.string().optional(),
    contextPromptTemplate: z.string().default(
      'Use the following context to answer the question:\n\n{context}\n\nQuestion: {question}'
    ),
    maxContextTokens: z.number().default(4000),
    temperature: z.number().min(0).max(2).default(0.7),
    streamResponse: z.boolean().default(true),
  }),

  // Source attribution
  attribution: z.object({
    enabled: z.boolean().default(true),
    format: z.enum(['citations', 'sources_section', 'inline_links']).default('citations'),
    includeConfidence: z.boolean().default(false),
  }),

  // Guardrails
  guardrails: z.object({
    requireSources: z.boolean().default(false),
    minSourceScore: z.number().min(0).max(1).optional(),
    maxSourceAge: z.number().optional(), // days
    factCheckEnabled: z.boolean().default(false),
  }),

  // Caching
  caching: z.object({
    enabled: z.boolean().default(true),
    ttlSeconds: z.number().default(3600),
    cacheKey: z.enum(['query', 'query_context', 'full']).default('query_context'),
  }),

  enabled: z.boolean().default(true),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export const RAGResponseSchema = z.object({
  id: z.string().uuid(),
  pipelineId: z.string().uuid(),
  sessionId: z.string().uuid().optional(),

  // Input
  query: z.string(),
  conversationHistory: z.array(z.object({
    role: z.enum(['user', 'assistant', 'system']),
    content: z.string(),
  })).optional(),

  // Retrieval
  retrieval: z.object({
    resultId: z.string().uuid(),
    sourcesUsed: z.number(),
    totalSources: z.number(),
  }),

  // Response
  response: z.object({
    content: z.string(),
    citations: z.array(z.object({
      sourceIndex: z.number(),
      documentId: z.string().uuid(),
      documentName: z.string(),
      chunkId: z.string().uuid(),
      excerpt: z.string(),
      confidence: z.number().optional(),
    })).optional(),
    finishReason: z.enum(['stop', 'length', 'content_filter', 'tool_calls']),
  }),

  // Metrics
  metrics: z.object({
    retrievalTimeMs: z.number(),
    generationTimeMs: z.number(),
    totalTimeMs: z.number(),
    inputTokens: z.number(),
    outputTokens: z.number(),
    contextTokens: z.number(),
    cost: z.number().optional(),
    cacheHit: z.boolean(),
  }),

  // Quality signals
  quality: z.object({
    groundedness: z.number().optional(), // 0-1, how well grounded in sources
    relevance: z.number().optional(),    // 0-1, relevance to query
    coherence: z.number().optional(),    // 0-1, response coherence
  }).optional(),

  createdAt: z.date(),
});

// ===========================================
// Types
// ===========================================

export type Document = z.infer<typeof DocumentSchema>;
export type DocumentChunk = z.infer<typeof DocumentChunkSchema>;
export type Collection = z.infer<typeof CollectionSchema>;
export type RetrievalConfig = z.infer<typeof RetrievalConfigSchema>;
export type RetrievalResult = z.infer<typeof RetrievalResultSchema>;
export type RAGPipeline = z.infer<typeof RAGPipelineSchema>;
export type RAGResponse = z.infer<typeof RAGResponseSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateCollectionInput {
  name: string;
  description?: string;
  vectorStore: {
    provider: VectorStoreProvider;
    dimension?: number;
    metric?: 'cosine' | 'euclidean' | 'dotProduct';
  };
  embedding: {
    modelId: string;
    batchSize?: number;
  };
  chunking: {
    strategy: ChunkingStrategy;
    chunkSize?: number;
    chunkOverlap?: number;
  };
  visibility?: 'private' | 'team' | 'public';
}

export interface IngestDocumentInput {
  collectionId: string;
  name: string;
  type: DocumentType;
  source: {
    type: 'upload' | 'url' | 'api';
    content?: string;
    url?: string;
  };
  tags?: string[];
  metadata?: Record<string, unknown>;
}

export interface CreateRetrievalConfigInput {
  name: string;
  description?: string;
  collectionIds: string[];
  strategy: RetrievalStrategy;
  strategyConfig?: Partial<RetrievalConfig['strategyConfig']>;
  filters?: Partial<RetrievalConfig['filters']>;
  postProcessing?: Partial<RetrievalConfig['postProcessing']>;
}

export interface CreateRAGPipelineInput {
  name: string;
  description?: string;
  retrievalConfigId: string;
  generation: {
    modelId: string;
    systemPrompt?: string;
    contextPromptTemplate?: string;
    maxContextTokens?: number;
    temperature?: number;
  };
  attribution?: Partial<RAGPipeline['attribution']>;
  guardrails?: Partial<RAGPipeline['guardrails']>;
  caching?: Partial<RAGPipeline['caching']>;
}

export interface QueryRAGInput {
  pipelineId: string;
  query: string;
  conversationHistory?: Array<{
    role: 'user' | 'assistant' | 'system';
    content: string;
  }>;
  sessionId?: string;
  filters?: Record<string, unknown>;
  stream?: boolean;
}
