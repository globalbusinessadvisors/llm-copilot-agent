/**
 * On-Call Routes
 *
 * REST API endpoints for on-call schedule and user management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { OnCallService } from '../services/onCallService';
import { AlertChannel } from '../models/alert';

const router = Router();

// Request validation schemas
const CreateScheduleSchema = z.object({
  name: z.string().min(1).max(255),
  description: z.string().optional(),
  timezone: z.string().default('UTC'),
  rotations: z.array(z.object({
    name: z.string().min(1),
    type: z.enum(['daily', 'weekly', 'custom']),
    startTime: z.string().regex(/^\d{2}:\d{2}$/),
    handoffTime: z.string().regex(/^\d{2}:\d{2}$/),
    users: z.array(z.string().uuid()).min(1),
    restrictions: z.array(z.object({
      type: z.enum(['time_of_day', 'day_of_week']),
      startTime: z.string().optional(),
      endTime: z.string().optional(),
      daysOfWeek: z.array(z.number().min(0).max(6)).optional(),
    })).optional(),
  })).min(1),
});

const CreateOverrideSchema = z.object({
  scheduleId: z.string().uuid(),
  userId: z.string().uuid(),
  startAt: z.coerce.date(),
  endAt: z.coerce.date(),
  reason: z.string().optional(),
});

const CreateUserSchema = z.object({
  name: z.string().min(1).max(255),
  email: z.string().email(),
  phone: z.string().optional(),
  slackUserId: z.string().optional(),
  notificationPreferences: z.object({
    email: z.boolean().default(true),
    slack: z.boolean().default(true),
    sms: z.boolean().default(false),
    phone: z.boolean().default(false),
  }).optional(),
});

export function createOnCallRoutes(onCallService: OnCallService): Router {
  // ===========================================
  // Schedule Management
  // ===========================================

  /**
   * Create a new on-call schedule
   * POST /api/v1/on-call/schedules
   */
  router.post('/schedules', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateScheduleSchema.parse(req.body);
      const schedule = await onCallService.createSchedule(input);

      res.status(201).json({
        success: true,
        data: schedule,
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
   * Get all on-call schedules
   * GET /api/v1/on-call/schedules
   */
  router.get('/schedules', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const schedules = await onCallService.getSchedules();

      res.json({
        success: true,
        data: schedules,
        meta: {
          total: schedules.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get schedule by ID
   * GET /api/v1/on-call/schedules/:scheduleId
   */
  router.get('/schedules/:scheduleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { scheduleId } = req.params;
      const schedule = await onCallService.getSchedule(scheduleId);

      if (!schedule) {
        res.status(404).json({
          success: false,
          error: 'Schedule not found',
        });
        return;
      }

      res.json({
        success: true,
        data: schedule,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update schedule
   * PUT /api/v1/on-call/schedules/:scheduleId
   */
  router.put('/schedules/:scheduleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { scheduleId } = req.params;
      const input = CreateScheduleSchema.partial().parse(req.body);

      const schedule = await onCallService.updateSchedule(scheduleId, input);

      if (!schedule) {
        res.status(404).json({
          success: false,
          error: 'Schedule not found',
        });
        return;
      }

      res.json({
        success: true,
        data: schedule,
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
   * Delete schedule
   * DELETE /api/v1/on-call/schedules/:scheduleId
   */
  router.delete('/schedules/:scheduleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { scheduleId } = req.params;
      const deleted = await onCallService.deleteSchedule(scheduleId);

      if (!deleted) {
        res.status(404).json({
          success: false,
          error: 'Schedule not found',
        });
        return;
      }

      res.json({
        success: true,
        message: 'Schedule deleted',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get current on-call for a schedule
   * GET /api/v1/on-call/schedules/:scheduleId/current
   */
  router.get('/schedules/:scheduleId/current', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { scheduleId } = req.params;
      const current = await onCallService.getCurrentOnCall(scheduleId);

      if (!current) {
        res.status(404).json({
          success: false,
          error: 'No current on-call found for this schedule',
        });
        return;
      }

      res.json({
        success: true,
        data: current,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get all current on-calls
   * GET /api/v1/on-call/current
   */
  router.get('/current', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const currentOnCalls = await onCallService.getAllCurrentOnCalls();

      res.json({
        success: true,
        data: currentOnCalls,
        meta: {
          total: currentOnCalls.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Override Management
  // ===========================================

  /**
   * Create an on-call override
   * POST /api/v1/on-call/overrides
   */
  router.post('/overrides', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateOverrideSchema.parse(req.body);

      // Validate dates
      if (input.endAt <= input.startAt) {
        res.status(400).json({
          success: false,
          error: 'End time must be after start time',
        });
        return;
      }

      const override = await onCallService.createOverride(input);

      res.status(201).json({
        success: true,
        data: override,
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
   * Get active overrides for a schedule
   * GET /api/v1/on-call/schedules/:scheduleId/overrides
   */
  router.get('/schedules/:scheduleId/overrides', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { scheduleId } = req.params;
      const overrides = await onCallService.getActiveOverrides(scheduleId);

      res.json({
        success: true,
        data: overrides,
        meta: {
          total: overrides.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete an override
   * DELETE /api/v1/on-call/overrides/:overrideId
   */
  router.delete('/overrides/:overrideId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { overrideId } = req.params;
      const deleted = await onCallService.deleteOverride(overrideId);

      if (!deleted) {
        res.status(404).json({
          success: false,
          error: 'Override not found',
        });
        return;
      }

      res.json({
        success: true,
        message: 'Override deleted',
      });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // User Management
  // ===========================================

  /**
   * Create an on-call user
   * POST /api/v1/on-call/users
   */
  router.post('/users', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateUserSchema.parse(req.body);
      const user = await onCallService.createUser(input);

      res.status(201).json({
        success: true,
        data: user,
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
   * Get all on-call users
   * GET /api/v1/on-call/users
   */
  router.get('/users', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const users = await onCallService.getUsers();

      res.json({
        success: true,
        data: users,
        meta: {
          total: users.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get user by ID
   * GET /api/v1/on-call/users/:userId
   */
  router.get('/users/:userId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId } = req.params;
      const user = await onCallService.getUser(userId);

      if (!user) {
        res.status(404).json({
          success: false,
          error: 'User not found',
        });
        return;
      }

      res.json({
        success: true,
        data: user,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update user
   * PUT /api/v1/on-call/users/:userId
   */
  router.put('/users/:userId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId } = req.params;
      const input = CreateUserSchema.partial().parse(req.body);

      const user = await onCallService.updateUser(userId, input);

      if (!user) {
        res.status(404).json({
          success: false,
          error: 'User not found',
        });
        return;
      }

      res.json({
        success: true,
        data: user,
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
   * Delete user
   * DELETE /api/v1/on-call/users/:userId
   */
  router.delete('/users/:userId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId } = req.params;
      const deleted = await onCallService.deleteUser(userId);

      if (!deleted) {
        res.status(404).json({
          success: false,
          error: 'User not found',
        });
        return;
      }

      res.json({
        success: true,
        message: 'User deleted',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get user's on-call shifts
   * GET /api/v1/on-call/users/:userId/shifts
   */
  router.get('/users/:userId/shifts', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId } = req.params;
      const { startDate, endDate } = req.query;

      const start = startDate ? new Date(startDate as string) : new Date();
      const end = endDate ? new Date(endDate as string) : new Date(Date.now() + 30 * 24 * 60 * 60 * 1000);

      const shifts = await onCallService.getUserShifts(userId, start, end);

      res.json({
        success: true,
        data: shifts,
        meta: {
          total: shifts.length,
          startDate: start.toISOString(),
          endDate: end.toISOString(),
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
