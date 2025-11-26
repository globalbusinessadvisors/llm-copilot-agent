/**
 * Notification Service
 *
 * Handles sending notifications across multiple channels (email, Slack, PagerDuty, webhooks, SMS).
 */

import axios from 'axios';
import nodemailer from 'nodemailer';
import { WebClient } from '@slack/web-api';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import {
  Alert,
  AlertChannel,
  AlertSeverity,
} from '../models/alert';

export interface NotificationTarget {
  channel: AlertChannel;
  recipientId?: string;
  recipientType?: 'user' | 'schedule' | 'webhook';
  alert: Alert;
  escalationLevel: number;
}

export interface NotificationResult {
  success: boolean;
  channel: AlertChannel;
  recipient: string;
  sentAt: Date;
  error?: string;
}

interface NotificationConfig {
  email: {
    host: string;
    port: number;
    secure: boolean;
    auth: {
      user: string;
      pass: string;
    };
    from: string;
  };
  slack: {
    token: string;
    defaultChannel?: string;
  };
  pagerduty: {
    apiKey: string;
    serviceId: string;
  };
  sms: {
    provider: 'twilio';
    accountSid: string;
    authToken: string;
    fromNumber: string;
  };
}

export class NotificationService {
  private db: Pool;
  private config: NotificationConfig;
  private emailTransport: nodemailer.Transporter | null = null;
  private slackClient: WebClient | null = null;

  constructor(db: Pool, config: Partial<NotificationConfig>) {
    this.db = db;
    this.config = {
      email: config.email || {
        host: process.env.SMTP_HOST || 'localhost',
        port: parseInt(process.env.SMTP_PORT || '587', 10),
        secure: process.env.SMTP_SECURE === 'true',
        auth: {
          user: process.env.SMTP_USER || '',
          pass: process.env.SMTP_PASS || '',
        },
        from: process.env.SMTP_FROM || 'alerts@llm-copilot.com',
      },
      slack: config.slack || {
        token: process.env.SLACK_BOT_TOKEN || '',
        defaultChannel: process.env.SLACK_DEFAULT_CHANNEL,
      },
      pagerduty: config.pagerduty || {
        apiKey: process.env.PAGERDUTY_API_KEY || '',
        serviceId: process.env.PAGERDUTY_SERVICE_ID || '',
      },
      sms: config.sms || {
        provider: 'twilio',
        accountSid: process.env.TWILIO_ACCOUNT_SID || '',
        authToken: process.env.TWILIO_AUTH_TOKEN || '',
        fromNumber: process.env.TWILIO_FROM_NUMBER || '',
      },
    };

    this.initializeClients();
  }

  private initializeClients(): void {
    // Initialize email transport
    if (this.config.email.auth.user) {
      this.emailTransport = nodemailer.createTransport({
        host: this.config.email.host,
        port: this.config.email.port,
        secure: this.config.email.secure,
        auth: this.config.email.auth,
      });
    }

    // Initialize Slack client
    if (this.config.slack.token) {
      this.slackClient = new WebClient(this.config.slack.token);
    }
  }

  /**
   * Send a notification to the appropriate channel
   */
  async sendNotification(target: NotificationTarget): Promise<NotificationResult> {
    const startTime = Date.now();

    try {
      let recipient = target.recipientId || 'default';

      // If targeting a user, get their contact info
      if (target.recipientType === 'user' && target.recipientId) {
        recipient = await this.resolveUserContact(target.recipientId, target.channel);
      } else if (target.recipientType === 'schedule' && target.recipientId) {
        recipient = await this.resolveOnCallUser(target.recipientId, target.channel);
      }

      switch (target.channel) {
        case AlertChannel.EMAIL:
          await this.sendEmail(recipient, target.alert, target.escalationLevel);
          break;
        case AlertChannel.SLACK:
          await this.sendSlack(recipient, target.alert, target.escalationLevel);
          break;
        case AlertChannel.PAGERDUTY:
          await this.sendPagerDuty(target.alert, target.escalationLevel);
          break;
        case AlertChannel.WEBHOOK:
          await this.sendWebhook(recipient, target.alert, target.escalationLevel);
          break;
        case AlertChannel.SMS:
          await this.sendSms(recipient, target.alert, target.escalationLevel);
          break;
        default:
          throw new Error(`Unsupported notification channel: ${target.channel}`);
      }

      const result: NotificationResult = {
        success: true,
        channel: target.channel,
        recipient,
        sentAt: new Date(),
      };

      // Log notification
      await this.logNotification(target.alert.id, result, Date.now() - startTime);

      return result;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      const result: NotificationResult = {
        success: false,
        channel: target.channel,
        recipient: target.recipientId || 'default',
        sentAt: new Date(),
        error: errorMessage,
      };

      await this.logNotification(target.alert.id, result, Date.now() - startTime);

      return result;
    }
  }

