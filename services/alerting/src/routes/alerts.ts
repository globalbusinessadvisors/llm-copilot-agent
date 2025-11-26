/**
 * Alert Routes
 *
 * REST API endpoints for alert management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { AlertService } from '../services/alertService';
import { AlertSeverity, AlertStatus, CreateAlertInput } from '../models/alert';

const router = Router();

// Request validation schemas
const CreateAlertSchema = z.object({
  ruleId: z.string().uuid(),
  title: z.string().min(1).max(255),
  description: z.string().min(1),
  severity: z.nativeEnum(AlertSeverity),
  source: z.string().min(1),
  tags: z.record(z.string()).optional(),
  metadata: z.record(z.unknown()).optional(),
});

const QueryAlertsSchema = z.object({
  status: z.nativeEnum(AlertStatus).optional(),
  severity: z.nativeEnum(AlertSeverity).optional(),
  source: z.string().optional(),
  limit: z.coerce.number().min(1).max(1000).default(100),
  offset: z.coerce.number().min(0).default(0),
});

export function createAlertRoutes(alertService: AlertService): Router {
  /**
   * Create a new alert
   * POST /api/v1/alerts
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateAlertSchema.parse(req.body);
      const alert = await alertService.createAlert(input as CreateAlertInput);

      res.status(201).json({
        success: true,
        data: alert,
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Get alert by ID
   * GET /api/v1/alerts/:alertId
   */
  router.get('/:alertId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { alertId } = req.params;
      const alert = await alertService.getAlert(alertId);

      if (!alert) {
        res.status(404).json({
          success: false,
          error: 'Alert not found',
        });
        return;
      }

      res.json({
        success: true,
        data: alert,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get active alerts
   * GET /api/v1/alerts/active
   */
  router.get('/status/active', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const alerts = await alertService.getActiveAlerts();

      res.json({
        success: true,
        data: alerts,
        meta: {
          total: alerts.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get alert history
   * GET /api/v1/alerts/history
   */
  router.get('/history/all', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const query = QueryAlertsSchema.parse(req.query);
      const alerts = await alertService.getAlertHistory(query.limit);

      res.json({
        success: true,
        data: alerts,
        meta: {
          limit: query.limit,
          offset: query.offset,
          total: alerts.length,
        },
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        res.status(400).json({
          success: false,
          error: 'Validation error',
          details: error.errors,
        });
        return;
      }
      next(error);
    }
  });

  /**
   * Get alert summary
   * GET /api/v1/alerts/summary
   */
  router.get('/stats/summary', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const summary = await alertService.getAlertSummary();

      res.json({
        success: true,
        data: summary,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Acknowledge an alert
   * POST /api/v1/alerts/:alertId/acknowledge
   */
  router.post('/:alertId/acknowledge', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { alertId } = req.params;
      const userId = req.body.userId || (req as any).user?.id || 'system';

      const alert = await alertService.acknowledgeAlert(alertId, userId);

      if (!alert) {
        res.status(404).json({
          success: false,
          error: 'Alert not found or already acknowledged/resolved',
        });
        return;
      }

      res.json({
        success: true,
        data: alert,
        message: 'Alert acknowledged successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Resolve an alert
   * POST /api/v1/alerts/:alertId/resolve
   */
  router.post('/:alertId/resolve', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { alertId } = req.params;
      const userId = req.body.userId || (req as any).user?.id || 'system';

      const alert = await alertService.resolveAlert(alertId, userId);

      if (!alert) {
        res.status(404).json({
          success: false,
          error: 'Alert not found or already resolved',
        });
        return;
      }

      res.json({
        success: true,
        data: alert,
        message: 'Alert resolved successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Process escalation for an alert
   * POST /api/v1/alerts/:alertId/escalate
   */
  router.post('/:alertId/escalate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { alertId } = req.params;

      await alertService.processEscalation(alertId);

      res.json({
        success: true,
        message: 'Escalation processed',
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
