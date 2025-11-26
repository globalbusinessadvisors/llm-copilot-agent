/**
 * Webhook Routes
 *
 * Handles incoming webhooks from Stripe and other services.
 */

import { Router, Request, Response } from 'express';
import { StripeService } from '../services/stripeService';
import { logger } from '../utils/logger';

export function createWebhooksRouter(stripeService: StripeService): Router {
  const router = Router();

  /**
   * Stripe webhook handler
   * POST /webhooks/stripe
   *
   * Note: This endpoint should receive raw body, not parsed JSON.
   * Configure express.raw() middleware for this route in the main app.
   */
  router.post('/stripe', async (req: Request, res: Response) => {
    const signature = req.headers['stripe-signature'] as string;

    if (!signature) {
      logger.warn('Stripe webhook missing signature');
      res.status(400).json({ error: 'Missing stripe-signature header' });
      return;
    }

    try {
      // req.body should be the raw Buffer when using express.raw()
      const payload = req.body as Buffer;

      await stripeService.handleWebhook(payload, signature);

      res.status(200).json({ received: true });
    } catch (error) {
      logger.error('Stripe webhook error', {
        error: error instanceof Error ? error.message : 'Unknown error',
      });

      // Return 400 for signature verification failures
      // Return 500 for processing errors (Stripe will retry)
      const statusCode = error instanceof Error && error.message.includes('signature')
        ? 400
        : 500;

      res.status(statusCode).json({
        error: error instanceof Error ? error.message : 'Webhook processing failed',
      });
    }
  });

  /**
   * Health check for webhook endpoint
   * GET /webhooks/health
   */
  router.get('/health', (_req: Request, res: Response) => {
    res.json({ status: 'ok', service: 'webhooks' });
  });

  return router;
}