  /**
   * Send email notification
   */
  private async sendEmail(to: string, alert: Alert, escalationLevel: number): Promise<void> {
    if (!this.emailTransport) {
      throw new Error('Email transport not configured');
    }

    const subject = this.formatEmailSubject(alert, escalationLevel);
    const html = this.formatEmailBody(alert, escalationLevel);

    await this.emailTransport.sendMail({
      from: this.config.email.from,
      to,
      subject,
      html,
    });
  }

  private formatEmailSubject(alert: Alert, escalationLevel: number): string {
    const severityEmoji = this.getSeverityEmoji(alert.severity);
    const escalationText = escalationLevel > 0 ? ` [Escalation Level ${escalationLevel}]` : '';
    return `${severityEmoji} [${alert.severity.toUpperCase()}] ${alert.title}${escalationText}`;
  }

  private formatEmailBody(alert: Alert, escalationLevel: number): string {
    return `
      <!DOCTYPE html>
      <html>
      <head>
        <style>
          body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
          .container { max-width: 600px; margin: 0 auto; padding: 20px; }
          .header { background: ${this.getSeverityColor(alert.severity)}; color: white; padding: 20px; border-radius: 8px 8px 0 0; }
          .body { background: #f9fafb; padding: 20px; border-radius: 0 0 8px 8px; }
          .badge { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 12px; font-weight: 600; }
          .severity-critical { background: #dc2626; color: white; }
          .severity-error { background: #ea580c; color: white; }
          .severity-warning { background: #ca8a04; color: white; }
          .severity-info { background: #2563eb; color: white; }
          .metadata { margin-top: 20px; }
          .metadata-item { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #e5e7eb; }
          .btn { display: inline-block; background: #2563eb; color: white; padding: 10px 20px; border-radius: 6px; text-decoration: none; margin-top: 20px; }
        </style>
      </head>
      <body>
        <div class="container">
          <div class="header">
            <h1 style="margin: 0 0 10px 0;">${alert.title}</h1>
            <span class="badge severity-${alert.severity}">${alert.severity.toUpperCase()}</span>
            ${escalationLevel > 0 ? `<span class="badge" style="background: #7c3aed; margin-left: 8px;">Escalation ${escalationLevel}</span>` : ''}
          </div>
          <div class="body">
            <p style="font-size: 16px; line-height: 1.6;">${alert.description}</p>

            <div class="metadata">
              <div class="metadata-item">
                <span><strong>Source:</strong></span>
                <span>${alert.source}</span>
              </div>
              <div class="metadata-item">
                <span><strong>Triggered At:</strong></span>
                <span>${alert.triggeredAt.toISOString()}</span>
              </div>
              <div class="metadata-item">
                <span><strong>Alert ID:</strong></span>
                <span>${alert.id}</span>
              </div>
              ${Object.entries(alert.tags).map(([key, value]) => `
                <div class="metadata-item">
                  <span><strong>${key}:</strong></span>
                  <span>${value}</span>
                </div>
              `).join('')}
            </div>

            <a href="${process.env.DASHBOARD_URL || 'https://dashboard.llm-copilot.com'}/alerts/${alert.id}" class="btn">View Alert</a>
          </div>
        </div>
      </body>
      </html>
    `;
  }

