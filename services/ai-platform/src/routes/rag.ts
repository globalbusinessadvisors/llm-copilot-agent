/**
 * RAG Routes
 *
 * API endpoints for document management, collections, and retrieval augmented generation.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { RAGService } from '../services/ragService';
import { RetrievalStrategy } from '../models/rag';

export function createRAGRoutes(ragService: RAGService): Router {
  const router = Router();

  // ===========================================
  // Collection Routes
  // ===========================================

  /**
   * List collections
   */
  router.get('/collections', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status } = req.query;
      const collections = await ragService.listCollections({
        status: status as any,
      });
      res.json({ collections });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get collection by ID
   */
  router.get('/collections/:collectionId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const collection = await ragService.getCollection(req.params.collectionId);
      if (!collection) {
        return res.status(404).json({ error: 'Collection not found' });
      }
      res.json({ collection });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create collection
   */
  router.post('/collections', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const collection = await ragService.createCollection(req.body, userId);
      res.status(201).json({ collection });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Document Routes
  // ===========================================

  /**
   * Ingest document into collection
   */
  router.post('/collections/:collectionId/documents', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const document = await ragService.ingestDocument(
        req.params.collectionId,
        req.body,
        userId
      );
      res.status(201).json({ document });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get document by ID
   */
  router.get('/documents/:documentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const document = await ragService.getDocument(req.params.documentId);
      if (!document) {
        return res.status(404).json({ error: 'Document not found' });
      }
      res.json({ document });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete document
   */
  router.delete('/documents/:documentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await ragService.deleteDocument(req.params.documentId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Batch ingest documents
   */
  router.post('/collections/:collectionId/documents/batch', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const { documents } = req.body;

      if (!Array.isArray(documents)) {
        return res.status(400).json({ error: 'documents must be an array' });
      }

      const results = await Promise.allSettled(
        documents.map(doc =>
          ragService.ingestDocument(req.params.collectionId, doc, userId)
        )
      );

      const ingested = results
        .filter((r): r is PromiseFulfilledResult<any> => r.status === 'fulfilled')
        .map(r => r.value);

      const failed = results
        .filter((r): r is PromiseRejectedResult => r.status === 'rejected')
        .map((r, i) => ({
          index: i,
          error: r.reason?.message || 'Unknown error',
        }));

      res.status(201).json({
        ingested,
        failed,
        total: documents.length,
        successCount: ingested.length,
        failureCount: failed.length,
      });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Retrieval Routes
  // ===========================================

  /**
   * Retrieve from collection
   */
  router.post('/collections/:collectionId/retrieve', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { query, strategy, topK, minScore, filter } = req.body;

      if (!query) {
        return res.status(400).json({ error: 'query is required' });
      }

      if (strategy && !Object.values(RetrievalStrategy).includes(strategy)) {
        return res.status(400).json({ error: 'Invalid retrieval strategy' });
      }

      const results = await ragService.retrieve(
        req.params.collectionId,
        query,
        { strategy, topK, minScore, filter }
      );

      res.json({ results, count: results.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Search across multiple collections
   */
  router.post('/search', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { query, collectionIds, strategy, topK, minScore } = req.body;

      if (!query) {
        return res.status(400).json({ error: 'query is required' });
      }

      if (!collectionIds || !Array.isArray(collectionIds) || collectionIds.length === 0) {
        return res.status(400).json({ error: 'collectionIds is required and must be a non-empty array' });
      }

      const allResults = await Promise.all(
        collectionIds.map(collectionId =>
          ragService.retrieve(collectionId, query, { strategy, topK, minScore })
        )
      );

      // Flatten and sort by score
      const flattenedResults = allResults
        .flat()
        .sort((a, b) => b.score - a.score)
        .slice(0, topK || 10);

      res.json({ results: flattenedResults, count: flattenedResults.length });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Pipeline Routes
  // ===========================================

  /**
   * List pipelines
   */
  router.get('/pipelines', async (req: Request, res: Response, next: NextFunction) => {
    try {
      // For now, return empty list - would need to add listPipelines to service
      res.json({ pipelines: [] });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get pipeline by ID
   */
  router.get('/pipelines/:pipelineId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const pipeline = await ragService.getPipeline(req.params.pipelineId);
      if (!pipeline) {
        return res.status(404).json({ error: 'Pipeline not found' });
      }
      res.json({ pipeline });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create pipeline
   */
  router.post('/pipelines', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const pipeline = await ragService.createPipeline(req.body, userId);
      res.status(201).json({ pipeline });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Execute pipeline (RAG query)
   */
  router.post('/pipelines/:pipelineId/query', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { query, context } = req.body;

      if (!query) {
        return res.status(400).json({ error: 'query is required' });
      }

      const result = await ragService.executePipeline(
        req.params.pipelineId,
        query,
        context
      );

      res.json(result);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Simple RAG query (without creating a pipeline)
   */
  router.post('/query', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const {
        query,
        collectionIds,
        retrievalConfig,
        generationConfig,
      } = req.body;

      if (!query) {
        return res.status(400).json({ error: 'query is required' });
      }

      if (!collectionIds || !Array.isArray(collectionIds) || collectionIds.length === 0) {
        return res.status(400).json({ error: 'collectionIds is required and must be a non-empty array' });
      }

      // Create a temporary pipeline configuration
      const userId = (req as any).user?.id || 'system';
      const pipeline = await ragService.createPipeline(
        {
          name: `temp-${Date.now()}`,
          collectionIds,
          retrievalConfig,
          generationConfig,
        },
        userId
      );

      const result = await ragService.executePipeline(pipeline.id, query);
      res.json(result);
    } catch (error) {
      next(error);
    }
  });

  return router;
}
