/**
 * Data Residency Routes
 *
 * API endpoints for data residency policies, assets, and transfers.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { DataResidencyService } from '../services/dataResidencyService';
import { DataClassification, DataRegion } from '../models/compliance';

export function createDataResidencyRoutes(dataResidencyService: DataResidencyService): Router {
  const router = Router();

  // ===========================================
  // Policy Routes
  // ===========================================

  /**
   * List policies
   */
  router.get('/policies', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { classification, status, region } = req.query;
      const policies = await dataResidencyService.listPolicies({
        classification: classification as DataClassification,
        status: status as any,
        region: region as DataRegion,
      });
      res.json({ policies, count: policies.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get policy by ID
   */
  router.get('/policies/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await dataResidencyService.getPolicy(req.params.policyId);
      if (!policy) {
        return res.status(404).json({ error: 'Policy not found' });
      }
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create policy
   */
  router.post('/policies', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const policy = await dataResidencyService.createPolicy(req.body, userId);
      res.status(201).json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Activate policy
   */
  router.post('/policies/:policyId/activate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await dataResidencyService.activatePolicy(req.params.policyId);
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Data Asset Routes
  // ===========================================

  /**
   * List data assets
   */
  router.get('/assets', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { classification, region, type } = req.query;
      const assets = await dataResidencyService.listDataAssets({
        classification: classification as DataClassification,
        region: region as DataRegion,
        type: type as string,
      });
      res.json({ assets, count: assets.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get data asset by ID
   */
  router.get('/assets/:assetId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const asset = await dataResidencyService.getDataAsset(req.params.assetId);
      if (!asset) {
        return res.status(404).json({ error: 'Asset not found' });
      }
      res.json({ asset });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Register data asset
   */
  router.post('/assets', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const asset = await dataResidencyService.registerDataAsset(req.body, userId);
      res.status(201).json({ asset });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Check asset compliance
   */
  router.get('/assets/:assetId/compliance', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const compliance = await dataResidencyService.checkAssetCompliance(req.params.assetId);
      res.json(compliance);
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Data Transfer Routes
  // ===========================================

  /**
   * List transfer requests
   */
  router.get('/transfers', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { assetId, status, sourceRegion, targetRegion } = req.query;
      const transfers = await dataResidencyService.listTransferRequests({
        assetId: assetId as string,
        status: status as any,
        sourceRegion: sourceRegion as DataRegion,
        targetRegion: targetRegion as DataRegion,
      });
      res.json({ transfers, count: transfers.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get transfer request by ID
   */
  router.get('/transfers/:requestId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const request = await dataResidencyService.getTransferRequest(req.params.requestId);
      if (!request) {
        return res.status(404).json({ error: 'Transfer request not found' });
      }
      res.json({ request });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Request data transfer
   */
  router.post('/transfers', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const { assetId, targetRegion, purpose, transferMechanism, dpaReference } = req.body;

      if (!assetId || !targetRegion || !purpose) {
        return res.status(400).json({
          error: 'assetId, targetRegion, and purpose are required',
        });
      }

      if (!Object.values(DataRegion).includes(targetRegion)) {
        return res.status(400).json({ error: 'Invalid target region' });
      }

      const request = await dataResidencyService.requestDataTransfer(
        { assetId, targetRegion, purpose, transferMechanism, dpaReference },
        userId
      );
      res.status(201).json({ request });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Approve transfer request
   */
  router.post('/transfers/:requestId/approve', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const approverId = (req as any).user?.id || 'system';
      const { notes } = req.body;
      const request = await dataResidencyService.approveTransferRequest(
        req.params.requestId,
        approverId,
        notes
      );
      res.json({ request });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Execute approved transfer
   */
  router.post('/transfers/:requestId/execute', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const request = await dataResidencyService.executeTransfer(req.params.requestId);
      res.json({ request });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Reporting Routes
  // ===========================================

  /**
   * Generate data residency report
   */
  router.get('/report', async (_req: Request, res: Response, next: NextFunction) => {
    try {
      const report = await dataResidencyService.generateReport();
      res.json(report);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get available regions
   */
  router.get('/regions', (_req: Request, res: Response) => {
    res.json({
      regions: Object.values(DataRegion),
      classifications: Object.values(DataClassification),
    });
  });

  return router;
}
