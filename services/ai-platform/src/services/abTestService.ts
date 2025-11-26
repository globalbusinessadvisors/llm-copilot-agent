/**
 * A/B Testing Service
 *
 * Manages A/B tests for model comparison and optimization.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  ABTest,
  CreateABTestInput,
} from '../models/model';

interface VariantMetrics {
  samples: number;
  metrics: Record<string, number>;
  latencySum: number;
  latencySquaredSum: number;
  errorCount: number;
  qualityScoreSum: number;
  costSum: number;
  userRatingSum: number;
  userRatingCount: number;
}

export class ABTestService {
  private db: Pool;
  private redis: RedisClientType;
  private cachePrefix = 'abtest:';

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  /**
   * Create a new A/B test
   */
  async createTest(input: CreateABTestInput, userId: string): Promise<ABTest> {
    // Validate traffic percentages sum to 100
    const totalTraffic = input.variants.reduce((sum, v) => sum + v.trafficPercent, 0);
    if (Math.abs(totalTraffic - 100) > 0.01) {
      throw new Error('Variant traffic percentages must sum to 100');
    }

    // Ensure exactly one control variant
    const controlCount = input.variants.filter(v => v.isControl).length;
    if (controlCount !== 1) {
      throw new Error('Exactly one variant must be marked as control');
    }

    const test: ABTest = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      status: 'draft',
      variants: input.variants.map(v => ({
        ...v,
        id: uuidv4(),
        isControl: v.isControl || false,
      })),
      targeting: input.targeting || {},
      primaryMetric: input.primaryMetric,
      secondaryMetrics: input.secondaryMetrics || [],
      minimumSampleSize: input.minimumSampleSize || 100,
      confidenceLevel: input.confidenceLevel || 0.95,
      startAt: input.startAt,
      endAt: input.endAt,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO ab_tests (
        id, name, description, status, variants, targeting,
        primary_metric, secondary_metrics, minimum_sample_size, confidence_level,
        start_at, end_at, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        test.id, test.name, test.description, test.status,
        JSON.stringify(test.variants), JSON.stringify(test.targeting),
        test.primaryMetric, JSON.stringify(test.secondaryMetrics),
        test.minimumSampleSize, test.confidenceLevel,
        test.startAt, test.endAt, test.createdAt, test.updatedAt, test.createdBy,
      ]
    );

    return test;
  }

  /**
   * Get A/B test by ID
   */
  async getTest(testId: string): Promise<ABTest | null> {
    const result = await this.db.query(
      `SELECT * FROM ab_tests WHERE id = $1`,
      [testId]
    );

    if (result.rows.length === 0) return null;

    return this.mapTestRow(result.rows[0]);
  }

  /**
   * List A/B tests
   */
  async listTests(filters?: {
    status?: ABTest['status'];
    modelId?: string;
  }): Promise<ABTest[]> {
    let query = `SELECT * FROM ab_tests WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapTestRow);
  }

  /**
   * Start an A/B test
   */
  async startTest(testId: string): Promise<void> {
    const test = await this.getTest(testId);
    if (!test) throw new Error('Test not found');
    if (test.status !== 'draft' && test.status !== 'paused') {
      throw new Error('Test can only be started from draft or paused status');
    }

    // Initialize metrics storage in Redis
    for (const variant of test.variants) {
      await this.redis.hSet(`${this.cachePrefix}${testId}:metrics:${variant.id}`, {
        samples: '0',
        latencySum: '0',
        latencySquaredSum: '0',
        errorCount: '0',
        qualityScoreSum: '0',
        costSum: '0',
        userRatingSum: '0',
        userRatingCount: '0',
      });
    }

    await this.db.query(
      `UPDATE ab_tests SET status = 'running', start_at = COALESCE(start_at, NOW()), updated_at = NOW() WHERE id = $1`,
      [testId]
    );
  }

  /**
   * Pause an A/B test
   */
  async pauseTest(testId: string): Promise<void> {
    await this.db.query(
      `UPDATE ab_tests SET status = 'paused', updated_at = NOW() WHERE id = $1`,
      [testId]
    );
  }

  /**
   * Complete an A/B test
   */
  async completeTest(testId: string): Promise<ABTest> {
    const test = await this.getTest(testId);
    if (!test) throw new Error('Test not found');

    // Calculate final results
    const results = await this.calculateResults(test);

    await this.db.query(
      `UPDATE ab_tests SET status = 'completed', results = $1, end_at = NOW(), updated_at = NOW() WHERE id = $2`,
      [JSON.stringify(results), testId]
    );

    return { ...test, status: 'completed', results };
  }

  /**
   * Cancel an A/B test
   */
  async cancelTest(testId: string): Promise<void> {
    await this.db.query(
      `UPDATE ab_tests SET status = 'cancelled', end_at = NOW(), updated_at = NOW() WHERE id = $1`,
      [testId]
    );

    // Clean up Redis data
    const test = await this.getTest(testId);
    if (test) {
      for (const variant of test.variants) {
        await this.redis.del(`${this.cachePrefix}${testId}:metrics:${variant.id}`);
      }
    }
  }

  /**
   * Assign a user to a variant
   */
  async assignVariant(
    testId: string,
    context: {
      userId?: string;
      tenantId?: string;
      requestId: string;
    }
  ): Promise<{ variantId: string; modelId: string; versionId?: string } | null> {
    const test = await this.getTest(testId);
    if (!test || test.status !== 'running') return null;

    // Check targeting rules
    if (!this.matchesTargeting(test.targeting, context)) {
      return null;
    }

    // Check if user is already assigned
    const existingAssignment = context.userId
      ? await this.redis.get(`${this.cachePrefix}${testId}:assignment:${context.userId}`)
      : null;

    if (existingAssignment) {
      const variant = test.variants.find(v => v.id === existingAssignment);
      if (variant) {
        return {
          variantId: variant.id,
          modelId: variant.modelId,
          versionId: variant.versionId,
        };
      }
    }

    // Assign to variant based on traffic weights
    const variant = this.selectVariant(test.variants, context.requestId);

    // Store assignment for consistent bucketing
    if (context.userId) {
      await this.redis.set(
        `${this.cachePrefix}${testId}:assignment:${context.userId}`,
        variant.id,
        { EX: 86400 * 30 } // 30 days
      );
    }

    return {
      variantId: variant.id,
      modelId: variant.modelId,
      versionId: variant.versionId,
    };
  }

  /**
   * Record a test impression/sample
   */
  async recordSample(
    testId: string,
    variantId: string,
    metrics: {
      latencyMs: number;
      success: boolean;
      qualityScore?: number;
      cost?: number;
      userRating?: number;
    }
  ): Promise<void> {
    const key = `${this.cachePrefix}${testId}:metrics:${variantId}`;

    const pipeline = this.redis.multi();
    pipeline.hIncrBy(key, 'samples', 1);
    pipeline.hIncrByFloat(key, 'latencySum', metrics.latencyMs);
    pipeline.hIncrByFloat(key, 'latencySquaredSum', metrics.latencyMs * metrics.latencyMs);

    if (!metrics.success) {
      pipeline.hIncrBy(key, 'errorCount', 1);
    }
    if (metrics.qualityScore !== undefined) {
      pipeline.hIncrByFloat(key, 'qualityScoreSum', metrics.qualityScore);
    }
    if (metrics.cost !== undefined) {
      pipeline.hIncrByFloat(key, 'costSum', metrics.cost);
    }
    if (metrics.userRating !== undefined) {
      pipeline.hIncrByFloat(key, 'userRatingSum', metrics.userRating);
      pipeline.hIncrBy(key, 'userRatingCount', 1);
    }

    await pipeline.exec();

    // Check if we've reached statistical significance
    await this.checkSignificance(testId);
  }

  /**
   * Get current test results
   */
  async getResults(testId: string): Promise<ABTest['results']> {
    const test = await this.getTest(testId);
    if (!test) throw new Error('Test not found');

    return this.calculateResults(test);
  }

  // ===========================================
  // Statistical Analysis
  // ===========================================

  /**
   * Calculate test results with statistical analysis
   */
  private async calculateResults(test: ABTest): Promise<ABTest['results']> {
    const variantResults: Record<string, { samples: number; metrics: Record<string, number> }> = {};
    let totalSamples = 0;

    // Gather metrics for each variant
    for (const variant of test.variants) {
      const data = await this.redis.hGetAll(`${this.cachePrefix}${test.id}:metrics:${variant.id}`);

      const samples = parseInt(data.samples || '0', 10);
      totalSamples += samples;

      const latencySum = parseFloat(data.latencySum || '0');
      const latencySquaredSum = parseFloat(data.latencySquaredSum || '0');
      const errorCount = parseInt(data.errorCount || '0', 10);
      const qualityScoreSum = parseFloat(data.qualityScoreSum || '0');
      const costSum = parseFloat(data.costSum || '0');
      const userRatingSum = parseFloat(data.userRatingSum || '0');
      const userRatingCount = parseInt(data.userRatingCount || '0', 10);

      variantResults[variant.id] = {
        samples,
        metrics: {
          latency: samples > 0 ? latencySum / samples : 0,
          latencyStdDev: samples > 1
            ? Math.sqrt((latencySquaredSum - (latencySum * latencySum) / samples) / (samples - 1))
            : 0,
          error_rate: samples > 0 ? errorCount / samples : 0,
          quality_score: samples > 0 ? qualityScoreSum / samples : 0,
          cost: samples > 0 ? costSum / samples : 0,
          user_rating: userRatingCount > 0 ? userRatingSum / userRatingCount : 0,
        },
      };
    }

    // Determine winner based on primary metric
    let winner: string | undefined;
    let statisticalSignificance: number | undefined;

    const controlVariant = test.variants.find(v => v.isControl);
    if (controlVariant && totalSamples >= test.minimumSampleSize * test.variants.length) {
      const controlMetrics = variantResults[controlVariant.id];

      // Find best performing variant
      let bestVariantId: string | undefined;
      let bestImprovement = 0;

      for (const variant of test.variants) {
        if (variant.isControl) continue;

        const variantMetricsData = variantResults[variant.id];
        const controlValue = this.getPrimaryMetricValue(controlMetrics.metrics, test.primaryMetric);
        const variantValue = this.getPrimaryMetricValue(variantMetricsData.metrics, test.primaryMetric);

        // Calculate improvement (lower is better for latency/cost/error_rate)
        const lowerIsBetter = ['latency', 'error_rate', 'cost'].includes(test.primaryMetric);
        const improvement = lowerIsBetter
          ? (controlValue - variantValue) / controlValue
          : (variantValue - controlValue) / controlValue;

        if (improvement > bestImprovement) {
          bestImprovement = improvement;
          bestVariantId = variant.id;
        }
      }

      // Calculate statistical significance using z-test
      if (bestVariantId) {
        const significance = this.calculateSignificance(
          controlMetrics,
          variantResults[bestVariantId],
          test.primaryMetric
        );

        if (significance >= test.confidenceLevel) {
          winner = bestVariantId;
          statisticalSignificance = significance;
        }
      }
    }

    return {
      totalSamples,
      variantResults,
      winner,
      statisticalSignificance,
      conclusionAt: winner ? new Date() : undefined,
    };
  }

  /**
   * Calculate statistical significance using two-sample z-test
   */
  private calculateSignificance(
    control: { samples: number; metrics: Record<string, number> },
    treatment: { samples: number; metrics: Record<string, number> },
    metric: string
  ): number {
    // Get metric values
    const controlMean = this.getPrimaryMetricValue(control.metrics, metric);
    const treatmentMean = this.getPrimaryMetricValue(treatment.metrics, metric);

    // For simplicity, use a pooled estimate of standard deviation
    // In production, would use actual sample standard deviations
    const controlStdDev = control.metrics[`${metric}_std_dev`] || control.metrics.latencyStdDev || controlMean * 0.2;
    const treatmentStdDev = treatment.metrics[`${metric}_std_dev`] || treatment.metrics.latencyStdDev || treatmentMean * 0.2;

    const n1 = control.samples;
    const n2 = treatment.samples;

    if (n1 < 30 || n2 < 30) {
      return 0; // Not enough samples
    }

    // Calculate z-score
    const pooledStdErr = Math.sqrt(
      (controlStdDev * controlStdDev) / n1 + (treatmentStdDev * treatmentStdDev) / n2
    );

    if (pooledStdErr === 0) return 0;

    const zScore = Math.abs(treatmentMean - controlMean) / pooledStdErr;

    // Convert z-score to confidence level using normal CDF approximation
    const confidence = this.normalCDF(zScore);

    return confidence;
  }

  /**
   * Normal CDF approximation
   */
  private normalCDF(x: number): number {
    const a1 = 0.254829592;
    const a2 = -0.284496736;
    const a3 = 1.421413741;
    const a4 = -1.453152027;
    const a5 = 1.061405429;
    const p = 0.3275911;

    const sign = x < 0 ? -1 : 1;
    x = Math.abs(x) / Math.sqrt(2);

    const t = 1.0 / (1.0 + p * x);
    const y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x);

    return 0.5 * (1.0 + sign * y);
  }

  private getPrimaryMetricValue(metrics: Record<string, number>, metric: string): number {
    switch (metric) {
      case 'latency':
        return metrics.latency || 0;
      case 'error_rate':
        return metrics.error_rate || 0;
      case 'quality_score':
        return metrics.quality_score || 0;
      case 'cost':
        return metrics.cost || 0;
      case 'user_rating':
        return metrics.user_rating || 0;
      default:
        return metrics[metric] || 0;
    }
  }

  /**
   * Check if test has reached statistical significance
   */
  private async checkSignificance(testId: string): Promise<void> {
    const test = await this.getTest(testId);
    if (!test || test.status !== 'running') return;

    const results = await this.calculateResults(test);

    if (results.winner && results.statisticalSignificance) {
      // Optionally auto-complete the test
      if (test.endAt && new Date() >= test.endAt) {
        await this.completeTest(testId);
      }
    }
  }

  // ===========================================
  // Targeting
  // ===========================================

  private matchesTargeting(
    targeting: ABTest['targeting'],
    context: { userId?: string; tenantId?: string }
  ): boolean {
    // Check tenant targeting
    if (targeting.tenantIds?.length && context.tenantId) {
      if (!targeting.tenantIds.includes(context.tenantId)) {
        return false;
      }
    }

    // Check user targeting
    if (targeting.userIds?.length && context.userId) {
      if (!targeting.userIds.includes(context.userId)) {
        return false;
      }
    }

    // Check percentage targeting
    if (targeting.userPercentage !== undefined && targeting.userPercentage < 100) {
      const hash = this.hashString(context.userId || context.tenantId || '');
      if ((hash % 100) >= targeting.userPercentage) {
        return false;
      }
    }

    return true;
  }

  /**
   * Select variant based on traffic weights
   */
  private selectVariant(
    variants: ABTest['variants'],
    requestId: string
  ): ABTest['variants'][0] {
    // Use consistent hashing for variant assignment
    const hash = this.hashString(requestId) % 10000;
    let cumulative = 0;

    for (const variant of variants) {
      cumulative += variant.trafficPercent * 100;
      if (hash < cumulative) {
        return variant;
      }
    }

    // Fallback to last variant
    return variants[variants.length - 1];
  }

  /**
   * Simple hash function for consistent bucketing
   */
  private hashString(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return Math.abs(hash);
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapTestRow(row: any): ABTest {
    return {
      id: row.id,
      name: row.name,
      description: row.description,
      status: row.status,
      variants: row.variants,
      targeting: row.targeting || {},
      primaryMetric: row.primary_metric,
      secondaryMetrics: row.secondary_metrics || [],
      minimumSampleSize: row.minimum_sample_size,
      confidenceLevel: row.confidence_level,
      startAt: row.start_at,
      endAt: row.end_at,
      results: row.results,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
    };
  }
}
