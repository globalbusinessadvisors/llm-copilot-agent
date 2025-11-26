/**
 * Knowledge Base Article Models
 *
 * Data models for FAQ and help articles.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum ArticleStatus {
  DRAFT = 'draft',
  PUBLISHED = 'published',
  ARCHIVED = 'archived',
}

export enum ArticleCategory {
  GETTING_STARTED = 'getting_started',
  API_REFERENCE = 'api_reference',
  BILLING = 'billing',
  TROUBLESHOOTING = 'troubleshooting',
  BEST_PRACTICES = 'best_practices',
  FAQ = 'faq',
  RELEASE_NOTES = 'release_notes',
}

// ===========================================
// Schemas
// ===========================================

export const ArticleSchema = z.object({
  id: z.string().uuid(),
  slug: z.string().regex(/^[a-z0-9-]+$/),
  title: z.string().min(1).max(255),
  summary: z.string().max(500).optional(),
  content: z.string().min(1),
  contentHtml: z.string(),
  category: z.nativeEnum(ArticleCategory),
  status: z.nativeEnum(ArticleStatus),
  authorId: z.string().uuid(),
  authorName: z.string(),
  tags: z.array(z.string()).default([]),
  viewCount: z.number().default(0),
  helpfulCount: z.number().default(0),
  notHelpfulCount: z.number().default(0),
  relatedArticles: z.array(z.string().uuid()).default([]),
  metadata: z.record(z.unknown()).default({}),
  createdAt: z.date(),
  updatedAt: z.date(),
  publishedAt: z.date().optional(),
});

export const ArticleFeedbackSchema = z.object({
  id: z.string().uuid(),
  articleId: z.string().uuid(),
  userId: z.string().uuid().optional(),
  sessionId: z.string(),
  helpful: z.boolean(),
  feedback: z.string().optional(),
  createdAt: z.date(),
});

export const ArticleSearchResultSchema = z.object({
  id: z.string().uuid(),
  slug: z.string(),
  title: z.string(),
  summary: z.string().optional(),
  category: z.nativeEnum(ArticleCategory),
  score: z.number(),
  highlights: z.array(z.string()).optional(),
});

// ===========================================
// Types
// ===========================================

export type Article = z.infer<typeof ArticleSchema>;
export type ArticleFeedback = z.infer<typeof ArticleFeedbackSchema>;
export type ArticleSearchResult = z.infer<typeof ArticleSearchResultSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateArticleInput {
  title: string;
  slug?: string;
  summary?: string;
  content: string;
  category: ArticleCategory;
  authorId: string;
  authorName: string;
  tags?: string[];
  relatedArticles?: string[];
  status?: ArticleStatus;
}

export interface UpdateArticleInput {
  title?: string;
  slug?: string;
  summary?: string;
  content?: string;
  category?: ArticleCategory;
  tags?: string[];
  relatedArticles?: string[];
  status?: ArticleStatus;
}

export interface ArticleQueryParams {
  category?: ArticleCategory;
  status?: ArticleStatus;
  search?: string;
  tags?: string[];
  page?: number;
  pageSize?: number;
  sortBy?: 'createdAt' | 'updatedAt' | 'viewCount' | 'helpfulCount';
  sortOrder?: 'asc' | 'desc';
}

// ===========================================
// Category Metadata
// ===========================================

export const CATEGORY_INFO: Record<ArticleCategory, { name: string; description: string; icon: string }> = {
  [ArticleCategory.GETTING_STARTED]: {
    name: 'Getting Started',
    description: 'Quick start guides and tutorials',
    icon: '🚀',
  },
  [ArticleCategory.API_REFERENCE]: {
    name: 'API Reference',
    description: 'API documentation and examples',
    icon: '📚',
  },
  [ArticleCategory.BILLING]: {
    name: 'Billing',
    description: 'Pricing, invoices, and payment',
    icon: '💳',
  },
  [ArticleCategory.TROUBLESHOOTING]: {
    name: 'Troubleshooting',
    description: 'Common issues and solutions',
    icon: '🔧',
  },
  [ArticleCategory.BEST_PRACTICES]: {
    name: 'Best Practices',
    description: 'Tips and recommendations',
    icon: '💡',
  },
  [ArticleCategory.FAQ]: {
    name: 'FAQ',
    description: 'Frequently asked questions',
    icon: '❓',
  },
  [ArticleCategory.RELEASE_NOTES]: {
    name: 'Release Notes',
    description: 'Updates and changelogs',
    icon: '📝',
  },
};
