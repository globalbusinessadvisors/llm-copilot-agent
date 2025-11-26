/**
 * Article Service
 *
 * Manages knowledge base articles for self-service support.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  Article,
  ArticleStatus,
  ArticleCategory,
  ArticleFeedback,
  CreateArticleInput,
  UpdateArticleInput,
  CATEGORY_INFO,
} from '../models/article';

export interface ArticleSearchResult {
  articles: Article[];
  total: number;
  page: number;
  pageSize: number;
}

export interface ArticleStats {
  total: number;
  byCategory: Record<ArticleCategory, number>;
  byStatus: Record<ArticleStatus, number>;
  mostViewed: Array<{ id: string; title: string; views: number }>;
  mostHelpful: Array<{ id: string; title: string; helpfulCount: number }>;
}

export class ArticleService {
  private db: Pool;
  private redis: RedisClientType;
  private cachePrefix = 'kb:article:';
  private cacheTTL = 3600; // 1 hour

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  /**
   * Create a new article
   */
  async createArticle(input: CreateArticleInput, authorId: string): Promise<Article> {
    const slug = input.slug || this.generateSlug(input.title);

    const article: Article = {
      id: uuidv4(),
      title: input.title,
      slug,
      content: input.content,
      excerpt: input.excerpt || this.generateExcerpt(input.content),
      category: input.category,
      tags: input.tags || [],
      status: ArticleStatus.DRAFT,
      authorId,
      views: 0,
      helpfulCount: 0,
      notHelpfulCount: 0,
      relatedArticles: input.relatedArticles || [],
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO articles (
        id, title, slug, content, excerpt, category, tags, status,
        author_id, views, helpful_count, not_helpful_count, related_articles,
        created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        article.id,
        article.title,
        article.slug,
        article.content,
        article.excerpt,
        article.category,
        JSON.stringify(article.tags),
        article.status,
        article.authorId,
        article.views,
        article.helpfulCount,
        article.notHelpfulCount,
        JSON.stringify(article.relatedArticles),
        article.createdAt,
        article.updatedAt,
      ]
    );

    return article;
  }

  /**
   * Get article by ID
   */
  async getArticle(articleId: string): Promise<Article | null> {
    // Try cache first
    const cached = await this.redis.get(`${this.cachePrefix}${articleId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    const result = await this.db.query(
      `SELECT * FROM articles WHERE id = $1`,
      [articleId]
    );

    if (result.rows.length === 0) return null;

    const article = this.mapArticleRow(result.rows[0]);

    // Cache the article
    await this.redis.set(
      `${this.cachePrefix}${articleId}`,
      JSON.stringify(article),
      { EX: this.cacheTTL }
    );

    return article;
  }

  /**
   * Get article by slug
   */
  async getArticleBySlug(slug: string): Promise<Article | null> {
    const result = await this.db.query(
      `SELECT * FROM articles WHERE slug = $1`,
      [slug]
    );

    if (result.rows.length === 0) return null;

    return this.mapArticleRow(result.rows[0]);
  }

  /**
   * Update an article
   */
  async updateArticle(articleId: string, input: UpdateArticleInput): Promise<Article | null> {
    const article = await this.getArticle(articleId);
    if (!article) return null;

    const updates: string[] = [];
    const values: unknown[] = [];
    let paramIndex = 1;

    if (input.title !== undefined) {
      updates.push(`title = $${paramIndex++}`);
      values.push(input.title);
    }
    if (input.content !== undefined) {
      updates.push(`content = $${paramIndex++}`);
      values.push(input.content);
      // Update excerpt if content changes
      updates.push(`excerpt = $${paramIndex++}`);
      values.push(input.excerpt || this.generateExcerpt(input.content));
    }
    if (input.category !== undefined) {
      updates.push(`category = $${paramIndex++}`);
      values.push(input.category);
    }
    if (input.tags !== undefined) {
      updates.push(`tags = $${paramIndex++}`);
      values.push(JSON.stringify(input.tags));
    }
    if (input.status !== undefined) {
      updates.push(`status = $${paramIndex++}`);
      values.push(input.status);

      if (input.status === ArticleStatus.PUBLISHED && !article.publishedAt) {
        updates.push(`published_at = NOW()`);
      }
    }
    if (input.relatedArticles !== undefined) {
      updates.push(`related_articles = $${paramIndex++}`);
      values.push(JSON.stringify(input.relatedArticles));
    }

    if (updates.length === 0) return article;

    updates.push('updated_at = NOW()');
    values.push(articleId);

    await this.db.query(
      `UPDATE articles SET ${updates.join(', ')} WHERE id = $${paramIndex}`,
      values
    );

    // Invalidate cache
    await this.redis.del(`${this.cachePrefix}${articleId}`);

    return this.getArticle(articleId);
  }

  /**
   * Publish an article
   */
  async publishArticle(articleId: string): Promise<Article | null> {
    return this.updateArticle(articleId, { status: ArticleStatus.PUBLISHED });
  }

  /**
   * Archive an article
   */
  async archiveArticle(articleId: string): Promise<Article | null> {
    return this.updateArticle(articleId, { status: ArticleStatus.ARCHIVED });
  }

  /**
   * Delete an article
   */
  async deleteArticle(articleId: string): Promise<boolean> {
    const result = await this.db.query(
      `DELETE FROM articles WHERE id = $1 RETURNING id`,
      [articleId]
    );

    if (result.rows.length > 0) {
      await this.redis.del(`${this.cachePrefix}${articleId}`);
      return true;
    }

    return false;
  }

  /**
   * Search articles
   */
  async searchArticles(params: {
    query?: string;
    category?: ArticleCategory;
    tags?: string[];
    status?: ArticleStatus;
    page?: number;
    pageSize?: number;
  }): Promise<ArticleSearchResult> {
    const {
      query,
      category,
      tags,
      status = ArticleStatus.PUBLISHED,
      page = 1,
      pageSize = 20,
    } = params;

    const conditions: string[] = ['status = $1'];
    const values: unknown[] = [status];
    let paramIndex = 2;

    if (query) {
      conditions.push(`(
        title ILIKE $${paramIndex} OR
        content ILIKE $${paramIndex} OR
        excerpt ILIKE $${paramIndex}
      )`);
      values.push(`%${query}%`);
      paramIndex++;
    }

    if (category) {
      conditions.push(`category = $${paramIndex++}`);
      values.push(category);
    }

    if (tags && tags.length > 0) {
      conditions.push(`tags ?| $${paramIndex++}`);
      values.push(tags);
    }

    const whereClause = conditions.join(' AND ');
    const offset = (page - 1) * pageSize;

    // Get total count
    const countResult = await this.db.query(
      `SELECT COUNT(*) FROM articles WHERE ${whereClause}`,
      values
    );
    const total = parseInt(countResult.rows[0].count, 10);

    // Get articles
    values.push(pageSize, offset);
    const result = await this.db.query(
      `SELECT * FROM articles
       WHERE ${whereClause}
       ORDER BY
         CASE WHEN $${paramIndex - (tags && tags.length > 0 ? 2 : 1)} IS NOT NULL
           THEN ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $${paramIndex - (tags && tags.length > 0 ? 2 : 1)}))
           ELSE views
         END DESC,
         created_at DESC
       LIMIT $${paramIndex++} OFFSET $${paramIndex}`,
      values
    );

    return {
      articles: result.rows.map(this.mapArticleRow),
      total,
      page,
      pageSize,
    };
  }

  /**
   * Get articles by category
   */
  async getArticlesByCategory(category: ArticleCategory): Promise<Article[]> {
    const result = await this.db.query(
      `SELECT * FROM articles
       WHERE category = $1 AND status = 'published'
       ORDER BY views DESC, created_at DESC`,
      [category]
    );
    return result.rows.map(this.mapArticleRow);
  }

  /**
   * Get popular articles
   */
  async getPopularArticles(limit: number = 10): Promise<Article[]> {
    const result = await this.db.query(
      `SELECT * FROM articles
       WHERE status = 'published'
       ORDER BY views DESC
       LIMIT $1`,
      [limit]
    );
    return result.rows.map(this.mapArticleRow);
  }

  /**
   * Get recent articles
   */
  async getRecentArticles(limit: number = 10): Promise<Article[]> {
    const result = await this.db.query(
      `SELECT * FROM articles
       WHERE status = 'published'
       ORDER BY published_at DESC NULLS LAST, created_at DESC
       LIMIT $1`,
      [limit]
    );
    return result.rows.map(this.mapArticleRow);
  }

  /**
   * Record article view
   */
  async recordView(articleId: string): Promise<void> {
    await this.db.query(
      `UPDATE articles SET views = views + 1 WHERE id = $1`,
      [articleId]
    );

    // Invalidate cache
    await this.redis.del(`${this.cachePrefix}${articleId}`);
  }

  /**
   * Submit article feedback
   */
  async submitFeedback(articleId: string, helpful: boolean, comment?: string): Promise<void> {
    const feedback: ArticleFeedback = {
      id: uuidv4(),
      articleId,
      helpful,
      comment,
      createdAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO article_feedback (id, article_id, helpful, comment, created_at)
       VALUES ($1, $2, $3, $4, $5)`,
      [feedback.id, feedback.articleId, feedback.helpful, feedback.comment, feedback.createdAt]
    );

    // Update article counts
    const field = helpful ? 'helpful_count' : 'not_helpful_count';
    await this.db.query(
      `UPDATE articles SET ${field} = ${field} + 1 WHERE id = $1`,
      [articleId]
    );

    // Invalidate cache
    await this.redis.del(`${this.cachePrefix}${articleId}`);
  }

  /**
   * Get article feedback
   */
  async getArticleFeedback(articleId: string): Promise<ArticleFeedback[]> {
    const result = await this.db.query(
      `SELECT * FROM article_feedback WHERE article_id = $1 ORDER BY created_at DESC`,
      [articleId]
    );

    return result.rows.map(row => ({
      id: row.id,
      articleId: row.article_id,
      helpful: row.helpful,
      comment: row.comment,
      createdAt: row.created_at,
    }));
  }

  /**
   * Get all categories with article counts
   */
  async getCategories(): Promise<Array<{
    category: ArticleCategory;
    name: string;
    description: string;
    icon: string;
    articleCount: number;
  }>> {
    const result = await this.db.query(`
      SELECT category, COUNT(*) as count
      FROM articles
      WHERE status = 'published'
      GROUP BY category
    `);

    const countMap: Record<string, number> = {};
    for (const row of result.rows) {
      countMap[row.category] = parseInt(row.count, 10);
    }

    return Object.values(ArticleCategory).map(category => ({
      category,
      name: CATEGORY_INFO[category].name,
      description: CATEGORY_INFO[category].description,
      icon: CATEGORY_INFO[category].icon,
      articleCount: countMap[category] || 0,
    }));
  }

  /**
   * Get article statistics
   */
  async getStats(): Promise<ArticleStats> {
    const result = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE status = 'draft') as draft_count,
        COUNT(*) FILTER (WHERE status = 'published') as published_count,
        COUNT(*) FILTER (WHERE status = 'archived') as archived_count
      FROM articles
    `);

    const categoryResult = await this.db.query(`
      SELECT category, COUNT(*) as count
      FROM articles
      GROUP BY category
    `);

    const mostViewedResult = await this.db.query(`
      SELECT id, title, views
      FROM articles
      WHERE status = 'published'
      ORDER BY views DESC
      LIMIT 10
    `);

    const mostHelpfulResult = await this.db.query(`
      SELECT id, title, helpful_count
      FROM articles
      WHERE status = 'published'
      ORDER BY helpful_count DESC
      LIMIT 10
    `);

    const row = result.rows[0];
    const categoryMap: Record<string, number> = {};
    for (const catRow of categoryResult.rows) {
      categoryMap[catRow.category] = parseInt(catRow.count, 10);
    }

    return {
      total: parseInt(row.total, 10),
      byStatus: {
        [ArticleStatus.DRAFT]: parseInt(row.draft_count, 10),
        [ArticleStatus.PUBLISHED]: parseInt(row.published_count, 10),
        [ArticleStatus.ARCHIVED]: parseInt(row.archived_count, 10),
      },
      byCategory: Object.values(ArticleCategory).reduce((acc, cat) => {
        acc[cat] = categoryMap[cat] || 0;
        return acc;
      }, {} as Record<ArticleCategory, number>),
      mostViewed: mostViewedResult.rows.map(r => ({
        id: r.id,
        title: r.title,
        views: r.views,
      })),
      mostHelpful: mostHelpfulResult.rows.map(r => ({
        id: r.id,
        title: r.title,
        helpfulCount: r.helpful_count,
      })),
    };
  }

  // ===========================================
  // Private Helpers
  // ===========================================

  private generateSlug(title: string): string {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .substring(0, 100);
  }

  private generateExcerpt(content: string, maxLength: number = 200): string {
    // Strip HTML tags and truncate
    const plainText = content.replace(/<[^>]*>/g, '').trim();
    if (plainText.length <= maxLength) return plainText;
    return plainText.substring(0, maxLength).trim() + '...';
  }

  private mapArticleRow(row: any): Article {
    return {
      id: row.id,
      title: row.title,
      slug: row.slug,
      content: row.content,
      excerpt: row.excerpt,
      category: row.category,
      tags: row.tags || [],
      status: row.status,
      authorId: row.author_id,
      views: row.views,
      helpfulCount: row.helpful_count,
      notHelpfulCount: row.not_helpful_count,
      relatedArticles: row.related_articles || [],
      publishedAt: row.published_at,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
