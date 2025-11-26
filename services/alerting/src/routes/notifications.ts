/**
 * Notification Routes
 *
 * REST API endpoints for notification testing and management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { NotificationService } from '../services/notificationService';
import { AlertChannel } from '../models/alert';

const router = Router();

// Request validation schemas
const TestNotificationSchema = z.object({
  channel: z.nativeEnum(AlertChannel),
  recipient: z.string().min(1),
});

export function createNotificationRoutes(notificationService: NotificationService): Router {
  /**
   * Send a test notification
   * POST /api/v1/notifications/test
   */
  router.post('/test', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = TestNotificationSchema.parse(req.body);

      const result = await notificationService.sendTestNotification(
        input.channel,
        input.recipient
      );

      if (result.success) {
        res.json({
          success: true,
          message: `Test notification sent successfully to ${result.recipient} via ${result.channel}`,
          data: result,
        });
      } else {
        res.status(500).json({
          success: false,
          error: 'Failed to send test notification',
          details: result.error,
          data: result,
        });
      }
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
   * Get supported notification channels
   * GET /api/v1/notifications/channels
   */
  router.get('/channels', (req: Request, res: Response) => {
    const channels = Object.values(AlertChannel).map(channel => ({
      id: channel,
      name: formatChannelName(channel),
      description: getChannelDescription(channel),
      configurationRequired: getChannelConfigRequirements(channel),
    }));

    res.json({
      success: true,
      data: channels,
    });
  });

  return router;
}

function formatChannelName(channel: AlertChannel): string {
  switch (channel) {
    case AlertChannel.EMAIL:
      return 'Email';
    case AlertChannel.SLACK:
      return 'Slack';
    case AlertChannel.PAGERDUTY:
      return 'PagerDuty';
    case AlertChannel.WEBHOOK:
      return 'Webhook';
    case AlertChannel.SMS:
      return 'SMS';
    default:
      return channel;
  }
}

function getChannelDescription(channel: AlertChannel): string {
  switch (channel) {
    case AlertChannel.EMAIL:
      return 'Send alerts via email. Supports rich HTML formatting with alert details.';
    case AlertChannel.SLACK:
      return 'Send alerts to Slack channels or DMs. Includes interactive buttons for acknowledgment.';
    case AlertChannel.PAGERDUTY:
      return 'Integrate with PagerDuty for incident management and on-call routing.';
    case AlertChannel.WEBHOOK:
      return 'Send alerts to custom HTTP endpoints. Includes HMAC signature for verification.';
    case AlertChannel.SMS:
      return 'Send SMS alerts via Twilio. Best for critical alerts requiring immediate attention.';
    default:
      return 'Unknown notification channel';
  }
}

function getChannelConfigRequirements(channel: AlertChannel): string[] {
  switch (channel) {
    case AlertChannel.EMAIL:
      return ['SMTP_HOST', 'SMTP_PORT', 'SMTP_USER', 'SMTP_PASS', 'SMTP_FROM'];
    case AlertChannel.SLACK:
      return ['SLACK_BOT_TOKEN', 'SLACK_DEFAULT_CHANNEL (optional)'];
    case AlertChannel.PAGERDUTY:
      return ['PAGERDUTY_API_KEY', 'PAGERDUTY_SERVICE_ID'];
    case AlertChannel.WEBHOOK:
      return ['WEBHOOK_SECRET'];
    case AlertChannel.SMS:
      return ['TWILIO_ACCOUNT_SID', 'TWILIO_AUTH_TOKEN', 'TWILIO_FROM_NUMBER'];
    default:
      return [];
  }
}
