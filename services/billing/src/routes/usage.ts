/**
 * Usage API Routes
 *
 * REST endpoints for usage tracking, metering, and reporting.
 */

import { Router, Response } from 'express';
import { z } from 'zod';
import { UsageService } from '../services/usageService';
import { AuthenticatedRequest, authenticate, requireTenantAccess, authenticateInternal } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import { ValidationError } from '../utils/errors';
import { UsageType, UsageUnit, CreateUsageEventInput } from '../models/usage';

export function createUsageRouter(usageService: UsageService): Router {
  const router = Router();

  // Schema definitions
  const RecordUsageSchema = z.object({
    tenantId: z.string().uuid(),
    userId: z.string().uuid().optional(),
    type: z.nativeEnum(UsageType),
    unit: z.nativeEnum(UsageUnit),
    quantity: z.number().positive(),
    metadata: z.record(z.unknown()).optional(),
    resourceId: z.string().optional(),
    resourceType: z.string().optional(),
    model: z.string().optional(),
    endpoint: z.string().optional(),
    statusCode: z.number().optional(),
  });

  const BatchUsageSchema = z.object({
    events: z.array(RecordUsageSchema).min(1).max(100),
  });

  const UsageQuerySchema = z.object({
    startDate: z.string().transform((s) => new Date(s)),
    endDate: z.string().transform((s) => new Date(s)),
    userId: z.string().uuid().optional(),
    type: z.nativeEnum(UsageType).optional(),
    groupBy: z.enum(['hour', 'day', 'week', 'month']).optional(),
  });

  /**
   * Record a single usage event
   * POST /usage
   * Used internally by other services to track usage
   */
  router.post(
    '/',
    authenticateInternal,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const validationResult = RecordUsageSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid usage event', {
          errors: validationResult.error.errors,
        });
      }

      const event = await usageService.recordUsage(validationResult.data);

      res.status(201).json({
        success: true,
        data: event,
      });
    })
  );

  /**
   * Record multiple usage events in batch
   * POST /usage/batch
   */
  router.post(
    '/batch',
    authenticateInternal,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const validationResult = BatchUsageSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid batch usage data', {
          errors: validationResult.error.errors,
        });
      }

      const events = await usageService.recordUsageBatch(validationResult.data.events);

      res.status(201).json({
        success: true,
        data: { recorded: events.length },
      });
    })
  );

  /**
   * Get usage summary for a tenant
   * GET /usage/summary/:tenantId
   */
  router.get(
    '/summary/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const queryResult = UsageQuerySchema.safeParse(req.query);

      if (!queryResult.success) {
        throw new ValidationError('Invalid query parameters', {
          errors: queryResult.error.errors,
        });
      }

      const { startDate, endDate } = queryResult.data;
      const summary = await usageService.getUsageSummary(tenantId, startDate, endDate);

      res.json({
        success: true,
        data: summary,
      });
    })
  );

  /**
   * Get usage aggregations with breakdown
   * GET /usage/aggregations/:tenantId
   */
  router.get(
    '/aggregations/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const queryResult = UsageQuerySchema.safeParse(req.query);

      if (!queryResult.success) {
        throw new ValidationError('Invalid query parameters', {
          errors: queryResult.error.errors,
        });
      }

      const aggregations = await usageService.getUsageAggregations({
        tenantId,
        ...queryResult.data,
      });

      res.json({
        success: true,
        data: aggregations,
      });
    })
  );

  /**
   * Get detailed usage report for a tenant
   * GET /usage/report/:tenantId
   */
  router.get(
    '/report/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const queryResult = UsageQuerySchema.safeParse(req.query);

      if (!queryResult.success) {
        throw new ValidationError('Invalid query parameters', {
          errors: queryResult.error.errors,
        });
      }

      const { startDate, endDate } = queryResult.data;
      const report = await usageService.getTenantUsageReport(tenantId, startDate, endDate);

      res.json({
        success: true,
        data: report,
      });
    })
  );

  /**
   * Get current quota for a tenant
   * GET /usage/quota/:tenantId
   */
  router.get(
    '/quota/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const quota = await usageService.getQuota(tenantId);

      res.json({
        success: true,
        data: quota,
      });
    })
  );

  /**
   * Check if quota is exceeded
   * GET /usage/quota-check/:tenantId
   */
  router.get(
    '/quota-check/:tenantId',
    authenticateInternal,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const result = await usageService.checkQuotaExceeded(tenantId);

      res.json({
        success: true,
        data: result,
      });
    })
  );

  /**
   * Get real-time usage counters
   * GET /usage/realtime/:tenantId
   */
  router.get(
    '/realtime/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const usage = await usageService.getRealtimeUsage(tenantId);

      res.json({
        success: true,
        data: usage,
      });
    })
  );

  /**
   * Set quota for a tenant (admin only)
   * PUT /usage/quota/:tenantId
   */
  router.put(
    '/quota/:tenantId',
    authenticateInternal,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const quotaSchema = z.object({
        apiCallsLimit: z.number().nullable(),
        inputTokensLimit: z.number().nullable(),
        outputTokensLimit: z.number().nullable(),
        storageBytesLimit: z.number().nullable(),
        computeSecondsLimit: z.number().nullable(),
        periodStart: z.string().transform((s) => new Date(s)),
        periodEnd: z.string().transform((s) => new Date(s)),
      });

      const validationResult = quotaSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid quota data', {
          errors: validationResult.error.errors,
        });
      }

      await usageService.setQuota({
        tenantId,
        ...validationResult.data,
      });

      res.json({
        success: true,
        message: 'Quota updated successfully',
      });
    })
  );

  return router;
}
