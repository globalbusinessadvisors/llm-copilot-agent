/**
 * Compliance Routes
 *
 * API endpoints for compliance controls, audits, and findings.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { ComplianceService } from '../services/complianceService';
import {
  ComplianceFramework,
  ControlCategory,
  ControlStatus,
  AuditStatus,
  FindingSeverity,
  FindingStatus,
} from '../models/compliance';

export function createComplianceRoutes(complianceService: ComplianceService): Router {
  const router = Router();

  // ===========================================
  // Control Routes
  // ===========================================

  /**
   * List controls
   */
  router.get('/controls', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { framework, category, status, owner } = req.query;
      const controls = await complianceService.listControls({
        framework: framework as ComplianceFramework,
        category: category as ControlCategory,
        status: status as ControlStatus,
        owner: owner as string,
      });
      res.json({ controls });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get control by ID
   */
  router.get('/controls/:controlId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const control = await complianceService.getControl(req.params.controlId);
      if (!control) {
        return res.status(404).json({ error: 'Control not found' });
      }
      res.json({ control });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create control
   */
  router.post('/controls', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const control = await complianceService.createControl(req.body, userId);
      res.status(201).json({ control });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update control status
   */
  router.patch('/controls/:controlId/status', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, evidence } = req.body;
      if (!Object.values(ControlStatus).includes(status)) {
        return res.status(400).json({ error: 'Invalid status' });
      }
      const control = await complianceService.updateControlStatus(
        req.params.controlId,
        status,
        evidence
      );
      res.json({ control });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Record control test
   */
  router.post('/controls/:controlId/test', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const control = await complianceService.recordControlTest(
        req.params.controlId,
        req.body
      );
      res.json({ control });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Audit Routes
  // ===========================================

  /**
   * List audits
   */
  router.get('/audits', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { framework, status, type } = req.query;
      const audits = await complianceService.listAudits({
        framework: framework as ComplianceFramework,
        status: status as AuditStatus,
        type: type as any,
      });
      res.json({ audits });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get audit by ID
   */
  router.get('/audits/:auditId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const audit = await complianceService.getAudit(req.params.auditId);
      if (!audit) {
        return res.status(404).json({ error: 'Audit not found' });
      }
      res.json({ audit });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create audit
   */
  router.post('/audits', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const audit = await complianceService.createAudit(req.body, userId);
      res.status(201).json({ audit });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update audit status
   */
  router.patch('/audits/:auditId/status', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status } = req.body;
      if (!Object.values(AuditStatus).includes(status)) {
        return res.status(400).json({ error: 'Invalid status' });
      }
      const audit = await complianceService.updateAuditStatus(req.params.auditId, status);
      res.json({ audit });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Finding Routes
  // ===========================================

  /**
   * List findings
   */
  router.get('/findings', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { auditId, controlId, severity, status } = req.query;
      const findings = await complianceService.listFindings({
        auditId: auditId as string,
        controlId: controlId as string,
        severity: severity as FindingSeverity,
        status: status as FindingStatus,
      });
      res.json({ findings });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get finding by ID
   */
  router.get('/findings/:findingId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const finding = await complianceService.getFinding(req.params.findingId);
      if (!finding) {
        return res.status(404).json({ error: 'Finding not found' });
      }
      res.json({ finding });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create finding
   */
  router.post('/findings', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const finding = await complianceService.createFinding(req.body, userId);
      res.status(201).json({ finding });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update finding status
   */
  router.patch('/findings/:findingId/status', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const { status, comment } = req.body;
      if (!Object.values(FindingStatus).includes(status)) {
        return res.status(400).json({ error: 'Invalid status' });
      }
      const finding = await complianceService.updateFindingStatus(
        req.params.findingId,
        status,
        userId,
        comment
      );
      res.json({ finding });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Reporting Routes
  // ===========================================

  /**
   * Generate compliance report
   */
  router.post('/reports', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const report = await complianceService.generateReport(req.body, userId);
      res.status(201).json({ report });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get dashboard metrics
   */
  router.get('/dashboard', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { framework } = req.query;
      const metrics = await complianceService.getDashboardMetrics(
        framework as ComplianceFramework
      );
      res.json(metrics);
    } catch (error) {
      next(error);
    }
  });

  return router;
}
