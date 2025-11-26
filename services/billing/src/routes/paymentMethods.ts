/**
 * Payment Methods API Routes
 *
 * REST endpoints for managing payment methods.
 */

import { Router, Response } from 'express';
import { z } from 'zod';
import { StripeService } from '../services/stripeService';
import { AuthenticatedRequest, authenticate, requireTenantAccess } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import { ValidationError, NotFoundError } from '../utils/errors';

export function createPaymentMethodsRouter(stripeService: StripeService): Router {
  const router = Router();

  const AddPaymentMethodSchema = z.object({
    stripePaymentMethodId: z.string().min(1),
    setAsDefault: z.boolean().optional().default(false),
  });

  /**
   * Get payment methods for a tenant
   * GET /payment-methods/tenant/:tenantId
   */
  router.get(
    '/tenant/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const paymentMethods = await stripeService.getPaymentMethods(tenantId);

      res.json({
        success: true,
        data: paymentMethods,
      });
    })
  );

  /**
   * Add a payment method
   * POST /payment-methods/tenant/:tenantId
   */
  router.post(
    '/tenant/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;

      const validationResult = AddPaymentMethodSchema.safeParse(req.body);
      if (!validationResult.success) {
        throw new ValidationError('Invalid payment method data', {
          errors: validationResult.error.errors,
        });
      }

      const paymentMethod = await stripeService.addPaymentMethod({
        tenantId,
        ...validationResult.data,
      });

      res.status(201).json({
        success: true,
        data: paymentMethod,
      });
    })
  );

  /**
   * Remove a payment method
   * DELETE /payment-methods/:paymentMethodId
   */
  router.delete(
    '/:paymentMethodId',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { paymentMethodId } = req.params;

      // Get payment methods to verify ownership
      const tenantId = req.user?.tenantId;
      if (!tenantId) {
        throw new ValidationError('Tenant ID required');
      }

      const paymentMethods = await stripeService.getPaymentMethods(tenantId);
      const method = paymentMethods.find((pm) => pm.id === paymentMethodId);

      if (!method) {
        throw new NotFoundError('Payment method', paymentMethodId);
      }

      if (method.isDefault) {
        throw new ValidationError('Cannot remove default payment method');
      }

      await stripeService.removePaymentMethod(paymentMethodId);

      res.json({
        success: true,
        message: 'Payment method removed successfully',
      });
    })
  );

  /**
   * Set a payment method as default
   * POST /payment-methods/:paymentMethodId/default
   */
  router.post(
    '/:paymentMethodId/default',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { paymentMethodId } = req.params;

      const tenantId = req.user?.tenantId;
      if (!tenantId) {
        throw new ValidationError('Tenant ID required');
      }

      const paymentMethods = await stripeService.getPaymentMethods(tenantId);
      const method = paymentMethods.find((pm) => pm.id === paymentMethodId);

      if (!method) {
        throw new NotFoundError('Payment method', paymentMethodId);
      }

      // Re-add with setAsDefault flag (this will update the default)
      await stripeService.addPaymentMethod({
        tenantId,
        stripePaymentMethodId: method.stripePaymentMethodId!,
        setAsDefault: true,
      });

      res.json({
        success: true,
        message: 'Default payment method updated',
      });
    })
  );

  /**
   * Create a setup intent for adding new payment methods
   * POST /payment-methods/setup-intent/:tenantId
   */
  router.post(
    '/setup-intent/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const setupIntent = await stripeService.createSetupIntent(tenantId);

      res.json({
        success: true,
        data: setupIntent,
      });
    })
  );

  return router;
}
