/**
 * Model Management Routes
 *
 * API endpoints for model configuration, versioning, and deployment.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { ModelService } from '../services/modelService';
import { ABTestService } from '../services/abTestService';
import { FineTuneService } from '../services/fineTuneService';
import { ModelStatus, DeploymentStrategy } from '../models/model';

export function createModelRoutes(
  modelService: ModelService,
  abTestService: ABTestService,
  fineTuneService: FineTuneService
): Router {
  const router = Router();

  // ===========================================
  // Model Configuration Routes
  // ===========================================

  /**
   * List models
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, provider, type } = req.query;
      const models = await modelService.listModels({
        status: status as ModelStatus,
        provider: provider as string,
        type: type as string,
      });
      res.json({ models });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get model by ID
   */
  router.get('/:modelId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const model = await modelService.getModel(req.params.modelId);
      if (!model) {
        return res.status(404).json({ error: 'Model not found' });
      }
      res.json({ model });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create model
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const model = await modelService.createModel(req.body, userId);
      res.status(201).json({ model });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update model status
   */
  router.patch('/:modelId/status', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status } = req.body;
      if (!Object.values(ModelStatus).includes(status)) {
        return res.status(400).json({ error: 'Invalid status' });
      }
      await modelService.updateModelStatus(req.params.modelId, status);
      const model = await modelService.getModel(req.params.modelId);
      res.json({ model });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get model metrics
   */
  router.get('/:modelId/metrics', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { startDate, endDate, granularity } = req.query;
      const metrics = await modelService.getMetrics(req.params.modelId, {
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        granularity: granularity as 'hour' | 'day' | 'week',
      });
      res.json({ metrics });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Record model metrics
   */
  router.post('/:modelId/metrics', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await modelService.recordMetrics(req.params.modelId, req.body);
      res.status(201).json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Version Management Routes
  // ===========================================

  /**
   * List model versions
   */
  router.get('/:modelId/versions', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const versions = await modelService.listVersions(req.params.modelId);
      res.json({ versions });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create model version
   */
  router.post('/:modelId/versions', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const { version, changelog, config } = req.body;
      const modelVersion = await modelService.createVersion(
        req.params.modelId,
        version,
        userId,
        { changelog, config }
      );
      res.status(201).json({ version: modelVersion });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get model version
   */
  router.get('/:modelId/versions/:versionId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const version = await modelService.getVersion(req.params.versionId);
      if (!version) {
        return res.status(404).json({ error: 'Version not found' });
      }
      res.json({ version });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Deployment Routes
  // ===========================================

  /**
   * List deployments
   */
  router.get('/:modelId/deployments', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const deployments = await modelService.listDeployments(req.params.modelId);
      res.json({ deployments });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create deployment
   */
  router.post('/:modelId/deployments', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const { versionId, strategy, trafficPercentage, rolloutConfig } = req.body;

      if (strategy && !Object.values(DeploymentStrategy).includes(strategy)) {
        return res.status(400).json({ error: 'Invalid deployment strategy' });
      }

      const deployment = await modelService.createDeployment(
        req.params.modelId,
        versionId,
        { strategy, trafficPercentage, rolloutConfig },
        userId
      );
      res.status(201).json({ deployment });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Rollback deployment
   */
  router.post('/deployments/:deploymentId/rollback', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      await modelService.rollbackDeployment(req.params.deploymentId, userId);
      res.json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // A/B Test Routes
  // ===========================================

  /**
   * List A/B tests
   */
  router.get('/:modelId/ab-tests', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const tests = await abTestService.listTests({ modelId: req.params.modelId });
      res.json({ tests });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create A/B test
   */
  router.post('/:modelId/ab-tests', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const test = await abTestService.createTest(
        { ...req.body, modelId: req.params.modelId },
        userId
      );
      res.status(201).json({ test });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get A/B test
   */
  router.get('/ab-tests/:testId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const test = await abTestService.getTest(req.params.testId);
      if (!test) {
        return res.status(404).json({ error: 'Test not found' });
      }
      res.json({ test });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Start A/B test
   */
  router.post('/ab-tests/:testId/start', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await abTestService.startTest(req.params.testId);
      const test = await abTestService.getTest(req.params.testId);
      res.json({ test });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Stop A/B test
   */
  router.post('/ab-tests/:testId/stop', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await abTestService.stopTest(req.params.testId);
      const test = await abTestService.getTest(req.params.testId);
      res.json({ test });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get A/B test results
   */
  router.get('/ab-tests/:testId/results', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const results = await abTestService.getResults(req.params.testId);
      res.json({ results });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Assign variant
   */
  router.post('/ab-tests/:testId/assign', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const assignment = await abTestService.assignVariant(req.params.testId, req.body);
      if (!assignment) {
        return res.status(400).json({ error: 'Could not assign variant' });
      }
      res.json({ assignment });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Record sample
   */
  router.post('/ab-tests/:testId/samples', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { variantId, metrics } = req.body;
      await abTestService.recordSample(req.params.testId, variantId, metrics);
      res.status(201).json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Fine-Tuning Routes
  // ===========================================

  /**
   * List fine-tune jobs
   */
  router.get('/fine-tune/jobs', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, baseModelId } = req.query;
      const jobs = await fineTuneService.listJobs({
        status: status as any,
        baseModelId: baseModelId as string,
      });
      res.json({ jobs });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create fine-tune job
   */
  router.post('/fine-tune/jobs', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const job = await fineTuneService.createJob(req.body, userId);
      res.status(201).json({ job });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get fine-tune job
   */
  router.get('/fine-tune/jobs/:jobId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const job = await fineTuneService.getJob(req.params.jobId);
      if (!job) {
        return res.status(404).json({ error: 'Job not found' });
      }
      res.json({ job });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Start fine-tune job
   */
  router.post('/fine-tune/jobs/:jobId/start', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const job = await fineTuneService.startJob(req.params.jobId);
      res.json({ job });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Cancel fine-tune job
   */
  router.post('/fine-tune/jobs/:jobId/cancel', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await fineTuneService.cancelJob(req.params.jobId);
      res.json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Sync fine-tune job status
   */
  router.post('/fine-tune/jobs/:jobId/sync', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const job = await fineTuneService.syncJobStatus(req.params.jobId);
      res.json({ job });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Estimate fine-tune cost
   */
  router.post('/fine-tune/estimate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { baseModelId, trainingData, hyperparameters } = req.body;
      const estimate = await fineTuneService.estimateCost(
        baseModelId,
        trainingData,
        hyperparameters
      );
      res.json({ estimate });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
