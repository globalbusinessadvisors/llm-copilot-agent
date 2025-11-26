/**
 * Article Routes
 *
 * REST API endpoints for knowledge base article management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { ArticleService } from '../services/articleService';
import { ArticleCategory, ArticleStatus } from '../models/article';

const router = Router();

// Request validation schemas
const CreateArticleSchema = z.object({
  title: z.string().min(1).max(255),
  slug: z.string().max(100).optional(),
  content: z.string().min(1),
  excerpt: z.string().max(500).optional(),
  category: z.nativeEnum(ArticleCategory),
  tags: z.array(z.string()).optional(),
  relatedArticles: z.array(z.string().uuid()).optional(),
});

const UpdateArticleSchema = CreateArticleSchema.partial().extend({
  status: z.nativeEnum(ArticleStatus).optional(),
});

const SearchArticlesSchema = z.object({
  query: z.string().optional(),
  category: z.nativeEnum(ArticleCategory).optional(),
  tags: z.string().optional(), // Comma-separated
  status: z.nativeEnum(ArticleStatus).optional(),
  page: z.coerce.number().min(1).default(1),
  pageSize: z.coerce.number().min(1).max(100).default(20),
});

const FeedbackSchema = z.object({
  helpful: z.boolean(),
  comment: z.string().max(1000).optional(),
});

export function createArticleRoutes(articleService: ArticleService): Router {
  /**
   * Create a new article
   * POST /api/v1/articles
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateArticleSchema.parse(req.body);
      const authorId = (req as any).user?.id || req.body.authorId || 'system';

      const article = await articleService.createArticle(input, authorId);

      res.status(201).json({
        success: true,
        data: article,
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Get article by ID
   * GET /api/v1/articles/:articleId
   */
  router.get('/:articleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const article = await articleService.getArticle(articleId);

      if (!article) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      // Record view for published articles
      if (article.status === ArticleStatus.PUBLISHED) {
        await articleService.recordView(articleId);
      }

      res.json({
        success: true,
        data: article,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get article by slug
   * GET /api/v1/articles/slug/:slug
   */
  router.get('/slug/:slug', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { slug } = req.params;
      const article = await articleService.getArticleBySlug(slug);

      if (!article) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      // Record view for published articles
      if (article.status === ArticleStatus.PUBLISHED) {
        await articleService.recordView(article.id);
      }

      res.json({
        success: true,
        data: article,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Search articles
   * GET /api/v1/articles
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const params = SearchArticlesSchema.parse(req.query);
      const tags = params.tags?.split(',').filter(Boolean);

      const result = await articleService.searchArticles({
        ...params,
        tags,
      });

      res.json({
        success: true,
        data: result.articles,
        meta: {
          total: result.total,
          page: result.page,
          pageSize: result.pageSize,
          totalPages: Math.ceil(result.total / result.pageSize),
        },
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Update article
   * PUT /api/v1/articles/:articleId
   */
  router.put('/:articleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const updates = UpdateArticleSchema.parse(req.body);

      const article = await articleService.updateArticle(articleId, updates);

      if (!article) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      res.json({
        success: true,
        data: article,
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Publish article
   * POST /api/v1/articles/:articleId/publish
   */
  router.post('/:articleId/publish', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const article = await articleService.publishArticle(articleId);

      if (!article) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      res.json({
        success: true,
        data: article,
        message: 'Article published successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Archive article
   * POST /api/v1/articles/:articleId/archive
   */
  router.post('/:articleId/archive', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const article = await articleService.archiveArticle(articleId);

      if (!article) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      res.json({
        success: true,
        data: article,
        message: 'Article archived successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete article
   * DELETE /api/v1/articles/:articleId
   */
  router.delete('/:articleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const deleted = await articleService.deleteArticle(articleId);

      if (!deleted) {
        res.status(404).json({
          success: false,
          error: 'Article not found',
        });
        return;
      }

      res.json({
        success: true,
        message: 'Article deleted successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Submit article feedback
   * POST /api/v1/articles/:articleId/feedback
   */
  router.post('/:articleId/feedback', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const input = FeedbackSchema.parse(req.body);

      await articleService.submitFeedback(articleId, input.helpful, input.comment);

      res.json({
        success: true,
        message: 'Thank you for your feedback',
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Get article feedback
   * GET /api/v1/articles/:articleId/feedback
   */
  router.get('/:articleId/feedback', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { articleId } = req.params;
      const feedback = await articleService.getArticleFeedback(articleId);

      res.json({
        success: true,
        data: feedback,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get all categories
   * GET /api/v1/articles/meta/categories
   */
  router.get('/meta/categories', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const categories = await articleService.getCategories();

      res.json({
        success: true,
        data: categories,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get popular articles
   * GET /api/v1/articles/featured/popular
   */
  router.get('/featured/popular', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const limit = parseInt(req.query.limit as string, 10) || 10;
      const articles = await articleService.getPopularArticles(limit);

      res.json({
        success: true,
        data: articles,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get recent articles
   * GET /api/v1/articles/featured/recent
   */
  router.get('/featured/recent', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const limit = parseInt(req.query.limit as string, 10) || 10;
      const articles = await articleService.getRecentArticles(limit);

      res.json({
        success: true,
        data: articles,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get articles by category
   * GET /api/v1/articles/category/:category
   */
  router.get('/category/:category', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { category } = req.params;

      if (!Object.values(ArticleCategory).includes(category as ArticleCategory)) {
        res.status(400).json({
          success: false,
          error: 'Invalid category',
        });
        return;
      }

      const articles = await articleService.getArticlesByCategory(category as ArticleCategory);

      res.json({
        success: true,
        data: articles,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get article statistics
   * GET /api/v1/articles/stats/summary
   */
  router.get('/stats/summary', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const stats = await articleService.getStats();

      res.json({
        success: true,
        data: stats,
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
