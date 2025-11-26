/**
 * Subscription API Routes
 *
 * REST endpoints for subscription management, plan changes, and billing.
 */

import { Router, Response } from 'express';
import { z } from 'zod';
import { StripeService } from '../services/stripeService';
import { AuthenticatedRequest, authenticate, requireTenantAccess, requireAdmin } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import { ValidationError, NotFoundError } from '../utils/errors';
import { PlanType, PLANS } from '../models/subscription';

export function createSubscriptionsRouter(stripeService: StripeService): Router {
  const router = Router();

  // Schema definitions
  const CreateSubscriptionSchema = z.object({
    tenantId: z.string().uuid(),
    planType: z.nativeEnum(PlanType),
    paymentMethodId: z.string().optional(),
    trialDays: z.number().min(0).max(30).optional(),
  });

  const UpdateSubscriptionSchema = z.object({
    planType: z.nativeEnum(PlanType).optional(),
    cancelAtPeriodEnd: z.boolean().optional(),
  });

  /**
   * Get available plans
   * GET /subscriptions/plans
   */
  router.get(
    '/plans',
    asyncHandler(async (_req: AuthenticatedRequest, res: Response) => {
      const plans = Object.values(PLANS).filter((plan) => plan.isActive);

      res.json({
        success: true,
        data: plans,
      });
    })
  );

  /**
   * Get plan details
   * GET /subscriptions/plans/:planType
   */
  router.get(
    '/plans/:planType',
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { planType } = req.params;
      const plan = PLANS[planType as PlanType];

      if (!plan) {
        throw new NotFoundError('Plan', planType);
      }

      res.json({
        success: true,
        data: plan,
      });
    })
  );

  /**
   * Create a new subscription
   * POST /subscriptions
   */
  router.post(
    '/',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const validationResult = CreateSubscriptionSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid subscription data', {
          errors: validationResult.error.errors,
        });
      }

      // Ensure user can only create subscription for their tenant
      if (req.user?.tenantId !== validationResult.data.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new ValidationError('Cannot create subscription for another tenant');
        }
      }

      const subscription = await stripeService.createSubscription(validationResult.data);

      res.status(201).json({
        success: true,
        data: subscription,
      });
    })
  );

  /**
   * Get current subscription for a tenant
   * GET /subscriptions/tenant/:tenantId
   */
  router.get(
    '/tenant/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const subscription = await stripeService.getTenantSubscription(tenantId);

      if (!subscription) {
        throw new NotFoundError('Subscription', tenantId);
      }

      res.json({
        success: true,
        data: subscription,
      });
    })
  );

  /**
   * Get subscription by ID
   * GET /subscriptions/:subscriptionId
   */
  router.get(
    '/:subscriptionId',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { subscriptionId } = req.params;
      const subscription = await stripeService.getSubscription(subscriptionId);

      if (!subscription) {
        throw new NotFoundError('Subscription', subscriptionId);
      }

      // Check tenant access
      if (req.user?.tenantId !== subscription.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Subscription', subscriptionId);
        }
      }

      res.json({
        success: true,
        data: subscription,
      });
    })
  );

  /**
   * Update a subscription (change plan, cancel at period end)
   * PATCH /subscriptions/:subscriptionId
   */
  router.patch(
    '/:subscriptionId',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { subscriptionId } = req.params;

      const validationResult = UpdateSubscriptionSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid update data', {
          errors: validationResult.error.errors,
        });
      }

      // Check current subscription exists and user has access
      const existing = await stripeService.getSubscription(subscriptionId);
      if (!existing) {
        throw new NotFoundError('Subscription', subscriptionId);
      }

      if (req.user?.tenantId !== existing.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Subscription', subscriptionId);
        }
      }

      const subscription = await stripeService.updateSubscription(
        subscriptionId,
        validationResult.data
      );

      res.json({
        success: true,
        data: subscription,
      });
    })
  );

  /**
   * Cancel a subscription
   * POST /subscriptions/:subscriptionId/cancel
   */
  router.post(
    '/:subscriptionId/cancel',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { subscriptionId } = req.params;
      const { immediately } = req.body;

      // Check current subscription exists and user has access
      const existing = await stripeService.getSubscription(subscriptionId);
      if (!existing) {
        throw new NotFoundError('Subscription', subscriptionId);
      }

      if (req.user?.tenantId !== existing.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Subscription', subscriptionId);
        }
      }

      const subscription = await stripeService.cancelSubscription(
        subscriptionId,
        immediately === true
      );

      res.json({
        success: true,
        data: subscription,
        message: immediately
          ? 'Subscription canceled immediately'
          : 'Subscription will be canceled at period end',
      });
    })
  );

  /**
   * Preview plan change (show proration)
   * POST /subscriptions/:subscriptionId/preview-change
   */
  router.post(
    '/:subscriptionId/preview-change',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { subscriptionId } = req.params;
      const { planType } = req.body;

      if (!planType || !Object.values(PlanType).includes(planType)) {
        throw new ValidationError('Invalid plan type');
      }

      // Check subscription access
      const existing = await stripeService.getSubscription(subscriptionId);
      if (!existing) {
        throw new NotFoundError('Subscription', subscriptionId);
      }

      if (req.user?.tenantId !== existing.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Subscription', subscriptionId);
        }
      }

      // Get upcoming invoice to show what change would cost
      const upcoming = await stripeService.getUpcomingInvoice(existing.tenantId);

      res.json({
        success: true,
        data: {
          currentPlan: existing.planType,
          newPlan: planType,
          upcomingInvoice: upcoming,
        },
      });
    })
  );

  /**
   * Reactivate a canceled subscription
   * POST /subscriptions/:subscriptionId/reactivate
   */
  router.post(
    '/:subscriptionId/reactivate',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { subscriptionId } = req.params;

      // Check subscription access
      const existing = await stripeService.getSubscription(subscriptionId);
      if (!existing) {
        throw new NotFoundError('Subscription', subscriptionId);
      }

      if (req.user?.tenantId !== existing.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Subscription', subscriptionId);
        }
      }

      if (!existing.cancelAtPeriodEnd) {
        throw new ValidationError('Subscription is not scheduled for cancellation');
      }

      const subscription = await stripeService.updateSubscription(subscriptionId, {
        cancelAtPeriodEnd: false,
      });

      res.json({
        success: true,
        data: subscription,
        message: 'Subscription reactivated successfully',
      });
    })
  );

  return router;
}
