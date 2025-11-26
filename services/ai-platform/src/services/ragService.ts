/**
 * RAG (Retrieval Augmented Generation) Service
 *
 * Manages document ingestion, chunking, embedding, and retrieval for RAG pipelines.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import crypto from 'crypto';
import OpenAI from 'openai';
import {
  Document,
  DocumentChunk,
  Collection,
  RAGPipeline,
  RetrievalConfig,
  ChunkingStrategy,
  RetrievalStrategy,
  VectorStoreProvider,
  CreateDocumentInput,
  CreateCollectionInput,
  CreateRAGPipelineInput,
  RetrievalResult,
} from '../models/rag';

interface EmbeddingProvider {
  embed(texts: string[]): Promise<number[][]>;
  dimension: number;
}

interface VectorStore {
  upsert(vectors: Array<{ id: string; values: number[]; metadata: Record<string, unknown> }>): Promise<void>;
  query(vector: number[], topK: number, filter?: Record<string, unknown>): Promise<Array<{ id: string; score: number; metadata: Record<string, unknown> }>>;
  delete(ids: string[]): Promise<void>;
}

export class RAGService {
  private db: Pool;
  private redis: RedisClientType;
  private openai: OpenAI | null = null;
  private vectorStores: Map<string, VectorStore> = new Map();
  private embeddingProviders: Map<string, EmbeddingProvider> = new Map();

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;

    // Initialize OpenAI client if API key is available
    if (process.env.OPENAI_API_KEY) {
      this.openai = new OpenAI({
        apiKey: process.env.OPENAI_API_KEY,
      });
    }

    this.initializeDefaultProviders();
  }

  /**
   * Initialize default embedding providers
   */
  private initializeDefaultProviders(): void {
    // OpenAI embeddings
    if (this.openai) {
      this.embeddingProviders.set('openai', {
        embed: async (texts: string[]): Promise<number[][]> => {
          const response = await this.openai!.embeddings.create({
            model: 'text-embedding-3-small',
            input: texts,
          });
          return response.data.map(d => d.embedding);
        },
        dimension: 1536,
      });
    }
  }

  // ===========================================
  // Collection Management
  // ===========================================

  /**
   * Create a new collection
   */
  async createCollection(input: CreateCollectionInput, userId: string): Promise<Collection> {
    const collection: Collection = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      embeddingConfig: {
        provider: 'openai',
        model: 'text-embedding-3-small',
        dimension: 1536,
        ...input.embeddingConfig,
      },
      vectorStoreConfig: {
        provider: input.vectorStoreConfig?.provider || VectorStoreProvider.PGVECTOR,
        ...input.vectorStoreConfig,
      },
      chunkingConfig: {
        strategy: ChunkingStrategy.RECURSIVE,
        chunkSize: 512,
        chunkOverlap: 50,
        ...input.chunkingConfig,
      },
      metadata: input.metadata || {},
      documentCount: 0,
      totalChunks: 0,
      status: 'active',
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO rag_collections (
        id, name, description, embedding_config, vector_store_config,
        chunking_config, metadata, document_count, total_chunks, status,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
      [
        collection.id, collection.name, collection.description,
        JSON.stringify(collection.embeddingConfig), JSON.stringify(collection.vectorStoreConfig),
        JSON.stringify(collection.chunkingConfig), JSON.stringify(collection.metadata),
        collection.documentCount, collection.totalChunks, collection.status,
        collection.createdAt, collection.updatedAt, collection.createdBy,
      ]
    );

    // Initialize vector store for collection
    await this.initializeVectorStore(collection);

    return collection;
  }

  /**
   * Get collection by ID
   */
  async getCollection(collectionId: string): Promise<Collection | null> {
    const result = await this.db.query(
      `SELECT * FROM rag_collections WHERE id = $1`,
      [collectionId]
    );

    if (result.rows.length === 0) return null;

    return this.mapCollectionRow(result.rows[0]);
  }

  /**
   * List collections
   */
  async listCollections(filters?: { status?: Collection['status'] }): Promise<Collection[]> {
    let query = `SELECT * FROM rag_collections WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapCollectionRow);
  }

  /**
   * Initialize vector store for a collection
   */
  private async initializeVectorStore(collection: Collection): Promise<void> {
    const provider = collection.vectorStoreConfig.provider;

    switch (provider) {
      case VectorStoreProvider.PGVECTOR:
        // Use PostgreSQL with pgvector extension
        await this.db.query(`
          CREATE TABLE IF NOT EXISTS vector_store_${collection.id.replace(/-/g, '_')} (
            id TEXT PRIMARY KEY,
            embedding vector(${collection.embeddingConfig.dimension}),
            metadata JSONB,
            created_at TIMESTAMP DEFAULT NOW()
          )
        `);

        // Create index for vector similarity search
        await this.db.query(`
          CREATE INDEX IF NOT EXISTS idx_vector_store_${collection.id.replace(/-/g, '_')}_embedding
          ON vector_store_${collection.id.replace(/-/g, '_')}
          USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100)
        `).catch(() => {
          // Index might already exist or pgvector not installed
        });

        this.vectorStores.set(collection.id, this.createPGVectorStore(collection));
        break;

      case VectorStoreProvider.MEMORY:
        this.vectorStores.set(collection.id, this.createInMemoryVectorStore());
        break;

      // Other providers would be initialized here
      default:
        throw new Error(`Vector store provider ${provider} not implemented`);
    }
  }

  /**
   * Create PGVector store implementation
   */
  private createPGVectorStore(collection: Collection): VectorStore {
    const tableName = `vector_store_${collection.id.replace(/-/g, '_')}`;

    return {
      upsert: async (vectors) => {
        for (const vector of vectors) {
          await this.db.query(
            `INSERT INTO ${tableName} (id, embedding, metadata)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET embedding = $2, metadata = $3`,
            [vector.id, JSON.stringify(vector.values), JSON.stringify(vector.metadata)]
          );
        }
      },
      query: async (vector, topK, filter) => {
        let query = `
          SELECT id, metadata, 1 - (embedding <=> $1::vector) as score
          FROM ${tableName}
        `;
        const values: unknown[] = [JSON.stringify(vector)];

        if (filter) {
          query += ` WHERE metadata @> $2`;
          values.push(JSON.stringify(filter));
        }

        query += ` ORDER BY embedding <=> $1::vector LIMIT $${values.length + 1}`;
        values.push(topK);

        const result = await this.db.query(query, values);
        return result.rows.map(row => ({
          id: row.id,
          score: row.score,
          metadata: row.metadata,
        }));
      },
      delete: async (ids) => {
        await this.db.query(
          `DELETE FROM ${tableName} WHERE id = ANY($1)`,
          [ids]
        );
      },
    };
  }

  /**
   * Create in-memory vector store (for testing)
   */
  private createInMemoryVectorStore(): VectorStore {
    const store: Map<string, { values: number[]; metadata: Record<string, unknown> }> = new Map();

    return {
      upsert: async (vectors) => {
        for (const vector of vectors) {
          store.set(vector.id, { values: vector.values, metadata: vector.metadata });
        }
      },
      query: async (vector, topK) => {
        const results: Array<{ id: string; score: number; metadata: Record<string, unknown> }> = [];

        store.forEach((stored, id) => {
          const score = this.cosineSimilarity(vector, stored.values);
          results.push({ id, score, metadata: stored.metadata });
        });

        return results
          .sort((a, b) => b.score - a.score)
          .slice(0, topK);
      },
      delete: async (ids) => {
        ids.forEach(id => store.delete(id));
      },
    };
  }

  /**
   * Calculate cosine similarity between two vectors
   */
  private cosineSimilarity(a: number[], b: number[]): number {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  // ===========================================
  // Document Management
  // ===========================================

  /**
   * Ingest a document into a collection
   */
  async ingestDocument(
    collectionId: string,
    input: CreateDocumentInput,
    userId: string
  ): Promise<Document> {
    const collection = await this.getCollection(collectionId);
    if (!collection) throw new Error('Collection not found');

    // Calculate content hash for deduplication
    const contentHash = crypto.createHash('sha256').update(input.content).digest('hex');

    // Check for duplicate
    const existingDoc = await this.db.query(
      `SELECT id FROM rag_documents WHERE collection_id = $1 AND content_hash = $2`,
      [collectionId, contentHash]
    );

    if (existingDoc.rows.length > 0) {
      throw new Error(`Document with same content already exists: ${existingDoc.rows[0].id}`);
    }

    const document: Document = {
      id: uuidv4(),
      collectionId,
      title: input.title,
      content: input.content,
      contentType: input.contentType || 'text/plain',
      source: input.source,
      metadata: input.metadata || {},
      contentHash,
      status: 'processing',
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO rag_documents (
        id, collection_id, title, content, content_type, source,
        metadata, content_hash, status, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        document.id, document.collectionId, document.title, document.content,
        document.contentType, JSON.stringify(document.source), JSON.stringify(document.metadata),
        document.contentHash, document.status, document.createdAt, document.updatedAt, document.createdBy,
      ]
    );

    // Process document asynchronously
    this.processDocument(document, collection).catch(error => {
      console.error(`Error processing document ${document.id}:`, error);
      this.updateDocumentStatus(document.id, 'failed').catch(console.error);
    });

    return document;
  }

  /**
   * Process document: chunk, embed, and store
   */
  private async processDocument(document: Document, collection: Collection): Promise<void> {
    try {
      // Chunk the document
      const chunks = await this.chunkDocument(document, collection.chunkingConfig);

      // Store chunks
      for (const chunk of chunks) {
        await this.db.query(
          `INSERT INTO rag_document_chunks (
            id, document_id, collection_id, content, chunk_index,
            start_offset, end_offset, metadata, created_at
          ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())`,
          [
            chunk.id, chunk.documentId, chunk.collectionId, chunk.content,
            chunk.chunkIndex, chunk.startOffset, chunk.endOffset,
            JSON.stringify(chunk.metadata),
          ]
        );
      }

      // Generate embeddings
      const embeddings = await this.generateEmbeddings(
        chunks.map(c => c.content),
        collection.embeddingConfig
      );

      // Store in vector store
      const vectorStore = this.vectorStores.get(collection.id);
      if (!vectorStore) {
        await this.initializeVectorStore(collection);
      }

      const store = this.vectorStores.get(collection.id)!;
      await store.upsert(
        chunks.map((chunk, i) => ({
          id: chunk.id,
          values: embeddings[i],
          metadata: {
            documentId: document.id,
            collectionId: collection.id,
            chunkIndex: chunk.chunkIndex,
            title: document.title,
            ...chunk.metadata,
          },
        }))
      );

      // Update chunk records with embeddings
      for (let i = 0; i < chunks.length; i++) {
        await this.db.query(
          `UPDATE rag_document_chunks SET embedding = $1 WHERE id = $2`,
          [JSON.stringify(embeddings[i]), chunks[i].id]
        );
      }

      // Update document status
      await this.updateDocumentStatus(document.id, 'ready', chunks.length);

      // Update collection stats
      await this.db.query(
        `UPDATE rag_collections SET
          document_count = document_count + 1,
          total_chunks = total_chunks + $1,
          updated_at = NOW()
        WHERE id = $2`,
        [chunks.length, collection.id]
      );
    } catch (error) {
      await this.updateDocumentStatus(document.id, 'failed');
      throw error;
    }
  }

  /**
   * Chunk document based on strategy
   */
  private async chunkDocument(
    document: Document,
    config: Collection['chunkingConfig']
  ): Promise<DocumentChunk[]> {
    const chunks: DocumentChunk[] = [];
    const content = document.content;

    switch (config.strategy) {
      case ChunkingStrategy.FIXED_SIZE:
        chunks.push(...this.fixedSizeChunking(document, content, config));
        break;
      case ChunkingStrategy.SENTENCE:
        chunks.push(...this.sentenceChunking(document, content, config));
        break;
      case ChunkingStrategy.PARAGRAPH:
        chunks.push(...this.paragraphChunking(document, content, config));
        break;
      case ChunkingStrategy.RECURSIVE:
        chunks.push(...this.recursiveChunking(document, content, config));
        break;
      case ChunkingStrategy.SEMANTIC:
        chunks.push(...await this.semanticChunking(document, content, config));
        break;
      case ChunkingStrategy.CODE_AWARE:
        chunks.push(...this.codeAwareChunking(document, content, config));
        break;
      case ChunkingStrategy.MARKDOWN_HEADER:
        chunks.push(...this.markdownHeaderChunking(document, content, config));
        break;
      default:
        chunks.push(...this.recursiveChunking(document, content, config));
    }

    return chunks;
  }

  /**
   * Fixed size chunking
   */
  private fixedSizeChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = [];
    const chunkSize = config.chunkSize || 512;
    const overlap = config.chunkOverlap || 50;

    let startOffset = 0;
    let chunkIndex = 0;

    while (startOffset < content.length) {
      const endOffset = Math.min(startOffset + chunkSize, content.length);
      const chunkContent = content.slice(startOffset, endOffset);

      chunks.push({
        id: uuidv4(),
        documentId: document.id,
        collectionId: document.collectionId,
        content: chunkContent,
        chunkIndex,
        startOffset,
        endOffset,
        metadata: {},
        createdAt: new Date(),
      });

      startOffset = endOffset - overlap;
      if (startOffset >= content.length - overlap) break;
      chunkIndex++;
    }

    return chunks;
  }

  /**
   * Sentence-based chunking
   */
  private sentenceChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = [];
    const chunkSize = config.chunkSize || 512;

    // Split by sentence boundaries
    const sentences = content.match(/[^.!?]+[.!?]+/g) || [content];
    let currentChunk = '';
    let startOffset = 0;
    let chunkIndex = 0;
    let currentOffset = 0;

    for (const sentence of sentences) {
      if (currentChunk.length + sentence.length > chunkSize && currentChunk.length > 0) {
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: currentChunk.trim(),
          chunkIndex,
          startOffset,
          endOffset: currentOffset,
          metadata: {},
          createdAt: new Date(),
        });

        startOffset = currentOffset;
        currentChunk = '';
        chunkIndex++;
      }

      currentChunk += sentence;
      currentOffset += sentence.length;
    }

    // Add remaining content
    if (currentChunk.trim().length > 0) {
      chunks.push({
        id: uuidv4(),
        documentId: document.id,
        collectionId: document.collectionId,
        content: currentChunk.trim(),
        chunkIndex,
        startOffset,
        endOffset: content.length,
        metadata: {},
        createdAt: new Date(),
      });
    }

    return chunks;
  }

  /**
   * Paragraph-based chunking
   */
  private paragraphChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = [];
    const chunkSize = config.chunkSize || 1024;

    // Split by paragraph (double newlines)
    const paragraphs = content.split(/\n\s*\n/);
    let currentChunk = '';
    let startOffset = 0;
    let chunkIndex = 0;
    let currentOffset = 0;

    for (const paragraph of paragraphs) {
      if (currentChunk.length + paragraph.length > chunkSize && currentChunk.length > 0) {
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: currentChunk.trim(),
          chunkIndex,
          startOffset,
          endOffset: currentOffset,
          metadata: {},
          createdAt: new Date(),
        });

        startOffset = currentOffset;
        currentChunk = '';
        chunkIndex++;
      }

      currentChunk += paragraph + '\n\n';
      currentOffset += paragraph.length + 2;
    }

    if (currentChunk.trim().length > 0) {
      chunks.push({
        id: uuidv4(),
        documentId: document.id,
        collectionId: document.collectionId,
        content: currentChunk.trim(),
        chunkIndex,
        startOffset,
        endOffset: content.length,
        metadata: {},
        createdAt: new Date(),
      });
    }

    return chunks;
  }

  /**
   * Recursive chunking (LangChain-style)
   */
  private recursiveChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunkSize = config.chunkSize || 512;
    const overlap = config.chunkOverlap || 50;
    const separators = ['\n\n', '\n', '. ', ' ', ''];

    const splitText = (text: string, separatorIndex: number): string[] => {
      if (separatorIndex >= separators.length) {
        return [text];
      }

      const separator = separators[separatorIndex];
      const splits = separator ? text.split(separator) : [text];
      const result: string[] = [];

      for (const split of splits) {
        if (split.length <= chunkSize) {
          result.push(split);
        } else {
          result.push(...splitText(split, separatorIndex + 1));
        }
      }

      return result;
    };

    const splitParts = splitText(content, 0);
    const chunks: DocumentChunk[] = [];
    let currentChunk = '';
    let startOffset = 0;
    let chunkIndex = 0;
    let currentOffset = 0;

    for (const part of splitParts) {
      if (currentChunk.length + part.length > chunkSize && currentChunk.length > 0) {
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: currentChunk.trim(),
          chunkIndex,
          startOffset,
          endOffset: currentOffset,
          metadata: {},
          createdAt: new Date(),
        });

        // Handle overlap
        const words = currentChunk.split(' ');
        const overlapWords = words.slice(-Math.ceil(overlap / 5));
        startOffset = currentOffset - overlapWords.join(' ').length;
        currentChunk = overlapWords.join(' ') + ' ';
        chunkIndex++;
      }

      currentChunk += part + ' ';
      currentOffset += part.length + 1;
    }

    if (currentChunk.trim().length > 0) {
      chunks.push({
        id: uuidv4(),
        documentId: document.id,
        collectionId: document.collectionId,
        content: currentChunk.trim(),
        chunkIndex,
        startOffset,
        endOffset: content.length,
        metadata: {},
        createdAt: new Date(),
      });
    }

    return chunks;
  }

  /**
   * Semantic chunking using embeddings
   */
  private async semanticChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): Promise<DocumentChunk[]> {
    // First do sentence chunking to get initial segments
    const sentences = content.match(/[^.!?]+[.!?]+/g) || [content];

    if (sentences.length <= 1) {
      return this.fixedSizeChunking(document, content, config);
    }

    // For now, fall back to recursive chunking
    // In production, would compute embeddings for each sentence
    // and merge based on semantic similarity
    return this.recursiveChunking(document, content, config);
  }

  /**
   * Code-aware chunking
   */
  private codeAwareChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = [];
    const chunkSize = config.chunkSize || 1024;

    // Split by common code boundaries
    const codePatterns = [
      /(?:^|\n)(function\s+\w+|const\s+\w+\s*=\s*(?:async\s+)?function|class\s+\w+)/gm,
      /(?:^|\n)(def\s+\w+|class\s+\w+)/gm, // Python
      /(?:^|\n)(public\s+(?:class|interface|enum)\s+\w+)/gm, // Java/C#
    ];

    let boundaries: number[] = [0];

    for (const pattern of codePatterns) {
      let match;
      while ((match = pattern.exec(content)) !== null) {
        boundaries.push(match.index);
      }
    }

    boundaries.push(content.length);
    boundaries = [...new Set(boundaries)].sort((a, b) => a - b);

    let chunkIndex = 0;

    for (let i = 0; i < boundaries.length - 1; i++) {
      const startOffset = boundaries[i];
      let endOffset = boundaries[i + 1];

      // If segment is too large, split it
      while (endOffset - startOffset > chunkSize) {
        const chunkEnd = startOffset + chunkSize;
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: content.slice(startOffset, chunkEnd).trim(),
          chunkIndex: chunkIndex++,
          startOffset,
          endOffset: chunkEnd,
          metadata: { type: 'code' },
          createdAt: new Date(),
        });
        endOffset = chunkEnd;
      }

      const chunkContent = content.slice(startOffset, endOffset).trim();
      if (chunkContent.length > 0) {
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: chunkContent,
          chunkIndex: chunkIndex++,
          startOffset,
          endOffset,
          metadata: { type: 'code' },
          createdAt: new Date(),
        });
      }
    }

    return chunks;
  }

  /**
   * Markdown header chunking
   */
  private markdownHeaderChunking(
    document: Document,
    content: string,
    config: Collection['chunkingConfig']
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = [];
    const chunkSize = config.chunkSize || 1024;

    // Split by markdown headers
    const headerPattern = /^(#{1,6})\s+(.+)$/gm;
    const sections: Array<{ level: number; title: string; content: string; startOffset: number }> = [];
    let lastIndex = 0;
    let match;

    while ((match = headerPattern.exec(content)) !== null) {
      if (sections.length > 0) {
        sections[sections.length - 1].content = content.slice(
          sections[sections.length - 1].startOffset,
          match.index
        );
      }

      sections.push({
        level: match[1].length,
        title: match[2],
        content: '',
        startOffset: match.index,
      });
      lastIndex = match.index;
    }

    // Handle last section
    if (sections.length > 0) {
      sections[sections.length - 1].content = content.slice(
        sections[sections.length - 1].startOffset
      );
    } else {
      // No headers found, treat as single section
      sections.push({
        level: 0,
        title: document.title || 'Document',
        content: content,
        startOffset: 0,
      });
    }

    let chunkIndex = 0;

    for (const section of sections) {
      if (section.content.length <= chunkSize) {
        chunks.push({
          id: uuidv4(),
          documentId: document.id,
          collectionId: document.collectionId,
          content: section.content.trim(),
          chunkIndex: chunkIndex++,
          startOffset: section.startOffset,
          endOffset: section.startOffset + section.content.length,
          metadata: {
            headerLevel: section.level,
            headerTitle: section.title,
          },
          createdAt: new Date(),
        });
      } else {
        // Split large sections using recursive chunking
        const subChunks = this.recursiveChunking(
          { ...document, content: section.content },
          section.content,
          config
        );

        for (const subChunk of subChunks) {
          chunks.push({
            ...subChunk,
            chunkIndex: chunkIndex++,
            startOffset: section.startOffset + subChunk.startOffset,
            endOffset: section.startOffset + subChunk.endOffset,
            metadata: {
              ...subChunk.metadata,
              headerLevel: section.level,
              headerTitle: section.title,
            },
          });
        }
      }
    }

    return chunks;
  }

  /**
   * Generate embeddings for texts
   */
  private async generateEmbeddings(
    texts: string[],
    config: Collection['embeddingConfig']
  ): Promise<number[][]> {
    const provider = this.embeddingProviders.get(config.provider);
    if (!provider) {
      throw new Error(`Embedding provider ${config.provider} not configured`);
    }

    // Batch embeddings (OpenAI has a limit)
    const batchSize = 100;
    const embeddings: number[][] = [];

    for (let i = 0; i < texts.length; i += batchSize) {
      const batch = texts.slice(i, i + batchSize);
      const batchEmbeddings = await provider.embed(batch);
      embeddings.push(...batchEmbeddings);
    }

    return embeddings;
  }

  /**
   * Update document status
   */
  private async updateDocumentStatus(
    documentId: string,
    status: Document['status'],
    chunkCount?: number
  ): Promise<void> {
    await this.db.query(
      `UPDATE rag_documents SET
        status = $1,
        chunk_count = COALESCE($2, chunk_count),
        processed_at = CASE WHEN $1 = 'ready' THEN NOW() ELSE processed_at END,
        updated_at = NOW()
      WHERE id = $3`,
      [status, chunkCount, documentId]
    );
  }

  // ===========================================
  // Retrieval
  // ===========================================

  /**
   * Retrieve relevant chunks for a query
   */
  async retrieve(
    collectionId: string,
    query: string,
    config?: Partial<RetrievalConfig>
  ): Promise<RetrievalResult[]> {
    const collection = await this.getCollection(collectionId);
    if (!collection) throw new Error('Collection not found');

    const retrievalConfig: RetrievalConfig = {
      strategy: RetrievalStrategy.VECTOR_SIMILARITY,
      topK: 5,
      minScore: 0.7,
      ...config,
    };

    // Check cache
    const cacheKey = `rag:${collectionId}:${crypto.createHash('md5').update(query).digest('hex')}`;
    const cached = await this.redis.get(cacheKey);
    if (cached) {
      return JSON.parse(cached);
    }

    let results: RetrievalResult[];

    switch (retrievalConfig.strategy) {
      case RetrievalStrategy.VECTOR_SIMILARITY:
        results = await this.vectorSimilarityRetrieval(collection, query, retrievalConfig);
        break;
      case RetrievalStrategy.KEYWORD:
        results = await this.keywordRetrieval(collection, query, retrievalConfig);
        break;
      case RetrievalStrategy.HYBRID:
        results = await this.hybridRetrieval(collection, query, retrievalConfig);
        break;
      case RetrievalStrategy.RERANKING:
        results = await this.rerankingRetrieval(collection, query, retrievalConfig);
        break;
      case RetrievalStrategy.MULTI_QUERY:
        results = await this.multiQueryRetrieval(collection, query, retrievalConfig);
        break;
      default:
        results = await this.vectorSimilarityRetrieval(collection, query, retrievalConfig);
    }

    // Cache results
    await this.redis.setEx(cacheKey, 300, JSON.stringify(results)); // 5 minute cache

    return results;
  }

  /**
   * Vector similarity retrieval
   */
  private async vectorSimilarityRetrieval(
    collection: Collection,
    query: string,
    config: RetrievalConfig
  ): Promise<RetrievalResult[]> {
    // Generate query embedding
    const [queryEmbedding] = await this.generateEmbeddings([query], collection.embeddingConfig);

    // Query vector store
    const vectorStore = this.vectorStores.get(collection.id);
    if (!vectorStore) {
      await this.initializeVectorStore(collection);
    }

    const store = this.vectorStores.get(collection.id)!;
    const vectorResults = await store.query(
      queryEmbedding,
      config.topK || 5,
      config.filter
    );

    // Fetch chunk content
    const results: RetrievalResult[] = [];

    for (const result of vectorResults) {
      if (result.score < (config.minScore || 0)) continue;

      const chunkResult = await this.db.query(
        `SELECT dc.*, d.title as document_title, d.source as document_source
         FROM rag_document_chunks dc
         JOIN rag_documents d ON dc.document_id = d.id
         WHERE dc.id = $1`,
        [result.id]
      );

      if (chunkResult.rows.length > 0) {
        const chunk = chunkResult.rows[0];
        results.push({
          chunkId: chunk.id,
          documentId: chunk.document_id,
          content: chunk.content,
          score: result.score,
          metadata: {
            ...chunk.metadata,
            documentTitle: chunk.document_title,
            documentSource: chunk.document_source,
            chunkIndex: chunk.chunk_index,
          },
        });
      }
    }

    return results;
  }

  /**
   * Keyword-based retrieval using full-text search
   */
  private async keywordRetrieval(
    collection: Collection,
    query: string,
    config: RetrievalConfig
  ): Promise<RetrievalResult[]> {
    const result = await this.db.query(
      `SELECT dc.*, d.title as document_title, d.source as document_source,
              ts_rank(to_tsvector('english', dc.content), plainto_tsquery('english', $1)) as score
       FROM rag_document_chunks dc
       JOIN rag_documents d ON dc.document_id = d.id
       WHERE dc.collection_id = $2
         AND to_tsvector('english', dc.content) @@ plainto_tsquery('english', $1)
       ORDER BY score DESC
       LIMIT $3`,
      [query, collection.id, config.topK || 5]
    );

    return result.rows.map(row => ({
      chunkId: row.id,
      documentId: row.document_id,
      content: row.content,
      score: row.score,
      metadata: {
        ...row.metadata,
        documentTitle: row.document_title,
        documentSource: row.document_source,
        chunkIndex: row.chunk_index,
      },
    }));
  }

  /**
   * Hybrid retrieval combining vector and keyword search
   */
  private async hybridRetrieval(
    collection: Collection,
    query: string,
    config: RetrievalConfig
  ): Promise<RetrievalResult[]> {
    const hybridConfig = config.strategyConfig?.hybrid || {
      vectorWeight: 0.7,
      keywordWeight: 0.3,
    };

    // Get both results
    const [vectorResults, keywordResults] = await Promise.all([
      this.vectorSimilarityRetrieval(collection, query, { ...config, topK: (config.topK || 5) * 2 }),
      this.keywordRetrieval(collection, query, { ...config, topK: (config.topK || 5) * 2 }),
    ]);

    // Combine and normalize scores
    const combined = new Map<string, RetrievalResult & { combinedScore: number }>();

    for (const result of vectorResults) {
      combined.set(result.chunkId, {
        ...result,
        combinedScore: result.score * hybridConfig.vectorWeight,
      });
    }

    for (const result of keywordResults) {
      const existing = combined.get(result.chunkId);
      if (existing) {
        existing.combinedScore += result.score * hybridConfig.keywordWeight;
      } else {
        combined.set(result.chunkId, {
          ...result,
          combinedScore: result.score * hybridConfig.keywordWeight,
        });
      }
    }

    // Sort by combined score and take top K
    return Array.from(combined.values())
      .sort((a, b) => b.combinedScore - a.combinedScore)
      .slice(0, config.topK || 5)
      .map(r => ({ ...r, score: r.combinedScore }));
  }

  /**
   * Retrieval with reranking
   */
  private async rerankingRetrieval(
    collection: Collection,
    query: string,
    config: RetrievalConfig
  ): Promise<RetrievalResult[]> {
    // First pass: get more candidates than needed
    const candidates = await this.vectorSimilarityRetrieval(
      collection,
      query,
      { ...config, topK: (config.topK || 5) * 3, minScore: 0 }
    );

    // Reranking would use a cross-encoder model in production
    // For now, we'll use a simple heuristic based on query term overlap
    const queryTerms = query.toLowerCase().split(/\s+/);

    const reranked = candidates.map(result => {
      const contentLower = result.content.toLowerCase();
      let termScore = 0;

      for (const term of queryTerms) {
        if (contentLower.includes(term)) {
          termScore += 1;
        }
      }

      const normalizedTermScore = termScore / queryTerms.length;
      const combinedScore = result.score * 0.7 + normalizedTermScore * 0.3;

      return { ...result, score: combinedScore };
    });

    return reranked
      .sort((a, b) => b.score - a.score)
      .slice(0, config.topK || 5);
  }

  /**
   * Multi-query retrieval
   */
  private async multiQueryRetrieval(
    collection: Collection,
    query: string,
    config: RetrievalConfig
  ): Promise<RetrievalResult[]> {
    // Generate query variations
    // In production, would use LLM to generate variations
    const queries = [
      query,
      query.split(' ').reverse().join(' '), // Simple variation
      query.replace(/\?/g, ''), // Remove question marks
    ];

    // Retrieve for each query
    const allResults: RetrievalResult[] = [];

    for (const q of queries) {
      const results = await this.vectorSimilarityRetrieval(
        collection,
        q,
        { ...config, topK: Math.ceil((config.topK || 5) / queries.length) }
      );
      allResults.push(...results);
    }

    // Deduplicate and combine scores
    const combined = new Map<string, RetrievalResult>();

    for (const result of allResults) {
      const existing = combined.get(result.chunkId);
      if (!existing || result.score > existing.score) {
        combined.set(result.chunkId, result);
      }
    }

    return Array.from(combined.values())
      .sort((a, b) => b.score - a.score)
      .slice(0, config.topK || 5);
  }

  // ===========================================
  // RAG Pipeline
  // ===========================================

  /**
   * Create a RAG pipeline
   */
  async createPipeline(input: CreateRAGPipelineInput, userId: string): Promise<RAGPipeline> {
    const pipeline: RAGPipeline = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      collectionIds: input.collectionIds,
      retrievalConfig: {
        strategy: RetrievalStrategy.HYBRID,
        topK: 5,
        minScore: 0.7,
        ...input.retrievalConfig,
      },
      generationConfig: {
        modelId: input.generationConfig?.modelId || 'gpt-4',
        systemPrompt: input.generationConfig?.systemPrompt || 'You are a helpful assistant. Use the provided context to answer questions accurately.',
        temperature: input.generationConfig?.temperature || 0.7,
        maxTokens: input.generationConfig?.maxTokens || 1000,
        ...input.generationConfig,
      },
      attribution: input.attribution || { enabled: true, style: 'inline' },
      guardrails: input.guardrails || {},
      caching: input.caching || { enabled: true, ttl: 3600 },
      status: 'active',
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO rag_pipelines (
        id, name, description, collection_ids, retrieval_config,
        generation_config, attribution, guardrails, caching, status,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
      [
        pipeline.id, pipeline.name, pipeline.description, pipeline.collectionIds,
        JSON.stringify(pipeline.retrievalConfig), JSON.stringify(pipeline.generationConfig),
        JSON.stringify(pipeline.attribution), JSON.stringify(pipeline.guardrails),
        JSON.stringify(pipeline.caching), pipeline.status, pipeline.createdAt,
        pipeline.updatedAt, pipeline.createdBy,
      ]
    );

    return pipeline;
  }

  /**
   * Execute RAG pipeline
   */
  async executePipeline(
    pipelineId: string,
    query: string,
    context?: Record<string, unknown>
  ): Promise<{
    answer: string;
    sources: RetrievalResult[];
    metadata: Record<string, unknown>;
  }> {
    const pipeline = await this.getPipeline(pipelineId);
    if (!pipeline) throw new Error('Pipeline not found');

    // Retrieve from all collections
    const allResults: RetrievalResult[] = [];

    for (const collectionId of pipeline.collectionIds) {
      const results = await this.retrieve(collectionId, query, pipeline.retrievalConfig);
      allResults.push(...results);
    }

    // Sort by score and take top K
    const topResults = allResults
      .sort((a, b) => b.score - a.score)
      .slice(0, pipeline.retrievalConfig.topK || 5);

    // Build context for generation
    const contextText = topResults
      .map((r, i) => `[${i + 1}] ${r.content}`)
      .join('\n\n');

    // Generate response
    if (!this.openai) {
      throw new Error('OpenAI client not configured for generation');
    }

    const response = await this.openai.chat.completions.create({
      model: pipeline.generationConfig.modelId,
      messages: [
        {
          role: 'system',
          content: `${pipeline.generationConfig.systemPrompt}\n\nContext:\n${contextText}`,
        },
        {
          role: 'user',
          content: query,
        },
      ],
      temperature: pipeline.generationConfig.temperature,
      max_tokens: pipeline.generationConfig.maxTokens,
    });

    let answer = response.choices[0]?.message?.content || '';

    // Add attribution if enabled
    if (pipeline.attribution.enabled) {
      const sources = topResults.map((r, i) =>
        `[${i + 1}] ${r.metadata?.documentTitle || 'Unknown'}`
      ).join(', ');

      if (pipeline.attribution.style === 'footnote') {
        answer += `\n\nSources: ${sources}`;
      }
    }

    return {
      answer,
      sources: topResults,
      metadata: {
        model: pipeline.generationConfig.modelId,
        tokensUsed: response.usage?.total_tokens,
        retrievalCount: topResults.length,
      },
    };
  }

  /**
   * Get pipeline by ID
   */
  async getPipeline(pipelineId: string): Promise<RAGPipeline | null> {
    const result = await this.db.query(
      `SELECT * FROM rag_pipelines WHERE id = $1`,
      [pipelineId]
    );

    if (result.rows.length === 0) return null;

    return this.mapPipelineRow(result.rows[0]);
  }

  /**
   * Get document by ID
   */
  async getDocument(documentId: string): Promise<Document | null> {
    const result = await this.db.query(
      `SELECT * FROM rag_documents WHERE id = $1`,
      [documentId]
    );

    if (result.rows.length === 0) return null;

    return this.mapDocumentRow(result.rows[0]);
  }

  /**
   * Delete document and its chunks
   */
  async deleteDocument(documentId: string): Promise<void> {
    const document = await this.getDocument(documentId);
    if (!document) throw new Error('Document not found');

    // Get chunk IDs
    const chunksResult = await this.db.query(
      `SELECT id FROM rag_document_chunks WHERE document_id = $1`,
      [documentId]
    );

    const chunkIds = chunksResult.rows.map(r => r.id);

    // Delete from vector store
    const vectorStore = this.vectorStores.get(document.collectionId);
    if (vectorStore && chunkIds.length > 0) {
      await vectorStore.delete(chunkIds);
    }

    // Delete chunks
    await this.db.query(
      `DELETE FROM rag_document_chunks WHERE document_id = $1`,
      [documentId]
    );

    // Delete document
    await this.db.query(
      `DELETE FROM rag_documents WHERE id = $1`,
      [documentId]
    );

    // Update collection stats
    await this.db.query(
      `UPDATE rag_collections SET
        document_count = document_count - 1,
        total_chunks = total_chunks - $1,
        updated_at = NOW()
      WHERE id = $2`,
      [chunkIds.length, document.collectionId]
    );
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapCollectionRow(row: Record<string, unknown>): Collection {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      embeddingConfig: row.embedding_config as Collection['embeddingConfig'],
      vectorStoreConfig: row.vector_store_config as Collection['vectorStoreConfig'],
      chunkingConfig: row.chunking_config as Collection['chunkingConfig'],
      metadata: row.metadata as Record<string, unknown>,
      documentCount: row.document_count as number,
      totalChunks: row.total_chunks as number,
      status: row.status as Collection['status'],
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapDocumentRow(row: Record<string, unknown>): Document {
    return {
      id: row.id as string,
      collectionId: row.collection_id as string,
      title: row.title as string,
      content: row.content as string,
      contentType: row.content_type as string,
      source: row.source as Document['source'],
      metadata: row.metadata as Record<string, unknown>,
      contentHash: row.content_hash as string,
      chunkCount: row.chunk_count as number | undefined,
      status: row.status as Document['status'],
      processedAt: row.processed_at as Date | undefined,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapPipelineRow(row: Record<string, unknown>): RAGPipeline {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      collectionIds: row.collection_ids as string[],
      retrievalConfig: row.retrieval_config as RAGPipeline['retrievalConfig'],
      generationConfig: row.generation_config as RAGPipeline['generationConfig'],
      attribution: row.attribution as RAGPipeline['attribution'],
      guardrails: row.guardrails as RAGPipeline['guardrails'],
      caching: row.caching as RAGPipeline['caching'],
      status: row.status as RAGPipeline['status'],
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }
}
