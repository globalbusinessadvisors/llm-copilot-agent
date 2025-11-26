/**
 * HIPAA Routes
 *
 * API endpoints for HIPAA compliance features.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { HIPAAService } from '../services/hipaaService';

export function createHIPAARoutes(hipaaService: HIPAAService): Router {
  const router = Router();

  // ===========================================
  // PHI Access Logging Routes
  // ===========================================

  /**
   * Log PHI access
   */
  router.post('/phi-access', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const log = await hipaaService.logPHIAccess(req.body);
      res.status(201).json({ log });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get PHI access logs
   */
  router.get('/phi-access', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId, patientId, accessType, startDate, endDate, limit } = req.query;
      const logs = await hipaaService.getPHIAccessLogs({
        userId: userId as string,
        patientId: patientId as string,
        accessType: accessType as any,
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        limit: limit ? parseInt(limit as string, 10) : undefined,
      });
      res.json({ logs, count: logs.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Generate PHI access report
   */
  router.post('/phi-access/report', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { startDate, endDate, patientId, userId } = req.body;

      if (!startDate || !endDate) {
        return res.status(400).json({ error: 'startDate and endDate are required' });
      }

      const report = await hipaaService.generateAccessReport({
        startDate: new Date(startDate),
        endDate: new Date(endDate),
        patientId,
        userId,
      });
      res.json(report);
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Business Associate Agreement Routes
  // ===========================================

  /**
   * List BAAs
   */
  router.get('/baa', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, vendorId, expiringWithinDays } = req.query;
      const baas = await hipaaService.listBAAs({
        status: status as any,
        vendorId: vendorId as string,
        expiringWithinDays: expiringWithinDays ? parseInt(expiringWithinDays as string, 10) : undefined,
      });
      res.json({ baas, count: baas.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get BAA by ID
   */
  router.get('/baa/:baaId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const baa = await hipaaService.getBAA(req.params.baaId);
      if (!baa) {
        return res.status(404).json({ error: 'BAA not found' });
      }
      res.json({ baa });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create BAA
   */
  router.post('/baa', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const baa = await hipaaService.createBAA(req.body, userId);
      res.status(201).json({ baa });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update BAA status
   */
  router.patch('/baa/:baaId/status', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status } = req.body;
      const validStatuses = ['pending', 'active', 'expired', 'terminated'];
      if (!validStatuses.includes(status)) {
        return res.status(400).json({ error: 'Invalid status' });
      }
      const baa = await hipaaService.updateBAAStatus(req.params.baaId, status);
      res.json({ baa });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Add document to BAA
   */
  router.post('/baa/:baaId/documents', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { name, url } = req.body;
      if (!name || !url) {
        return res.status(400).json({ error: 'name and url are required' });
      }
      const baa = await hipaaService.addBAADocument(req.params.baaId, { name, url });
      res.json({ baa });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // HIPAA Assessment Routes
  // ===========================================

  /**
   * Get HIPAA control requirements
   */
  router.get('/requirements', async (_req: Request, res: Response, next: NextFunction) => {
    try {
      const requirements = hipaaService.getHIPAAControlRequirements();
      res.json({
        requirements,
        count: requirements.length,
        bySafeguard: {
          administrative: requirements.filter(r => r.safeguard === 'administrative').length,
          physical: requirements.filter(r => r.safeguard === 'physical').length,
          technical: requirements.filter(r => r.safeguard === 'technical').length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Assess HIPAA compliance
   */
  router.get('/assessment', async (_req: Request, res: Response, next: NextFunction) => {
    try {
      const assessment = await hipaaService.assessHIPAACompliance();
      res.json(assessment);
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Breach Reporting Routes
  // ===========================================

  /**
   * Report a breach
   */
  router.post('/breaches', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const {
        discoveryDate,
        affectedIndividuals,
        phiTypes,
        description,
        containmentActions,
      } = req.body;

      if (!discoveryDate || !affectedIndividuals || !phiTypes || !description || !containmentActions) {
        return res.status(400).json({
          error: 'discoveryDate, affectedIndividuals, phiTypes, description, and containmentActions are required',
        });
      }

      const reportedBy = (req as any).user?.id || 'system';

      const breach = await hipaaService.reportBreach({
        discoveryDate: new Date(discoveryDate),
        affectedIndividuals,
        phiTypes,
        description,
        containmentActions,
        reportedBy,
      });

      res.status(201).json({ breach });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