  /**
   * Send Slack notification
   */
  private async sendSlack(channel: string, alert: Alert, escalationLevel: number): Promise<void> {
    if (!this.slackClient) {
      throw new Error('Slack client not configured');
    }

    const blocks = this.formatSlackBlocks(alert, escalationLevel);

    await this.slackClient.chat.postMessage({
      channel: channel || this.config.slack.defaultChannel || '#alerts',
      text: `${this.getSeverityEmoji(alert.severity)} [${alert.severity.toUpperCase()}] ${alert.title}`,
      blocks,
    });
  }

  private formatSlackBlocks(alert: Alert, escalationLevel: number): object[] {
    const blocks: object[] = [
      {
        type: 'header',
        text: {
          type: 'plain_text',
          text: `${this.getSeverityEmoji(alert.severity)} ${alert.title}`,
          emoji: true,
        },
      },
      {
        type: 'section',
        fields: [
          {
            type: 'mrkdwn',
            text: `*Severity:*\n${alert.severity.toUpperCase()}`,
          },
          {
            type: 'mrkdwn',
            text: `*Status:*\n${alert.status}`,
          },
          {
            type: 'mrkdwn',
            text: `*Source:*\n${alert.source}`,
          },
          {
            type: 'mrkdwn',
            text: `*Triggered:*\n<!date^${Math.floor(alert.triggeredAt.getTime() / 1000)}^{date_short_pretty} at {time}|${alert.triggeredAt.toISOString()}>`,
          },
        ],
      },
      {
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: `*Description:*\n${alert.description}`,
        },
      },
    ];

    if (escalationLevel > 0) {
      blocks.push({
        type: 'context',
        elements: [
          {
            type: 'mrkdwn',
            text: `:warning: *Escalation Level ${escalationLevel}* - This alert has been escalated`,
          },
        ],
      });
    }

    if (Object.keys(alert.tags).length > 0) {
      const tagText = Object.entries(alert.tags)
        .map(([key, value]) => `\`${key}: ${value}\``)
        .join(' ');
      blocks.push({
        type: 'context',
        elements: [
          {
            type: 'mrkdwn',
            text: `Tags: ${tagText}`,
          },
        ],
      });
    }

    blocks.push(
      {
        type: 'divider',
      },
      {
        type: 'actions',
        elements: [
          {
            type: 'button',
            text: {
              type: 'plain_text',
              text: 'Acknowledge',
              emoji: true,
            },
            style: 'primary',
            action_id: `acknowledge_alert_${alert.id}`,
            value: alert.id,
          },
          {
            type: 'button',
            text: {
              type: 'plain_text',
              text: 'Resolve',
              emoji: true,
            },
            style: 'danger',
            action_id: `resolve_alert_${alert.id}`,
            value: alert.id,
          },
          {
            type: 'button',
            text: {
              type: 'plain_text',
              text: 'View Details',
              emoji: true,
            },
            url: `${process.env.DASHBOARD_URL || 'https://dashboard.llm-copilot.com'}/alerts/${alert.id}`,
            action_id: `view_alert_${alert.id}`,
          },
        ],
      }
    );

