/**
 * Invoice API Routes
 *
 * REST endpoints for invoice management and retrieval.
 */

import { Router, Response } from 'express';
import { z } from 'zod';
import { StripeService } from '../services/stripeService';
import { AuthenticatedRequest, authenticate, requireTenantAccess } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import { NotFoundError, ValidationError } from '../utils/errors';

export function createInvoicesRouter(stripeService: StripeService): Router {
  const router = Router();

  /**
   * Get invoices for a tenant
   * GET /invoices/tenant/:tenantId
   */
  router.get(
    '/tenant/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const limit = Math.min(parseInt(req.query.limit as string) || 10, 100);

      const invoices = await stripeService.getTenantInvoices(tenantId, limit);

      res.json({
        success: true,
        data: invoices,
      });
    })
  );

  /**
   * Get invoice by ID
   * GET /invoices/:invoiceId
   */
  router.get(
    '/:invoiceId',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { invoiceId } = req.params;
      const invoice = await stripeService.getInvoice(invoiceId);

      if (!invoice) {
        throw new NotFoundError('Invoice', invoiceId);
      }

      // Check tenant access
      if (req.user?.tenantId !== invoice.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Invoice', invoiceId);
        }
      }

      res.json({
        success: true,
        data: invoice,
      });
    })
  );

  /**
   * Get upcoming invoice preview
   * GET /invoices/upcoming/:tenantId
   */
  router.get(
    '/upcoming/:tenantId',
    authenticate,
    requireTenantAccess,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { tenantId } = req.params;
      const upcoming = await stripeService.getUpcomingInvoice(tenantId);

      if (!upcoming) {
        res.json({
          success: true,
          data: null,
          message: 'No upcoming invoice',
        });
        return;
      }

      res.json({
        success: true,
        data: upcoming,
      });
    })
  );

  /**
   * Download invoice PDF
   * GET /invoices/:invoiceId/pdf
   */
  router.get(
    '/:invoiceId/pdf',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { invoiceId } = req.params;
      const invoice = await stripeService.getInvoice(invoiceId);

      if (!invoice) {
        throw new NotFoundError('Invoice', invoiceId);
      }

      // Check tenant access
      if (req.user?.tenantId !== invoice.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Invoice', invoiceId);
        }
      }

      if (!invoice.pdfUrl) {
        throw new ValidationError('Invoice PDF not available');
      }

      // Redirect to Stripe-hosted PDF
      res.redirect(invoice.pdfUrl);
    })
  );

  /**
   * Get invoice payment link
   * GET /invoices/:invoiceId/pay
   */
  router.get(
    '/:invoiceId/pay',
    authenticate,
    asyncHandler(async (req: AuthenticatedRequest, res: Response) => {
      const { invoiceId } = req.params;
      const invoice = await stripeService.getInvoice(invoiceId);

      if (!invoice) {
        throw new NotFoundError('Invoice', invoiceId);
      }

      // Check tenant access
      if (req.user?.tenantId !== invoice.tenantId) {
        if (!req.user?.roles.includes('admin')) {
          throw new NotFoundError('Invoice', invoiceId);
        }
      }

      if (!invoice.hostedInvoiceUrl) {
        throw new ValidationError('Invoice payment link not available');
      }

      res.json({
        success: true,
        data: {
          paymentUrl: invoice.hostedInvoiceUrl,
        },
      });
    })
  );

  return router;
}
