/**
 * Content Filter Service
 *
 * Manages content filtering rules and processes content for safety and compliance.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import OpenAI from 'openai';
import {
  ContentFilterRule,
  ContentFilterResult,
  ContentCategory,
  FilterAction,
  FilterDirection,
  CreateContentFilterRuleInput,
  FilterContentInput,
} from '../models/governance';

// PII patterns
const PII_PATTERNS = {
  email: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g,
  phone: /\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b/g,
  ssn: /\b\d{3}[-.\s]?\d{2}[-.\s]?\d{4}\b/g,
  creditCard: /\b(?:\d{4}[-.\s]?){3}\d{4}\b/g,
  ipAddress: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g,
  dateOfBirth: /\b(?:\d{1,2}[-/]\d{1,2}[-/]\d{2,4})\b/g,
};

export class ContentFilterService {
  private db: Pool;
  private redis: RedisClientType;
  private openai: OpenAI | null = null;
  private rulesCache: Map<string, ContentFilterRule[]> = new Map();

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;

    // Initialize OpenAI for content moderation
    if (process.env.OPENAI_API_KEY) {
      this.openai = new OpenAI({
        apiKey: process.env.OPENAI_API_KEY,
      });
    }
  }

  // ===========================================
  // Rule Management
  // ===========================================

  /**
   * Create a content filter rule
   */
  async createRule(input: CreateContentFilterRuleInput, userId: string): Promise<ContentFilterRule> {
    const rule: ContentFilterRule = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      category: input.category,
      direction: input.direction,
      action: input.action,
      priority: input.priority || 50,
      conditions: input.conditions,
      exceptions: input.exceptions,
      redactionConfig: input.redactionConfig,
      enabled: input.enabled ?? true,
      metadata: input.metadata,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO content_filter_rules (
        id, name, description, category, direction, action, priority,
        conditions, exceptions, redaction_config, enabled, metadata,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        rule.id, rule.name, rule.description, rule.category, rule.direction,
        rule.action, rule.priority, JSON.stringify(rule.conditions),
        JSON.stringify(rule.exceptions), JSON.stringify(rule.redactionConfig),
        rule.enabled, JSON.stringify(rule.metadata), rule.createdAt,
        rule.updatedAt, userId,
      ]
    );

    // Invalidate cache
    await this.invalidateRulesCache();

    return rule;
  }

  /**
   * Get rule by ID
   */
  async getRule(ruleId: string): Promise<ContentFilterRule | null> {
    const result = await this.db.query(
      `SELECT * FROM content_filter_rules WHERE id = $1`,
      [ruleId]
    );

    if (result.rows.length === 0) return null;

    return this.mapRuleRow(result.rows[0]);
  }

  /**
   * List rules
   */
  async listRules(filters?: {
    category?: ContentCategory;
    direction?: FilterDirection;
    enabled?: boolean;
  }): Promise<ContentFilterRule[]> {
    let query = `SELECT * FROM content_filter_rules WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.category) {
      query += ` AND category = $${paramIndex++}`;
      values.push(filters.category);
    }
    if (filters?.direction) {
      query += ` AND direction IN ($${paramIndex++}, 'both')`;
      values.push(filters.direction);
    }
    if (filters?.enabled !== undefined) {
      query += ` AND enabled = $${paramIndex++}`;
      values.push(filters.enabled);
    }

    query += ` ORDER BY priority DESC, created_at ASC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapRuleRow);
  }

  /**
   * Update rule
   */
  async updateRule(
    ruleId: string,
    updates: Partial<CreateContentFilterRuleInput>
  ): Promise<ContentFilterRule> {
    const rule = await this.getRule(ruleId);
    if (!rule) throw new Error('Rule not found');

    const updatedRule = {
      ...rule,
      ...updates,
      updatedAt: new Date(),
    };

    await this.db.query(
      `UPDATE content_filter_rules SET
        name = $1, description = $2, category = $3, direction = $4,
        action = $5, priority = $6, conditions = $7, exceptions = $8,
        redaction_config = $9, enabled = $10, metadata = $11, updated_at = $12
      WHERE id = $13`,
      [
        updatedRule.name, updatedRule.description, updatedRule.category,
        updatedRule.direction, updatedRule.action, updatedRule.priority,
        JSON.stringify(updatedRule.conditions), JSON.stringify(updatedRule.exceptions),
        JSON.stringify(updatedRule.redactionConfig), updatedRule.enabled,
        JSON.stringify(updatedRule.metadata), updatedRule.updatedAt, ruleId,
      ]
    );

    await this.invalidateRulesCache();

    return updatedRule;
  }

  /**
   * Delete rule
   */
  async deleteRule(ruleId: string): Promise<void> {
    await this.db.query(`DELETE FROM content_filter_rules WHERE id = $1`, [ruleId]);
    await this.invalidateRulesCache();
  }

  // ===========================================
  // Content Filtering
  // ===========================================

  /**
   * Filter content
   */
  async filterContent(input: FilterContentInput): Promise<ContentFilterResult> {
    const startTime = Date.now();
    const rules = await this.getActiveRules(input.direction);
    const matches: ContentFilterResult['matches'] = [];
    let highestPriorityAction: FilterAction = FilterAction.ALLOW;
    let highestPriorityRule: ContentFilterRule | undefined;
    let category: ContentCategory | undefined;
    let confidence = 1.0;

    // Process each rule in priority order
    for (const rule of rules) {
      // Check exceptions
      if (this.isExempt(rule, input)) {
        continue;
      }

      const ruleMatches = await this.evaluateRule(rule, input.content);

      if (ruleMatches.length > 0) {
        matches.push(...ruleMatches);

        if (this.getActionPriority(rule.action) > this.getActionPriority(highestPriorityAction)) {
          highestPriorityAction = rule.action;
          highestPriorityRule = rule;
          category = rule.category;
        }
      }
    }

    // Use OpenAI moderation for additional safety check
    if (this.openai && input.direction !== FilterDirection.INPUT) {
      const moderationResult = await this.moderateWithOpenAI(input.content);
      if (moderationResult.flagged) {
        confidence = Math.max(...Object.values(moderationResult.scores));
        if (this.getActionPriority(FilterAction.BLOCK) > this.getActionPriority(highestPriorityAction)) {
          highestPriorityAction = FilterAction.BLOCK;
          category = this.mapModerationCategory(moderationResult.categories);
        }
      }
    }

    // Apply redaction if needed
    let redactedContent: string | undefined;
    if (highestPriorityAction === FilterAction.REDACT && matches.length > 0) {
      redactedContent = this.redactContent(input.content, matches, highestPriorityRule?.redactionConfig);
    }

    const result: ContentFilterResult = {
      id: uuidv4(),
      ruleId: highestPriorityRule?.id,
      content: input.content,
      direction: input.direction,
      category,
      action: highestPriorityAction,
      confidence,
      matches: matches.length > 0 ? matches : undefined,
      redactedContent,
      metadata: {
        processingTimeMs: Date.now() - startTime,
        rulesEvaluated: rules.length,
        matchCount: matches.length,
      },
      processedAt: new Date(),
    };

    // Log the filter result
    await this.logFilterResult(result, input);

    return result;
  }

  /**
   * Evaluate a rule against content
   */
  private async evaluateRule(
    rule: ContentFilterRule,
    content: string
  ): Promise<NonNullable<ContentFilterResult['matches']>> {
    const matches: NonNullable<ContentFilterResult['matches']> = [];

    // Check patterns
    if (rule.conditions.patterns) {
      for (const pattern of rule.conditions.patterns) {
        try {
          const regex = new RegExp(pattern, 'gi');
          let match;
          while ((match = regex.exec(content)) !== null) {
            matches.push({
              pattern,
              location: {
                start: match.index,
                end: match.index + match[0].length,
              },
              text: match[0],
            });
          }
        } catch {
          // Invalid regex, skip
        }
      }
    }

    // Check keywords
    if (rule.conditions.keywords) {
      const contentLower = content.toLowerCase();
      for (const keyword of rule.conditions.keywords) {
        const keywordLower = keyword.toLowerCase();
        let index = 0;
        while ((index = contentLower.indexOf(keywordLower, index)) !== -1) {
          matches.push({
            pattern: keyword,
            location: {
              start: index,
              end: index + keyword.length,
            },
            text: content.slice(index, index + keyword.length),
          });
          index += keyword.length;
        }
      }
    }

    // Check PII patterns for PII category
    if (rule.category === ContentCategory.PII) {
      for (const [piiType, pattern] of Object.entries(PII_PATTERNS)) {
        let match;
        while ((match = pattern.exec(content)) !== null) {
          matches.push({
            pattern: piiType,
            location: {
              start: match.index,
              end: match.index + match[0].length,
            },
            text: match[0],
          });
        }
      }
    }

    return matches;
  }

  /**
   * Moderate content with OpenAI
   */
  private async moderateWithOpenAI(content: string): Promise<{
    flagged: boolean;
    categories: Record<string, boolean>;
    scores: Record<string, number>;
  }> {
    if (!this.openai) {
      return { flagged: false, categories: {}, scores: {} };
    }

    try {
      const response = await this.openai.moderations.create({
        input: content,
      });

      const result = response.results[0];
      return {
        flagged: result.flagged,
        categories: result.categories as unknown as Record<string, boolean>,
        scores: result.category_scores as unknown as Record<string, number>,
      };
    } catch (error) {
      console.error('OpenAI moderation failed:', error);
      return { flagged: false, categories: {}, scores: {} };
    }
  }

  /**
   * Map OpenAI moderation categories to our categories
   */
  private mapModerationCategory(categories: Record<string, boolean>): ContentCategory {
    if (categories['hate'] || categories['hate/threatening']) {
      return ContentCategory.HATE_SPEECH;
    }
    if (categories['violence'] || categories['violence/graphic']) {
      return ContentCategory.VIOLENCE;
    }
    if (categories['sexual'] || categories['sexual/minors']) {
      return ContentCategory.SEXUAL;
    }
    if (categories['self-harm'] || categories['self-harm/intent'] || categories['self-harm/instructions']) {
      return ContentCategory.SELF_HARM;
    }
    if (categories['harassment'] || categories['harassment/threatening']) {
      return ContentCategory.HARASSMENT;
    }
    return ContentCategory.CUSTOM;
  }

  /**
   * Redact content
   */
  private redactContent(
    content: string,
    matches: NonNullable<ContentFilterResult['matches']>,
    config?: ContentFilterRule['redactionConfig']
  ): string {
    // Sort matches by location in reverse order to preserve indices
    const sortedMatches = [...matches].sort((a, b) => b.location.start - a.location.start);

    let redacted = content;
    for (const match of sortedMatches) {
      const replacement = config?.replacement ||
        (config?.preserveLength
          ? (config?.maskChar || '*').repeat(match.text.length)
          : '[REDACTED]');

      redacted =
        redacted.slice(0, match.location.start) +
        replacement +
        redacted.slice(match.location.end);
    }

    return redacted;
  }

  /**
   * Check if user is exempt from rule
   */
  private isExempt(rule: ContentFilterRule, input: FilterContentInput): boolean {
    if (!rule.exceptions) return false;

    if (rule.exceptions.users && input.userId) {
      if (rule.exceptions.users.includes(input.userId)) {
        return true;
      }
    }

    if (rule.exceptions.contexts && input.context) {
      for (const context of rule.exceptions.contexts) {
        if (input.context[context]) {
          return true;
        }
      }
    }

    return false;
  }

  /**
   * Get action priority (higher = more restrictive)
   */
  private getActionPriority(action: FilterAction): number {
    const priorities: Record<FilterAction, number> = {
      [FilterAction.ALLOW]: 0,
      [FilterAction.LOG]: 1,
      [FilterAction.FLAG]: 2,
      [FilterAction.WARN]: 3,
      [FilterAction.REDACT]: 4,
      [FilterAction.BLOCK]: 5,
    };
    return priorities[action];
  }

  /**
   * Get active rules from cache or database
   */
  private async getActiveRules(direction: FilterDirection): Promise<ContentFilterRule[]> {
    const cacheKey = `filter-rules:${direction}`;

    if (this.rulesCache.has(cacheKey)) {
      return this.rulesCache.get(cacheKey)!;
    }

    const rules = await this.listRules({ direction, enabled: true });
    this.rulesCache.set(cacheKey, rules);

    return rules;
  }

  /**
   * Invalidate rules cache
   */
  private async invalidateRulesCache(): Promise<void> {
    this.rulesCache.clear();
    await this.redis.del('filter-rules:input');
    await this.redis.del('filter-rules:output');
    await this.redis.del('filter-rules:both');
  }

  /**
   * Log filter result
   */
  private async logFilterResult(result: ContentFilterResult, input: FilterContentInput): Promise<void> {
    // Only log if action is not ALLOW
    if (result.action === FilterAction.ALLOW) return;

    await this.db.query(
      `INSERT INTO content_filter_logs (
        id, rule_id, direction, category, action, confidence,
        match_count, user_id, processed_at, metadata
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        result.id, result.ruleId, result.direction, result.category,
        result.action, result.confidence, result.matches?.length || 0,
        input.userId, result.processedAt, JSON.stringify(result.metadata),
      ]
    );
  }

  /**
   * Get filter statistics
   */
  async getStatistics(options: {
    startDate?: Date;
    endDate?: Date;
    groupBy?: 'hour' | 'day' | 'week';
  }): Promise<{
    totalFiltered: number;
    byAction: Record<FilterAction, number>;
    byCategory: Record<ContentCategory, number>;
    byDirection: Record<FilterDirection, number>;
    timeline: Array<{ period: string; count: number }>;
  }> {
    const dateFilter = options.startDate
      ? `AND processed_at >= $1 AND processed_at <= $2`
      : '';
    const values = options.startDate
      ? [options.startDate, options.endDate || new Date()]
      : [];

    // Total and by action
    const actionResult = await this.db.query(
      `SELECT action, COUNT(*) as count FROM content_filter_logs
       WHERE 1=1 ${dateFilter} GROUP BY action`,
      values
    );

    const byAction: Record<string, number> = {};
    let total = 0;
    actionResult.rows.forEach(row => {
      byAction[row.action] = parseInt(row.count, 10);
      total += parseInt(row.count, 10);
    });

    // By category
    const categoryResult = await this.db.query(
      `SELECT category, COUNT(*) as count FROM content_filter_logs
       WHERE category IS NOT NULL ${dateFilter} GROUP BY category`,
      values
    );

    const byCategory: Record<string, number> = {};
    categoryResult.rows.forEach(row => {
      byCategory[row.category] = parseInt(row.count, 10);
    });

    // By direction
    const directionResult = await this.db.query(
      `SELECT direction, COUNT(*) as count FROM content_filter_logs
       WHERE 1=1 ${dateFilter} GROUP BY direction`,
      values
    );

    const byDirection: Record<string, number> = {};
    directionResult.rows.forEach(row => {
      byDirection[row.direction] = parseInt(row.count, 10);
    });

    // Timeline
    const groupBy = options.groupBy || 'day';
    const truncate = groupBy === 'hour' ? 'hour' : groupBy === 'week' ? 'week' : 'day';
    const timelineResult = await this.db.query(
      `SELECT DATE_TRUNC('${truncate}', processed_at) as period, COUNT(*) as count
       FROM content_filter_logs
       WHERE 1=1 ${dateFilter}
       GROUP BY period ORDER BY period`,
      values
    );

    const timeline = timelineResult.rows.map(row => ({
      period: row.period.toISOString(),
      count: parseInt(row.count, 10),
    }));

    return {
      totalFiltered: total,
      byAction: byAction as Record<FilterAction, number>,
      byCategory: byCategory as Record<ContentCategory, number>,
      byDirection: byDirection as Record<FilterDirection, number>,
      timeline,
    };
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapRuleRow(row: Record<string, unknown>): ContentFilterRule {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      category: row.category as ContentCategory,
      direction: row.direction as FilterDirection,
      action: row.action as FilterAction,
      priority: row.priority as number,
      conditions: row.conditions as ContentFilterRule['conditions'],
      exceptions: row.exceptions as ContentFilterRule['exceptions'],
      redactionConfig: row.redaction_config as ContentFilterRule['redactionConfig'],
      enabled: row.enabled as boolean,
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }
}