    return blocks;
  }

  /**
   * Send PagerDuty notification
   */
  private async sendPagerDuty(alert: Alert, escalationLevel: number): Promise<void> {
    if (!this.config.pagerduty.apiKey) {
      throw new Error('PagerDuty not configured');
    }

    const severity = this.mapSeverityToPagerDuty(alert.severity);
    const dedupKey = `${alert.ruleId}-${alert.source}`;

    await axios.post(
      'https://events.pagerduty.com/v2/enqueue',
      {
        routing_key: this.config.pagerduty.apiKey,
        event_action: 'trigger',
        dedup_key: dedupKey,
        payload: {
          summary: alert.title,
          severity,
          source: alert.source,
          timestamp: alert.triggeredAt.toISOString(),
          custom_details: {
            description: alert.description,
            alert_id: alert.id,
            rule_id: alert.ruleId,
            escalation_level: escalationLevel,
            tags: alert.tags,
            metadata: alert.metadata,
          },
        },
        links: [
          {
            href: `${process.env.DASHBOARD_URL || 'https://dashboard.llm-copilot.com'}/alerts/${alert.id}`,
            text: 'View in Dashboard',
          },
        ],
      },
      {
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  }

  private mapSeverityToPagerDuty(severity: AlertSeverity): string {
    switch (severity) {
      case AlertSeverity.CRITICAL:
        return 'critical';
      case AlertSeverity.ERROR:
        return 'error';
      case AlertSeverity.WARNING:
        return 'warning';
      case AlertSeverity.INFO:
        return 'info';
      default:
        return 'info';
    }
  }

  /**
   * Send webhook notification
   */
  private async sendWebhook(url: string, alert: Alert, escalationLevel: number): Promise<void> {
    const payload = {
      event: 'alert.triggered',
      timestamp: new Date().toISOString(),
      alert: {
        id: alert.id,
        ruleId: alert.ruleId,
        title: alert.title,
        description: alert.description,
        severity: alert.severity,
        status: alert.status,
        source: alert.source,
        tags: alert.tags,
        metadata: alert.metadata,
        triggeredAt: alert.triggeredAt.toISOString(),
        escalationLevel,
      },
    };

    await axios.post(url, payload, {
      headers: {
        'Content-Type': 'application/json',
        'X-Alert-Signature': this.generateWebhookSignature(payload),
      },
      timeout: 10000,
    });
  }

  private generateWebhookSignature(payload: object): string {
    const crypto = require('crypto');
    const secret = process.env.WEBHOOK_SECRET || 'default-secret';
    return crypto
      .createHmac('sha256', secret)
      .update(JSON.stringify(payload))
      .digest('hex');
  }

  /**
   * Send SMS notification
   */
  private async sendSms(phoneNumber: string, alert: Alert, escalationLevel: number): Promise<void> {
    if (!this.config.sms.accountSid) {
      throw new Error('SMS (Twilio) not configured');
    }

    const message = this.formatSmsMessage(alert, escalationLevel);

    await axios.post(
      `https://api.twilio.com/2010-04-01/Accounts/${this.config.sms.accountSid}/Messages.json`,
      new URLSearchParams({
        To: phoneNumber,
        From: this.config.sms.fromNumber,
        Body: message,
      }),
      {
        auth: {
          username: this.config.sms.accountSid,
          password: this.config.sms.authToken,
        },
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
      }
    );
  }

  private formatSmsMessage(alert: Alert, escalationLevel: number): string {
    const emoji = this.getSeverityEmoji(alert.severity);
    const escalation = escalationLevel > 0 ? ` [ESC${escalationLevel}]` : '';
    return `${emoji} [${alert.severity.toUpperCase()}]${escalation} ${alert.title}\n\n${alert.description.substring(0, 100)}${alert.description.length > 100 ? '...' : ''}\n\nSource: ${alert.source}`;
  }

  // ===========================================
  // Helper Methods
  // ===========================================

  private getSeverityEmoji(severity: AlertSeverity): string {
    switch (severity) {
      case AlertSeverity.CRITICAL:
        return '🔴';
      case AlertSeverity.ERROR:
        return '🟠';
      case AlertSeverity.WARNING:
        return '🟡';
      case AlertSeverity.INFO:
        return '🔵';
      default:
        return '⚪';
    }
  }

  private getSeverityColor(severity: AlertSeverity): string {
    switch (severity) {
      case AlertSeverity.CRITICAL:
        return '#dc2626';
      case AlertSeverity.ERROR:
        return '#ea580c';
      case AlertSeverity.WARNING:
        return '#ca8a04';
      case AlertSeverity.INFO:
        return '#2563eb';
      default:
        return '#6b7280';
    }
  }

  /**
   * Resolve user contact information for a specific channel
   */
  private async resolveUserContact(userId: string, channel: AlertChannel): Promise<string> {
    const result = await this.db.query(
      `SELECT email, phone, slack_user_id, notification_preferences
       FROM on_call_users WHERE id = $1`,
      [userId]
    );

    if (result.rows.length === 0) {
      throw new Error(`User not found: ${userId}`);
    }

    const user = result.rows[0];
    const preferences = user.notification_preferences || {};

    switch (channel) {
      case AlertChannel.EMAIL:
        if (!preferences.email) {
          throw new Error(`User ${userId} has email notifications disabled`);
        }
        return user.email;
      case AlertChannel.SLACK:
        if (!preferences.slack || !user.slack_user_id) {
          throw new Error(`User ${userId} has Slack notifications disabled or not configured`);
        }
        return user.slack_user_id;
      case AlertChannel.SMS:
        if (!preferences.sms || !user.phone) {
          throw new Error(`User ${userId} has SMS notifications disabled or phone not configured`);
        }
        return user.phone;
      default:
        return user.email;
    }
  }

  /**
   * Resolve the current on-call user for a schedule
   */
  private async resolveOnCallUser(scheduleId: string, channel: AlertChannel): Promise<string> {
    // First check for overrides
    const overrideResult = await this.db.query(
      `SELECT user_id FROM on_call_overrides
       WHERE schedule_id = $1 AND start_at <= NOW() AND end_at > NOW()
       ORDER BY created_at DESC LIMIT 1`,
      [scheduleId]
    );

    if (overrideResult.rows.length > 0) {
      return this.resolveUserContact(overrideResult.rows[0].user_id, channel);
    }

    // Get the schedule and current rotation
    const scheduleResult = await this.db.query(
      `SELECT rotations, timezone FROM on_call_schedules WHERE id = $1`,
      [scheduleId]
    );

    if (scheduleResult.rows.length === 0) {
      throw new Error(`Schedule not found: ${scheduleId}`);
    }

    const schedule = scheduleResult.rows[0];
    const rotations = schedule.rotations || [];

    if (rotations.length === 0) {
      throw new Error(`Schedule ${scheduleId} has no rotations configured`);
    }

    // For simplicity, use the first rotation and calculate the current user
    // In a real implementation, this would consider rotation type, restrictions, etc.
    const rotation = rotations[0];
    const users = rotation.users || [];

    if (users.length === 0) {
      throw new Error(`Rotation has no users`);
    }

    // Simple round-robin based on current time
    const now = new Date();
    const dayOfYear = Math.floor((now.getTime() - new Date(now.getFullYear(), 0, 0).getTime()) / (1000 * 60 * 60 * 24));
    const userIndex = dayOfYear % users.length;
    const currentUserId = users[userIndex];

    return this.resolveUserContact(currentUserId, channel);
  }

  /**
   * Log notification to database
   */
  private async logNotification(
    alertId: string,
    result: NotificationResult,
    durationMs: number
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO notification_logs (
        id, alert_id, channel, recipient, success, error_message, sent_at, duration_ms
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        uuidv4(),
        alertId,
        result.channel,
        result.recipient,
        result.success,
        result.error || null,
        result.sentAt,
        durationMs,
      ]
    );
  }

  /**
   * Send test notification
   */
  async sendTestNotification(
    channel: AlertChannel,
    recipient: string
  ): Promise<NotificationResult> {
    const testAlert: Alert = {
      id: uuidv4(),
      ruleId: 'test-rule',
      title: 'Test Alert',
      description: 'This is a test notification to verify your alerting configuration.',
      severity: AlertSeverity.INFO,
      status: 'triggered' as any,
      source: 'test',
      tags: { test: 'true' },
      metadata: {},
      triggeredAt: new Date(),
      escalationLevel: 0,
      notificationsSent: [],
    };

    return this.sendNotification({
      channel,
      recipientId: recipient,
      alert: testAlert,
      escalationLevel: 0,
    });
  }
}
