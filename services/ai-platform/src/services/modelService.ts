/**
 * Model Service
 *
 * Manages model configurations, versions, deployments, and performance monitoring.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  ModelConfig,
  ModelVersion,
  ModelDeployment,
  ModelMetrics,
  ModelStatus,
  ModelProvider,
  DeploymentStrategy,
  CreateModelInput,
} from '../models/model';

export class ModelService {
  private db: Pool;
  private redis: RedisClientType;
  private cachePrefix = 'model:';
  private cacheTTL = 300; // 5 minutes

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Model Configuration Management
  // ===========================================

  /**
   * Register a new model configuration
   */
  async createModel(input: CreateModelInput, userId: string): Promise<ModelConfig> {
    const model: ModelConfig = {
      id: uuidv4(),
      name: input.name,
      displayName: input.displayName,
      description: input.description,
      provider: input.provider,
      type: input.type,
      modelId: input.modelId,
      version: input.version,
      status: ModelStatus.DRAFT,
      capabilities: input.capabilities,
      config: {
        temperature: 0.7,
        ...input.config,
      },
      pricing: input.pricing,
      rateLimits: input.rateLimits,
      tags: input.tags || [],
      metadata: input.metadata || {},
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO models (
        id, name, display_name, description, provider, type, model_id, version,
        status, capabilities, config, pricing, rate_limits, tags, metadata,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)`,
      [
        model.id, model.name, model.displayName, model.description,
        model.provider, model.type, model.modelId, model.version,
        model.status, JSON.stringify(model.capabilities), JSON.stringify(model.config),
        JSON.stringify(model.pricing), JSON.stringify(model.rateLimits),
        JSON.stringify(model.tags), JSON.stringify(model.metadata),
        model.createdAt, model.updatedAt, model.createdBy,
      ]
    );

    // Create initial version
    await this.createVersion(model.id, model.version, userId);

    return model;
  }

  /**
   * Get model by ID
   */
  async getModel(modelId: string): Promise<ModelConfig | null> {
    const cached = await this.redis.get(`${this.cachePrefix}config:${modelId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    const result = await this.db.query(
      `SELECT * FROM models WHERE id = $1`,
      [modelId]
    );

    if (result.rows.length === 0) return null;

    const model = this.mapModelRow(result.rows[0]);

    await this.redis.set(
      `${this.cachePrefix}config:${modelId}`,
      JSON.stringify(model),
      { EX: this.cacheTTL }
    );

    return model;
  }

  /**
   * Get model by name
   */
  async getModelByName(name: string): Promise<ModelConfig | null> {
    const result = await this.db.query(
      `SELECT * FROM models WHERE name = $1`,
      [name]
    );

    if (result.rows.length === 0) return null;

    return this.mapModelRow(result.rows[0]);
  }

  /**
   * List all models
   */
  async listModels(filters?: {
    provider?: ModelProvider;
    status?: ModelStatus;
    type?: string;
  }): Promise<ModelConfig[]> {
    let query = `SELECT * FROM models WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.provider) {
      query += ` AND provider = $${paramIndex++}`;
      values.push(filters.provider);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }

    query += ` ORDER BY name ASC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapModelRow);
  }

  /**
   * Update model configuration
   */
  async updateModel(
    modelId: string,
    updates: Partial<CreateModelInput>
  ): Promise<ModelConfig | null> {
    const model = await this.getModel(modelId);
    if (!model) return null;

    const fields: string[] = [];
    const values: unknown[] = [];
    let paramIndex = 1;

    if (updates.displayName !== undefined) {
      fields.push(`display_name = $${paramIndex++}`);
      values.push(updates.displayName);
    }
    if (updates.description !== undefined) {
      fields.push(`description = $${paramIndex++}`);
      values.push(updates.description);
    }
    if (updates.config !== undefined) {
      fields.push(`config = $${paramIndex++}`);
      values.push(JSON.stringify({ ...model.config, ...updates.config }));
    }
    if (updates.pricing !== undefined) {
      fields.push(`pricing = $${paramIndex++}`);
      values.push(JSON.stringify(updates.pricing));
    }
    if (updates.rateLimits !== undefined) {
      fields.push(`rate_limits = $${paramIndex++}`);
      values.push(JSON.stringify(updates.rateLimits));
    }
    if (updates.tags !== undefined) {
      fields.push(`tags = $${paramIndex++}`);
      values.push(JSON.stringify(updates.tags));
    }

    if (fields.length === 0) return model;

    fields.push('updated_at = NOW()');
    values.push(modelId);

    await this.db.query(
      `UPDATE models SET ${fields.join(', ')} WHERE id = $${paramIndex}`,
      values
    );

    await this.redis.del(`${this.cachePrefix}config:${modelId}`);

    return this.getModel(modelId);
  }

  /**
   * Update model status
   */
  async updateModelStatus(modelId: string, status: ModelStatus): Promise<void> {
    await this.db.query(
      `UPDATE models SET status = $1, updated_at = NOW() WHERE id = $2`,
      [status, modelId]
    );

    await this.redis.del(`${this.cachePrefix}config:${modelId}`);
  }

  // ===========================================
  // Version Management
  // ===========================================

  /**
   * Create a new model version
   */
  async createVersion(
    modelId: string,
    version: string,
    userId: string,
    options?: {
      parentVersion?: string;
      configOverrides?: Record<string, unknown>;
      changelog?: string;
    }
  ): Promise<ModelVersion> {
    const modelVersion: ModelVersion = {
      id: uuidv4(),
      modelId,
      version,
      parentVersion: options?.parentVersion,
      status: ModelStatus.DRAFT,
      configOverrides: options?.configOverrides || {},
      changelog: options?.changelog,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO model_versions (
        id, model_id, version, parent_version, status, config_overrides,
        changelog, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
      [
        modelVersion.id, modelVersion.modelId, modelVersion.version,
        modelVersion.parentVersion, modelVersion.status,
        JSON.stringify(modelVersion.configOverrides), modelVersion.changelog,
        modelVersion.createdAt, modelVersion.updatedAt,
      ]
    );

    return modelVersion;
  }

  /**
   * Get model version
   */
  async getVersion(versionId: string): Promise<ModelVersion | null> {
    const result = await this.db.query(
      `SELECT * FROM model_versions WHERE id = $1`,
      [versionId]
    );

    if (result.rows.length === 0) return null;

    return this.mapVersionRow(result.rows[0]);
  }

  /**
   * List versions for a model
   */
  async listVersions(modelId: string): Promise<ModelVersion[]> {
    const result = await this.db.query(
      `SELECT * FROM model_versions WHERE model_id = $1 ORDER BY created_at DESC`,
      [modelId]
    );

    return result.rows.map(this.mapVersionRow);
  }

  /**
   * Get active production version
   */
  async getProductionVersion(modelId: string): Promise<ModelVersion | null> {
    const result = await this.db.query(
      `SELECT * FROM model_versions
       WHERE model_id = $1 AND status = 'production'
       ORDER BY deployed_at DESC
       LIMIT 1`,
      [modelId]
    );

    if (result.rows.length === 0) return null;

    return this.mapVersionRow(result.rows[0]);
  }

  // ===========================================
  // Deployment Management
  // ===========================================

  /**
   * Create a deployment
   */
  async createDeployment(
    modelId: string,
    versionId: string,
    options: {
      environment: 'development' | 'staging' | 'production';
      strategy: DeploymentStrategy;
      trafficPercent?: number;
      canaryConfig?: ModelDeployment['canaryConfig'];
    },
    userId: string
  ): Promise<ModelDeployment> {
    const deployment: ModelDeployment = {
      id: uuidv4(),
      modelId,
      versionId,
      environment: options.environment,
      strategy: options.strategy,
      trafficPercent: options.trafficPercent || 100,
      canaryConfig: options.canaryConfig,
      status: 'pending',
      createdAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO model_deployments (
        id, model_id, version_id, environment, strategy, traffic_percent,
        canary_config, status, created_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        deployment.id, deployment.modelId, deployment.versionId,
        deployment.environment, deployment.strategy, deployment.trafficPercent,
        deployment.canaryConfig ? JSON.stringify(deployment.canaryConfig) : null,
        deployment.status, deployment.createdAt, deployment.createdBy,
      ]
    );

    // Start deployment process
    await this.executeDeployment(deployment);

    return deployment;
  }

  /**
   * Execute the deployment
   */
  private async executeDeployment(deployment: ModelDeployment): Promise<void> {
    await this.db.query(
      `UPDATE model_deployments SET status = 'in_progress', started_at = NOW() WHERE id = $1`,
      [deployment.id]
    );

    try {
      switch (deployment.strategy) {
        case DeploymentStrategy.ROLLING:
          await this.executeRollingDeployment(deployment);
          break;
        case DeploymentStrategy.BLUE_GREEN:
          await this.executeBlueGreenDeployment(deployment);
          break;
        case DeploymentStrategy.CANARY:
          await this.executeCanaryDeployment(deployment);
          break;
        case DeploymentStrategy.SHADOW:
          await this.executeShadowDeployment(deployment);
          break;
      }

      // Update version status
      await this.db.query(
        `UPDATE model_versions SET status = $1, deployed_at = NOW(), deployed_by = $2 WHERE id = $3`,
        [deployment.environment, deployment.createdBy, deployment.versionId]
      );

      // Mark deployment complete
      await this.db.query(
        `UPDATE model_deployments SET status = 'completed', completed_at = NOW() WHERE id = $1`,
        [deployment.id]
      );

      // Update model status if production
      if (deployment.environment === 'production') {
        await this.updateModelStatus(deployment.modelId, ModelStatus.PRODUCTION);
      }

      // Clear caches
      await this.redis.del(`${this.cachePrefix}config:${deployment.modelId}`);
      await this.redis.del(`${this.cachePrefix}router:${deployment.modelId}`);

    } catch (error) {
      await this.db.query(
        `UPDATE model_deployments SET status = 'failed', completed_at = NOW() WHERE id = $1`,
        [deployment.id]
      );
      throw error;
    }
  }

  private async executeRollingDeployment(deployment: ModelDeployment): Promise<void> {
    // Rolling deployment - instant cutover for model configs
    await this.db.query(
      `UPDATE model_versions SET traffic_percent = 0
       WHERE model_id = $1 AND status = $2 AND id != $3`,
      [deployment.modelId, deployment.environment, deployment.versionId]
    );
  }

  private async executeBlueGreenDeployment(deployment: ModelDeployment): Promise<void> {
    // Blue-green - switch traffic completely
    await this.db.query(
      `UPDATE model_versions SET traffic_percent = 0
       WHERE model_id = $1 AND status = $2`,
      [deployment.modelId, deployment.environment]
    );
  }

  private async executeCanaryDeployment(deployment: ModelDeployment): Promise<void> {
    // Canary - start with configured traffic percent
    // Full rollout handled by separate job
    if (deployment.canaryConfig) {
      // Store canary state in Redis
      await this.redis.set(
        `${this.cachePrefix}canary:${deployment.id}`,
        JSON.stringify({
          currentPercent: deployment.trafficPercent,
          targetPercent: deployment.canaryConfig.targetPercent,
          incrementPercent: deployment.canaryConfig.incrementPercent,
          intervalMinutes: deployment.canaryConfig.intervalMinutes,
          startedAt: new Date().toISOString(),
        }),
        { EX: 86400 } // 24 hours
      );
    }
  }

  private async executeShadowDeployment(deployment: ModelDeployment): Promise<void> {
    // Shadow - run in parallel without affecting traffic
    await this.db.query(
      `UPDATE model_versions SET is_shadow = true WHERE id = $1`,
      [deployment.versionId]
    );
  }

  /**
   * Rollback a deployment
   */
  async rollbackDeployment(
    deploymentId: string,
    userId: string
  ): Promise<void> {
    const result = await this.db.query(
      `SELECT * FROM model_deployments WHERE id = $1`,
      [deploymentId]
    );

    if (result.rows.length === 0) {
      throw new Error('Deployment not found');
    }

    const deployment = result.rows[0];
    const version = await this.getVersion(deployment.version_id);

    if (!version?.rollbackVersion) {
      throw new Error('No rollback version available');
    }

    // Create rollback deployment
    await this.createDeployment(
      deployment.model_id,
      version.rollbackVersion,
      {
        environment: deployment.environment,
        strategy: DeploymentStrategy.ROLLING,
        trafficPercent: 100,
      },
      userId
    );

    await this.db.query(
      `UPDATE model_deployments SET status = 'rolled_back' WHERE id = $1`,
      [deploymentId]
    );
  }

  // ===========================================
  // Performance Monitoring
  // ===========================================

  /**
   * Record model metrics
   */
  async recordMetrics(
    modelId: string,
    metrics: {
      versionId?: string;
      latencyMs: number;
      inputTokens: number;
      outputTokens: number;
      success: boolean;
      errorType?: string;
      cost?: number;
    }
  ): Promise<void> {
    const now = new Date();
    const hourKey = `${this.cachePrefix}metrics:${modelId}:${now.toISOString().slice(0, 13)}`;

    // Increment counters in Redis
    const pipeline = this.redis.multi();
    pipeline.hIncrBy(hourKey, 'requests', 1);
    pipeline.hIncrBy(hourKey, metrics.success ? 'successful' : 'failed', 1);
    pipeline.hIncrBy(hourKey, 'input_tokens', metrics.inputTokens);
    pipeline.hIncrBy(hourKey, 'output_tokens', metrics.outputTokens);
    pipeline.hIncrByFloat(hourKey, 'total_latency', metrics.latencyMs);
    if (metrics.cost) {
      pipeline.hIncrByFloat(hourKey, 'total_cost', metrics.cost);
    }
    if (metrics.errorType) {
      pipeline.hIncrBy(hourKey, `error:${metrics.errorType}`, 1);
    }

    // Store latency for percentile calculation
    pipeline.lPush(`${hourKey}:latencies`, metrics.latencyMs.toString());
    pipeline.lTrim(`${hourKey}:latencies`, 0, 9999); // Keep last 10000

    pipeline.expire(hourKey, 86400 * 7); // 7 days
    pipeline.expire(`${hourKey}:latencies`, 86400 * 7);

    await pipeline.exec();
  }

  /**
   * Get model metrics for a time period
   */
  async getMetrics(
    modelId: string,
    options: {
      period: 'hour' | 'day' | 'week';
      versionId?: string;
    }
  ): Promise<ModelMetrics[]> {
    const now = new Date();
    const metrics: ModelMetrics[] = [];
    let hours: number;

    switch (options.period) {
      case 'hour':
        hours = 1;
        break;
      case 'day':
        hours = 24;
        break;
      case 'week':
        hours = 168;
        break;
    }

    for (let i = 0; i < hours; i++) {
      const timestamp = new Date(now.getTime() - i * 60 * 60 * 1000);
      const hourKey = `${this.cachePrefix}metrics:${modelId}:${timestamp.toISOString().slice(0, 13)}`;

      const data = await this.redis.hGetAll(hourKey);
      if (Object.keys(data).length === 0) continue;

      // Get latency percentiles
      const latencies = await this.redis.lRange(`${hourKey}:latencies`, 0, -1);
      const sortedLatencies = latencies.map(Number).sort((a, b) => a - b);

      const getPercentile = (arr: number[], p: number): number => {
        if (arr.length === 0) return 0;
        const index = Math.ceil((p / 100) * arr.length) - 1;
        return arr[Math.max(0, index)];
      };

      const requests = parseInt(data.requests || '0', 10);
      const inputTokens = parseInt(data.input_tokens || '0', 10);
      const outputTokens = parseInt(data.output_tokens || '0', 10);
      const totalLatency = parseFloat(data.total_latency || '0');

      // Extract errors
      const errors: Record<string, number> = {};
      for (const [key, value] of Object.entries(data)) {
        if (key.startsWith('error:')) {
          errors[key.replace('error:', '')] = parseInt(value, 10);
        }
      }

      metrics.push({
        id: uuidv4(),
        modelId,
        versionId: options.versionId,
        timestamp,
        period: 'hour',
        requests: {
          total: requests,
          successful: parseInt(data.successful || '0', 10),
          failed: parseInt(data.failed || '0', 10),
          cached: parseInt(data.cached || '0', 10),
        },
        latency: {
          avg: requests > 0 ? totalLatency / requests : 0,
          p50: getPercentile(sortedLatencies, 50),
          p90: getPercentile(sortedLatencies, 90),
          p95: getPercentile(sortedLatencies, 95),
          p99: getPercentile(sortedLatencies, 99),
          max: sortedLatencies[sortedLatencies.length - 1] || 0,
        },
        tokens: {
          inputTotal: inputTokens,
          outputTotal: outputTokens,
          avgInputPerRequest: requests > 0 ? inputTokens / requests : 0,
          avgOutputPerRequest: requests > 0 ? outputTokens / requests : 0,
        },
        cost: {
          total: parseFloat(data.total_cost || '0'),
          inputCost: 0, // Calculate from pricing
          outputCost: 0,
        },
        errors,
      });
    }

    return metrics;
  }

  /**
   * Get real-time model stats
   */
  async getRealtimeStats(modelId: string): Promise<{
    requestsPerMinute: number;
    avgLatency: number;
    errorRate: number;
    activeRequests: number;
  }> {
    const now = new Date();
    const hourKey = `${this.cachePrefix}metrics:${modelId}:${now.toISOString().slice(0, 13)}`;

    const data = await this.redis.hGetAll(hourKey);
    const minutesSinceHourStart = now.getMinutes() || 1;

    const requests = parseInt(data.requests || '0', 10);
    const failed = parseInt(data.failed || '0', 10);
    const totalLatency = parseFloat(data.total_latency || '0');

    const activeRequests = parseInt(
      await this.redis.get(`${this.cachePrefix}active:${modelId}`) || '0',
      10
    );

    return {
      requestsPerMinute: requests / minutesSinceHourStart,
      avgLatency: requests > 0 ? totalLatency / requests : 0,
      errorRate: requests > 0 ? failed / requests : 0,
      activeRequests,
    };
  }

  // ===========================================
  // Model Router
  // ===========================================

  /**
   * Get the best model for a request (for A/B testing and canary)
   */
  async selectModel(
    modelId: string,
    context?: {
      tenantId?: string;
      userId?: string;
      requestId?: string;
    }
  ): Promise<{ model: ModelConfig; version: ModelVersion } | null> {
    const model = await this.getModel(modelId);
    if (!model) return null;

    // Get production version
    const version = await this.getProductionVersion(modelId);
    if (!version) return null;

    // TODO: Add A/B test variant selection logic
    // TODO: Add canary traffic routing

    return { model, version };
  }

  // ===========================================
  // Private Helpers
  // ===========================================

  private mapModelRow(row: any): ModelConfig {
    return {
      id: row.id,
      name: row.name,
      displayName: row.display_name,
      description: row.description,
      provider: row.provider,
      type: row.type,
      modelId: row.model_id,
      version: row.version,
      status: row.status,
      capabilities: row.capabilities,
      config: row.config,
      pricing: row.pricing,
      rateLimits: row.rate_limits,
      tags: row.tags || [],
      metadata: row.metadata || {},
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
    };
  }

  private mapVersionRow(row: any): ModelVersion {
    return {
      id: row.id,
      modelId: row.model_id,
      version: row.version,
      parentVersion: row.parent_version,
      status: row.status,
      configOverrides: row.config_overrides || {},
      baselineMetrics: row.baseline_metrics,
      deployedAt: row.deployed_at,
      deployedBy: row.deployed_by,
      rollbackVersion: row.rollback_version,
      changelog: row.changelog,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
