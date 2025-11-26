/**
 * Fine-Tuning Service
 *
 * Manages fine-tuning jobs for custom model training.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import OpenAI from 'openai';
import {
  FineTuneJob,
  CreateFineTuneJobInput,
  ModelProvider,
} from '../models/model';

export class FineTuneService {
  private db: Pool;
  private redis: RedisClientType;
  private openai: OpenAI | null = null;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;

    // Initialize OpenAI client if API key is available
    if (process.env.OPENAI_API_KEY) {
      this.openai = new OpenAI({
        apiKey: process.env.OPENAI_API_KEY,
      });
    }
  }

  /**
   * Create a new fine-tuning job
   */
  async createJob(input: CreateFineTuneJobInput, userId: string): Promise<FineTuneJob> {
    const job: FineTuneJob = {
      id: uuidv4(),
      name: input.name,
      baseModelId: input.baseModelId,
      status: 'pending',
      trainingData: {
        ...input.trainingData,
        validationSplit: input.trainingData.validationSplit || 0.1,
      },
      hyperparameters: {
        epochs: 3,
        ...input.hyperparameters,
      },
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO fine_tune_jobs (
        id, name, base_model_id, status, training_data, hyperparameters,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
      [
        job.id, job.name, job.baseModelId, job.status,
        JSON.stringify(job.trainingData), JSON.stringify(job.hyperparameters),
        job.createdAt, job.updatedAt, job.createdBy,
      ]
    );

    return job;
  }

  /**
   * Get fine-tune job by ID
   */
  async getJob(jobId: string): Promise<FineTuneJob | null> {
    const result = await this.db.query(
      `SELECT * FROM fine_tune_jobs WHERE id = $1`,
      [jobId]
    );

    if (result.rows.length === 0) return null;

    return this.mapJobRow(result.rows[0]);
  }

  /**
   * List fine-tune jobs
   */
  async listJobs(filters?: {
    status?: FineTuneJob['status'];
    baseModelId?: string;
  }): Promise<FineTuneJob[]> {
    let query = `SELECT * FROM fine_tune_jobs WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.baseModelId) {
      query += ` AND base_model_id = $${paramIndex++}`;
      values.push(filters.baseModelId);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapJobRow);
  }

  /**
   * Start a fine-tuning job
   */
  async startJob(jobId: string): Promise<FineTuneJob> {
    const job = await this.getJob(jobId);
    if (!job) throw new Error('Job not found');
    if (job.status !== 'pending') {
      throw new Error('Job can only be started from pending status');
    }

    // Get base model info
    const modelResult = await this.db.query(
      `SELECT * FROM models WHERE id = $1`,
      [job.baseModelId]
    );

    if (modelResult.rows.length === 0) {
      throw new Error('Base model not found');
    }

    const baseModel = modelResult.rows[0];

    // Update status to preparing
    await this.updateJobStatus(jobId, 'preparing');

    try {
      // Start fine-tuning based on provider
      switch (baseModel.provider) {
        case ModelProvider.OPENAI:
          await this.startOpenAIFineTune(job, baseModel.model_id);
          break;
        case ModelProvider.ANTHROPIC:
          throw new Error('Anthropic fine-tuning not yet supported');
        default:
          throw new Error(`Fine-tuning not supported for provider: ${baseModel.provider}`);
      }

      return this.getJob(jobId) as Promise<FineTuneJob>;
    } catch (error) {
      await this.updateJobStatus(jobId, 'failed');
      throw error;
    }
  }

  /**
   * Start OpenAI fine-tuning
   */
  private async startOpenAIFineTune(job: FineTuneJob, baseModelId: string): Promise<void> {
    if (!this.openai) {
      throw new Error('OpenAI client not configured');
    }

    // In production, would upload training file first
    // For now, assume file is already uploaded to OpenAI
    const fineTuneResponse = await this.openai.fineTuning.jobs.create({
      model: baseModelId,
      training_file: job.trainingData.fileId,
      hyperparameters: {
        n_epochs: job.hyperparameters.epochs,
      },
    });

    await this.db.query(
      `UPDATE fine_tune_jobs SET
        provider_job_id = $1, status = 'training', started_at = NOW(), updated_at = NOW()
      WHERE id = $2`,
      [fineTuneResponse.id, job.id]
    );
  }

  /**
   * Cancel a fine-tuning job
   */
  async cancelJob(jobId: string): Promise<void> {
    const job = await this.getJob(jobId);
    if (!job) throw new Error('Job not found');

    if (!['pending', 'preparing', 'training'].includes(job.status)) {
      throw new Error('Cannot cancel job in current status');
    }

    // Cancel with provider if running
    if (job.providerJobId && this.openai) {
      try {
        await this.openai.fineTuning.jobs.cancel(job.providerJobId);
      } catch (error) {
        console.error('Failed to cancel with provider:', error);
      }
    }

    await this.updateJobStatus(jobId, 'cancelled');
  }

  /**
   * Sync job status with provider
   */
  async syncJobStatus(jobId: string): Promise<FineTuneJob> {
    const job = await this.getJob(jobId);
    if (!job || !job.providerJobId) {
      throw new Error('Job not found or not started');
    }

    if (!this.openai) {
      throw new Error('OpenAI client not configured');
    }

    const providerJob = await this.openai.fineTuning.jobs.retrieve(job.providerJobId);

    // Map provider status to our status
    let newStatus: FineTuneJob['status'] = job.status;
    let progress: FineTuneJob['progress'];

    switch (providerJob.status) {
      case 'validating_files':
        newStatus = 'preparing';
        break;
      case 'queued':
      case 'running':
        newStatus = 'training';
        if (providerJob.trained_tokens) {
          progress = {
            completedSteps: providerJob.trained_tokens,
          };
        }
        break;
      case 'succeeded':
        newStatus = 'completed';
        break;
      case 'failed':
        newStatus = 'failed';
        break;
      case 'cancelled':
        newStatus = 'cancelled';
        break;
    }

    // Update job
    await this.db.query(
      `UPDATE fine_tune_jobs SET
        status = $1,
        progress = COALESCE($2, progress),
        result_model_id = $3,
        completed_at = CASE WHEN $1 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END,
        updated_at = NOW()
      WHERE id = $4`,
      [
        newStatus,
        progress ? JSON.stringify(progress) : null,
        providerJob.fine_tuned_model,
        jobId,
      ]
    );

    // If completed, create a new model config for the fine-tuned model
    if (newStatus === 'completed' && providerJob.fine_tuned_model) {
      await this.createFineTunedModel(job, providerJob.fine_tuned_model);
    }

    return this.getJob(jobId) as Promise<FineTuneJob>;
  }

  /**
   * Create a model config for the fine-tuned model
   */
  private async createFineTunedModel(
    job: FineTuneJob,
    fineTunedModelId: string
  ): Promise<void> {
    // Get base model info
    const baseModelResult = await this.db.query(
      `SELECT * FROM models WHERE id = $1`,
      [job.baseModelId]
    );

    if (baseModelResult.rows.length === 0) return;

    const baseModel = baseModelResult.rows[0];
    const newModelId = uuidv4();

    await this.db.query(
      `INSERT INTO models (
        id, name, display_name, description, provider, type, model_id, version,
        status, capabilities, config, pricing, rate_limits, tags, metadata,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW(), $16)`,
      [
        newModelId,
        `${baseModel.name}-ft-${job.name}`,
        `${baseModel.display_name} (Fine-tuned: ${job.name})`,
        `Fine-tuned from ${baseModel.display_name}`,
        baseModel.provider,
        baseModel.type,
        fineTunedModelId,
        '1.0.0',
        'draft',
        JSON.stringify(baseModel.capabilities),
        JSON.stringify(baseModel.config),
        JSON.stringify(baseModel.pricing), // May need adjustment
        JSON.stringify(baseModel.rate_limits),
        JSON.stringify([...baseModel.tags, 'fine-tuned']),
        JSON.stringify({
          fineTuneJobId: job.id,
          baseModelId: job.baseModelId,
          trainingSamples: job.trainingData.samples,
        }),
        job.createdBy,
      ]
    );

    // Update job with result model ID
    await this.db.query(
      `UPDATE fine_tune_jobs SET result_model_id = $1 WHERE id = $2`,
      [newModelId, job.id]
    );
  }

  /**
   * Get training data statistics
   */
  async analyzeTrainingData(fileId: string): Promise<{
    samples: number;
    avgInputTokens: number;
    avgOutputTokens: number;
    estimatedCost: number;
    warnings: string[];
  }> {
    // In production, would analyze the actual file
    // For now, return placeholder data
    return {
      samples: 0,
      avgInputTokens: 0,
      avgOutputTokens: 0,
      estimatedCost: 0,
      warnings: [],
    };
  }

  /**
   * Validate training data format
   */
  async validateTrainingData(fileId: string, format: string): Promise<{
    valid: boolean;
    errors: string[];
    warnings: string[];
  }> {
    const errors: string[] = [];
    const warnings: string[] = [];

    // In production, would validate actual file contents
    // Check for common issues:
    // - Correct format (JSONL for OpenAI)
    // - Required fields present
    // - Message structure valid
    // - Token counts within limits

    return {
      valid: errors.length === 0,
      errors,
      warnings,
    };
  }

  /**
   * Estimate fine-tuning cost
   */
  async estimateCost(
    baseModelId: string,
    trainingData: { samples: number; avgTokensPerSample: number },
    hyperparameters: { epochs: number }
  ): Promise<{
    estimatedCost: number;
    currency: string;
    breakdown: {
      trainingCost: number;
      validationCost: number;
    };
  }> {
    // Get base model pricing
    const modelResult = await this.db.query(
      `SELECT pricing FROM models WHERE id = $1`,
      [baseModelId]
    );

    if (modelResult.rows.length === 0) {
      throw new Error('Base model not found');
    }

    const pricing = modelResult.rows[0].pricing;
    const totalTokens = trainingData.samples * trainingData.avgTokensPerSample * hyperparameters.epochs;

    // OpenAI fine-tuning pricing (approximate)
    const costPer1MTokens = 8.00; // $8 per 1M training tokens (approximate)
    const trainingCost = (totalTokens / 1_000_000) * costPer1MTokens;

    return {
      estimatedCost: trainingCost,
      currency: 'USD',
      breakdown: {
        trainingCost,
        validationCost: 0,
      },
    };
  }

  // ===========================================
  // Helpers
  // ===========================================

  private async updateJobStatus(jobId: string, status: FineTuneJob['status']): Promise<void> {
    await this.db.query(
      `UPDATE fine_tune_jobs SET status = $1, updated_at = NOW() WHERE id = $2`,
      [status, jobId]
    );
  }

  private mapJobRow(row: any): FineTuneJob {
    return {
      id: row.id,
      name: row.name,
      baseModelId: row.base_model_id,
      status: row.status,
      trainingData: row.training_data,
      hyperparameters: row.hyperparameters,
      progress: row.progress,
      resultModelId: row.result_model_id,
      providerJobId: row.provider_job_id,
      startedAt: row.started_at,
      completedAt: row.completed_at,
      estimatedCompletion: row.estimated_completion,
      estimatedCost: row.estimated_cost,
      actualCost: row.actual_cost,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
    };
  }
}
