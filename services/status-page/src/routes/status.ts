/**
 * Status Routes
 *
 * REST API endpoints for public status page.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { StatusService } from '../services/statusService';
import { ServiceStatus, IncidentSeverity, IncidentStatus, MaintenanceStatus } from '../models/status';

const router = Router();

// Request validation schemas
const CreateServiceSchema = z.object({
  name: z.string().min(1).max(255),
  slug: z.string().max(100).optional(),
  description: z.string().optional(),
  group: z.string().max(100).optional(),
  order: z.number().optional(),
  isPublic: z.boolean().optional(),
  healthCheckUrl: z.string().url().optional(),
  healthCheckInterval: z.number().min(30).max(3600).optional(),
});

const CreateIncidentSchema = z.object({
  title: z.string().min(1).max(255),
  description: z.string().min(1),
  severity: z.nativeEnum(IncidentSeverity),
  affectedServices: z.array(z.string().uuid()).min(1),
});

const UpdateIncidentSchema = z.object({
  status: z.nativeEnum(IncidentStatus),
  message: z.string().min(1),
});

const CreateMaintenanceSchema = z.object({
  title: z.string().min(1).max(255),
  description: z.string().min(1),
  affectedServices: z.array(z.string().uuid()).min(1),
  scheduledStartAt: z.coerce.date(),
  scheduledEndAt: z.coerce.date(),
});

export function createStatusRoutes(statusService: StatusService): Router {
  // ===========================================
  // Public Status Endpoints
  // ===========================================

  /**
   * Get overall status summary (public)
   * GET /api/v1/status
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const summary = await statusService.getStatusSummary();

      res.json({
        success: true,
        data: summary,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get all services (public)
   * GET /api/v1/status/services
   */
  router.get('/services', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const services = await statusService.getServices();

      res.json({
        success: true,
        data: services,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get active incidents (public)
   * GET /api/v1/status/incidents
   */
  router.get('/incidents', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const incidents = await statusService.getActiveIncidents();

      res.json({
        success: true,
        data: incidents,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get incident history (public)
   * GET /api/v1/status/incidents/history
   */
  router.get('/incidents/history', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const limit = parseInt(req.query.limit as string, 10) || 20;
      const incidents = await statusService.getIncidentHistory(limit);

      res.json({
        success: true,
        data: incidents,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get incident by ID (public)
   * GET /api/v1/status/incidents/:incidentId
   */
  router.get('/incidents/:incidentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { incidentId } = req.params;
      const incident = await statusService.getIncident(incidentId);

      if (!incident) {
        res.status(404).json({
          success: false,
          error: 'Incident not found',
        });
        return;
      }

      const updates = await statusService.getIncidentUpdates(incidentId);

      res.json({
        success: true,
        data: {
          ...incident,
          updates,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get scheduled maintenance (public)
   * GET /api/v1/status/maintenance
   */
  router.get('/maintenance', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const maintenance = await statusService.getScheduledMaintenance();

      res.json({
        success: true,
        data: maintenance,
      });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Admin Endpoints
  // ===========================================

  /**
   * Create a new service (admin)
   * POST /api/v1/status/admin/services
   */
  router.post('/admin/services', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateServiceSchema.parse(req.body);
      const service = await statusService.createService(input);

      res.status(201).json({
        success: true,
        data: service,
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
   * Update service status (admin)
   * PATCH /api/v1/status/admin/services/:serviceId
   */
  router.patch('/admin/services/:serviceId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { serviceId } = req.params;
      const { status } = req.body;

      if (!Object.values(ServiceStatus).includes(status)) {
        res.status(400).json({
          success: false,
          error: 'Invalid status',
        });
        return;
      }

      await statusService.updateServiceStatus(serviceId, status);

      res.json({
        success: true,
        message: 'Service status updated',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create a new incident (admin)
   * POST /api/v1/status/admin/incidents
   */
  router.post('/admin/incidents', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateIncidentSchema.parse(req.body);
      const incident = await statusService.createIncident(input);

      res.status(201).json({
        success: true,
        data: incident,
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
   * Update incident (admin)
   * POST /api/v1/status/admin/incidents/:incidentId/update
   */
  router.post('/admin/incidents/:incidentId/update', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { incidentId } = req.params;
      const input = UpdateIncidentSchema.parse(req.body);
      const createdBy = (req as any).user?.email || req.body.createdBy || 'System';

      const incident = await statusService.updateIncident(
        incidentId,
        input.status,
        input.message,
        createdBy
      );

      if (!incident) {
        res.status(404).json({
          success: false,
          error: 'Incident not found',
        });
        return;
      }

      res.json({
        success: true,
        data: incident,
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
   * Schedule maintenance (admin)
   * POST /api/v1/status/admin/maintenance
   */
  router.post('/admin/maintenance', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateMaintenanceSchema.parse(req.body);

      // Validate dates
      if (input.scheduledEndAt <= input.scheduledStartAt) {
        res.status(400).json({
          success: false,
          error: 'End time must be after start time',
        });
        return;
      }

      const maintenance = await statusService.scheduleMaintenance(input);

      res.status(201).json({
        success: true,
        data: maintenance,
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
   * Trigger health check for a service (admin)
   * POST /api/v1/status/admin/services/:serviceId/healthcheck
   */
  router.post('/admin/services/:serviceId/healthcheck', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { serviceId } = req.params;
      const services = await statusService.getServices();
      const service = services.find(s => s.id === serviceId);

      if (!service) {
        res.status(404).json({
          success: false,
          error: 'Service not found',
        });
        return;
      }

      const result = await statusService.healthCheck(service);

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
