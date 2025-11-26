/**
 * Stripe Integration Service
 *
 * Handles all Stripe-related operations for subscriptions, payments, and invoices.
 */

import Stripe from 'stripe';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import {
  Subscription,
  SubscriptionStatus,
  PlanType,
  PLANS,
  Invoice,
  InvoiceStatus,
  PaymentMethod,
  PaymentMethodType,
  CreateSubscriptionInput,
  UpdateSubscriptionInput,
  CreatePaymentMethodInput,
  BillingEvent,
  BillingEventType,
} from '../models/subscription';
import { logger } from '../utils/logger';

export class StripeService {
  private stripe: Stripe;
  private db: Pool;
  private webhookSecret: string;

  constructor(db: Pool, apiKey: string, webhookSecret: string) {
    this.stripe = new Stripe(apiKey, {
      apiVersion: '2023-10-16',
      typescript: true,
    });
    this.db = db;
    this.webhookSecret = webhookSecret;
  }

  // ===========================================
  // Customer Management
  // ===========================================

  /**
   * Create or get Stripe customer for a tenant
   */
  async getOrCreateCustomer(
    tenantId: string,
    email: string,
    name?: string,
    metadata?: Record<string, string>
  ): Promise<string> {
    // Check if customer already exists
    const result = await this.db.query(
      `SELECT stripe_customer_id FROM tenants WHERE id = $1`,
      [tenantId]
    );

    if (result.rows[0]?.stripe_customer_id) {
      return result.rows[0].stripe_customer_id;
    }

    // Create new Stripe customer
    const customer = await this.stripe.customers.create({
      email,
      name,
      metadata: {
        tenant_id: tenantId,
        ...metadata,
      },
    });

    // Store customer ID
    await this.db.query(
      `UPDATE tenants SET stripe_customer_id = $1 WHERE id = $2`,
      [customer.id, tenantId]
    );

    logger.info('Stripe customer created', { tenantId, customerId: customer.id });

    return customer.id;
  }

  /**
   * Update customer information
   */
  async updateCustomer(
    customerId: string,
    updates: { email?: string; name?: string; metadata?: Record<string, string> }
  ): Promise<void> {
    await this.stripe.customers.update(customerId, updates);
  }

  // ===========================================
  // Subscription Management
  // ===========================================

  /**
   * Create a new subscription
   */
  async createSubscription(input: CreateSubscriptionInput): Promise<Subscription> {
    const plan = PLANS[input.planType];
    if (!plan) {
      throw new Error(`Invalid plan type: ${input.planType}`);
    }

    // Get or create customer
    const tenant = await this.db.query(
      `SELECT email, name, stripe_customer_id FROM tenants WHERE id = $1`,
      [input.tenantId]
    );

    if (tenant.rows.length === 0) {
      throw new Error(`Tenant not found: ${input.tenantId}`);
    }

    const customerId = await this.getOrCreateCustomer(
      input.tenantId,
      tenant.rows[0].email,
      tenant.rows[0].name
    );

    // Create Stripe subscription
    const subscriptionParams: Stripe.SubscriptionCreateParams = {
      customer: customerId,
      items: [{ price: plan.stripePriceIdMonthly }],
      payment_behavior: 'default_incomplete',
      expand: ['latest_invoice.payment_intent'],
    };

    if (input.paymentMethodId) {
      subscriptionParams.default_payment_method = input.paymentMethodId;
    }

    if (input.trialDays) {
      subscriptionParams.trial_period_days = input.trialDays;
    }

    const stripeSubscription = await this.stripe.subscriptions.create(subscriptionParams);

    // Store subscription in database
    const subscription: Subscription = {
      id: uuidv4(),
      tenantId: input.tenantId,
      planId: plan.id,
      planType: input.planType,
      status: this.mapStripeStatus(stripeSubscription.status),
      stripeSubscriptionId: stripeSubscription.id,
      stripeCustomerId: customerId,
      currentPeriodStart: new Date(stripeSubscription.current_period_start * 1000),
      currentPeriodEnd: new Date(stripeSubscription.current_period_end * 1000),
      cancelAtPeriodEnd: stripeSubscription.cancel_at_period_end,
      trialStart: stripeSubscription.trial_start
        ? new Date(stripeSubscription.trial_start * 1000)
        : undefined,
      trialEnd: stripeSubscription.trial_end
        ? new Date(stripeSubscription.trial_end * 1000)
        : undefined,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO subscriptions (
        id, tenant_id, plan_id, plan_type, status, stripe_subscription_id,
        stripe_customer_id, current_period_start, current_period_end,
        cancel_at_period_end, trial_start, trial_end, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)`,
      [
        subscription.id,
        subscription.tenantId,
        subscription.planId,
        subscription.planType,
        subscription.status,
        subscription.stripeSubscriptionId,
        subscription.stripeCustomerId,
        subscription.currentPeriodStart,
        subscription.currentPeriodEnd,
        subscription.cancelAtPeriodEnd,
        subscription.trialStart,
        subscription.trialEnd,
        subscription.createdAt,
        subscription.updatedAt,
      ]
    );

    // Set quota based on plan
    await this.setQuotaForPlan(input.tenantId, input.planType, subscription.currentPeriodStart, subscription.currentPeriodEnd);

    await this.recordBillingEvent({
      type: BillingEventType.SUBSCRIPTION_CREATED,
      tenantId: input.tenantId,
      subscriptionId: subscription.id,
      data: { planType: input.planType },
    });

    logger.info('Subscription created', {
      subscriptionId: subscription.id,
      tenantId: input.tenantId,
      planType: input.planType,
    });

    return subscription;
  }

  /**
   * Update a subscription
   */
  async updateSubscription(
    subscriptionId: string,
    input: UpdateSubscriptionInput
  ): Promise<Subscription> {
    const subscription = await this.getSubscription(subscriptionId);
    if (!subscription) {
      throw new Error(`Subscription not found: ${subscriptionId}`);
    }

    const updates: Stripe.SubscriptionUpdateParams = {};

    if (input.planType && input.planType !== subscription.planType) {
      const newPlan = PLANS[input.planType];
      if (!newPlan) {
        throw new Error(`Invalid plan type: ${input.planType}`);
      }

      // Get current subscription items
      const stripeSubscription = await this.stripe.subscriptions.retrieve(
        subscription.stripeSubscriptionId!
      );

      updates.items = [
        {
          id: stripeSubscription.items.data[0].id,
          price: newPlan.stripePriceIdMonthly,
        },
      ];
      updates.proration_behavior = 'create_prorations';
    }

    if (input.cancelAtPeriodEnd !== undefined) {
      updates.cancel_at_period_end = input.cancelAtPeriodEnd;
    }

    const updatedStripeSubscription = await this.stripe.subscriptions.update(
      subscription.stripeSubscriptionId!,
      updates
    );

    // Update database
    await this.db.query(
      `UPDATE subscriptions SET
        plan_type = COALESCE($1, plan_type),
        cancel_at_period_end = $2,
        updated_at = NOW()
      WHERE id = $3`,
      [input.planType, input.cancelAtPeriodEnd, subscriptionId]
    );

    // Update quota if plan changed
    if (input.planType) {
      await this.setQuotaForPlan(
        subscription.tenantId,
        input.planType,
        subscription.currentPeriodStart,
        subscription.currentPeriodEnd
      );
    }

    await this.recordBillingEvent({
      type: BillingEventType.SUBSCRIPTION_UPDATED,
      tenantId: subscription.tenantId,
      subscriptionId,
      data: { updates: input },
    });

    return this.getSubscription(subscriptionId) as Promise<Subscription>;
  }

  /**
   * Cancel a subscription
   */
  async cancelSubscription(
    subscriptionId: string,
    immediately: boolean = false
  ): Promise<Subscription> {
    const subscription = await this.getSubscription(subscriptionId);
    if (!subscription) {
      throw new Error(`Subscription not found: ${subscriptionId}`);
    }

    if (immediately) {
      await this.stripe.subscriptions.cancel(subscription.stripeSubscriptionId!);
    } else {
      await this.stripe.subscriptions.update(subscription.stripeSubscriptionId!, {
        cancel_at_period_end: true,
      });
    }

    await this.db.query(
      `UPDATE subscriptions SET
        status = $1,
        cancel_at_period_end = $2,
        canceled_at = NOW(),
        updated_at = NOW()
      WHERE id = $3`,
      [
        immediately ? SubscriptionStatus.CANCELED : subscription.status,
        !immediately,
        subscriptionId,
      ]
    );

    await this.recordBillingEvent({
      type: BillingEventType.SUBSCRIPTION_CANCELED,
      tenantId: subscription.tenantId,
      subscriptionId,
      data: { immediately },
    });

    return this.getSubscription(subscriptionId) as Promise<Subscription>;
  }

  /**
   * Get subscription by ID
   */
  async getSubscription(subscriptionId: string): Promise<Subscription | null> {
    const result = await this.db.query(
      `SELECT * FROM subscriptions WHERE id = $1`,
      [subscriptionId]
    );

    if (result.rows.length === 0) return null;

    return this.mapSubscriptionRow(result.rows[0]);
  }

  /**
   * Get active subscription for a tenant
   */
  async getTenantSubscription(tenantId: string): Promise<Subscription | null> {
    const result = await this.db.query(
      `SELECT * FROM subscriptions
       WHERE tenant_id = $1 AND status IN ('active', 'trialing', 'past_due')
       ORDER BY created_at DESC
       LIMIT 1`,
      [tenantId]
    );

    if (result.rows.length === 0) return null;

    return this.mapSubscriptionRow(result.rows[0]);
  }

  // ===========================================
  // Invoice Management
  // ===========================================

  /**
   * Get invoices for a tenant
   */
  async getTenantInvoices(
    tenantId: string,
    limit: number = 10
  ): Promise<Invoice[]> {
    const result = await this.db.query(
      `SELECT * FROM invoices
       WHERE tenant_id = $1
       ORDER BY created_at DESC
       LIMIT $2`,
      [tenantId, limit]
    );

    return result.rows.map(this.mapInvoiceRow);
  }

  /**
   * Get invoice by ID
   */
  async getInvoice(invoiceId: string): Promise<Invoice | null> {
    const result = await this.db.query(
      `SELECT * FROM invoices WHERE id = $1`,
      [invoiceId]
    );

    if (result.rows.length === 0) return null;

    return this.mapInvoiceRow(result.rows[0]);
  }

  /**
   * Get upcoming invoice preview
   */
  async getUpcomingInvoice(tenantId: string): Promise<{
    amount: number;
    lineItems: { description: string; amount: number }[];
    dueDate: Date;
  } | null> {
    const subscription = await this.getTenantSubscription(tenantId);
    if (!subscription?.stripeCustomerId) return null;

    try {
      const invoice = await this.stripe.invoices.retrieveUpcoming({
        customer: subscription.stripeCustomerId,
      });

      return {
        amount: invoice.amount_due / 100,
        lineItems: invoice.lines.data.map((line) => ({
          description: line.description || 'Subscription',
          amount: line.amount / 100,
        })),
        dueDate: new Date((invoice.due_date || invoice.period_end) * 1000),
      };
    } catch (error) {
      return null;
    }
  }

  // ===========================================
  // Payment Methods
  // ===========================================

  /**
   * Add a payment method
   */
  async addPaymentMethod(input: CreatePaymentMethodInput): Promise<PaymentMethod> {
    const subscription = await this.getTenantSubscription(input.tenantId);
    if (!subscription?.stripeCustomerId) {
      throw new Error('No active subscription found');
    }

    // Attach payment method to customer
    const stripePaymentMethod = await this.stripe.paymentMethods.attach(
      input.stripePaymentMethodId,
      { customer: subscription.stripeCustomerId }
    );

    // Set as default if requested
    if (input.setAsDefault) {
      await this.stripe.customers.update(subscription.stripeCustomerId, {
        invoice_settings: { default_payment_method: input.stripePaymentMethodId },
      });

      // Unset other defaults
      await this.db.query(
        `UPDATE payment_methods SET is_default = false WHERE tenant_id = $1`,
        [input.tenantId]
      );
    }

    const paymentMethod: PaymentMethod = {
      id: uuidv4(),
      tenantId: input.tenantId,
      type: PaymentMethodType.CARD,
      stripePaymentMethodId: input.stripePaymentMethodId,
      isDefault: input.setAsDefault || false,
      card: stripePaymentMethod.card
        ? {
            brand: stripePaymentMethod.card.brand,
            last4: stripePaymentMethod.card.last4,
            expMonth: stripePaymentMethod.card.exp_month,
            expYear: stripePaymentMethod.card.exp_year,
          }
        : undefined,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO payment_methods (
        id, tenant_id, type, stripe_payment_method_id, is_default,
        card_brand, card_last4, card_exp_month, card_exp_year,
        created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)`,
      [
        paymentMethod.id,
        paymentMethod.tenantId,
        paymentMethod.type,
        paymentMethod.stripePaymentMethodId,
        paymentMethod.isDefault,
        paymentMethod.card?.brand,
        paymentMethod.card?.last4,
        paymentMethod.card?.expMonth,
        paymentMethod.card?.expYear,
        paymentMethod.createdAt,
        paymentMethod.updatedAt,
      ]
    );

    return paymentMethod;
  }

  /**
   * Get payment methods for a tenant
   */
  async getPaymentMethods(tenantId: string): Promise<PaymentMethod[]> {
    const result = await this.db.query(
      `SELECT * FROM payment_methods WHERE tenant_id = $1 ORDER BY is_default DESC, created_at DESC`,
      [tenantId]
    );

    return result.rows.map(this.mapPaymentMethodRow);
  }

  /**
   * Remove a payment method
   */
  async removePaymentMethod(paymentMethodId: string): Promise<void> {
    const result = await this.db.query(
      `SELECT stripe_payment_method_id FROM payment_methods WHERE id = $1`,
      [paymentMethodId]
    );

    if (result.rows[0]?.stripe_payment_method_id) {
      await this.stripe.paymentMethods.detach(result.rows[0].stripe_payment_method_id);
    }

    await this.db.query(`DELETE FROM payment_methods WHERE id = $1`, [paymentMethodId]);
  }

  /**
   * Create a setup intent for adding payment methods
   */
  async createSetupIntent(tenantId: string): Promise<{ clientSecret: string }> {
    const subscription = await this.getTenantSubscription(tenantId);
    const customerId = subscription?.stripeCustomerId ||
      await this.getOrCreateCustomer(tenantId, '', '');

    const setupIntent = await this.stripe.setupIntents.create({
      customer: customerId,
      payment_method_types: ['card'],
    });

    return { clientSecret: setupIntent.client_secret! };
  }

  // ===========================================
  // Webhook Handling
  // ===========================================

  /**
   * Handle Stripe webhook events
   */
  async handleWebhook(payload: Buffer, signature: string): Promise<void> {
    let event: Stripe.Event;

    try {
      event = this.stripe.webhooks.constructEvent(payload, signature, this.webhookSecret);
    } catch (error) {
      logger.error('Webhook signature verification failed', { error });
      throw new Error('Invalid webhook signature');
    }

    logger.info('Stripe webhook received', { type: event.type, id: event.id });

    switch (event.type) {
      case 'invoice.paid':
        await this.handleInvoicePaid(event.data.object as Stripe.Invoice);
        break;
      case 'invoice.payment_failed':
        await this.handleInvoicePaymentFailed(event.data.object as Stripe.Invoice);
        break;
      case 'customer.subscription.updated':
        await this.handleSubscriptionUpdated(event.data.object as Stripe.Subscription);
        break;
      case 'customer.subscription.deleted':
        await this.handleSubscriptionDeleted(event.data.object as Stripe.Subscription);
        break;
      default:
        logger.debug('Unhandled webhook event', { type: event.type });
    }
  }

  private async handleInvoicePaid(stripeInvoice: Stripe.Invoice): Promise<void> {
    const invoice = await this.syncInvoice(stripeInvoice);
    if (invoice) {
      await this.recordBillingEvent({
        type: BillingEventType.INVOICE_PAID,
        tenantId: invoice.tenantId,
        invoiceId: invoice.id,
        data: { amount: invoice.amountPaid },
      });
    }
  }

  private async handleInvoicePaymentFailed(stripeInvoice: Stripe.Invoice): Promise<void> {
    const invoice = await this.syncInvoice(stripeInvoice);
    if (invoice) {
      await this.recordBillingEvent({
        type: BillingEventType.INVOICE_FAILED,
        tenantId: invoice.tenantId,
        invoiceId: invoice.id,
        data: { amount: invoice.amountDue },
      });

      // Update subscription status
      if (stripeInvoice.subscription) {
        await this.db.query(
          `UPDATE subscriptions SET status = 'past_due', updated_at = NOW()
           WHERE stripe_subscription_id = $1`,
          [stripeInvoice.subscription]
        );
      }
    }
  }

  private async handleSubscriptionUpdated(stripeSubscription: Stripe.Subscription): Promise<void> {
    await this.db.query(
      `UPDATE subscriptions SET
        status = $1,
        current_period_start = $2,
        current_period_end = $3,
        cancel_at_period_end = $4,
        updated_at = NOW()
      WHERE stripe_subscription_id = $5`,
      [
        this.mapStripeStatus(stripeSubscription.status),
        new Date(stripeSubscription.current_period_start * 1000),
        new Date(stripeSubscription.current_period_end * 1000),
        stripeSubscription.cancel_at_period_end,
        stripeSubscription.id,
      ]
    );
  }

  private async handleSubscriptionDeleted(stripeSubscription: Stripe.Subscription): Promise<void> {
    await this.db.query(
      `UPDATE subscriptions SET
        status = 'canceled',
        canceled_at = NOW(),
        updated_at = NOW()
      WHERE stripe_subscription_id = $1`,
      [stripeSubscription.id]
    );
  }

  private async syncInvoice(stripeInvoice: Stripe.Invoice): Promise<Invoice | null> {
    const subscription = await this.db.query(
      `SELECT id, tenant_id FROM subscriptions WHERE stripe_subscription_id = $1`,
      [stripeInvoice.subscription]
    );

    if (subscription.rows.length === 0) return null;

    const existingInvoice = await this.db.query(
      `SELECT id FROM invoices WHERE stripe_invoice_id = $1`,
      [stripeInvoice.id]
    );

    const invoice: Invoice = {
      id: existingInvoice.rows[0]?.id || uuidv4(),
      tenantId: subscription.rows[0].tenant_id,
      subscriptionId: subscription.rows[0].id,
      stripeInvoiceId: stripeInvoice.id,
      number: stripeInvoice.number || '',
      status: this.mapInvoiceStatus(stripeInvoice.status),
      currency: stripeInvoice.currency,
      subtotal: stripeInvoice.subtotal / 100,
      tax: (stripeInvoice.tax || 0) / 100,
      total: stripeInvoice.total / 100,
      amountDue: stripeInvoice.amount_due / 100,
      amountPaid: stripeInvoice.amount_paid / 100,
      amountRemaining: stripeInvoice.amount_remaining / 100,
      periodStart: new Date(stripeInvoice.period_start * 1000),
      periodEnd: new Date(stripeInvoice.period_end * 1000),
      dueDate: stripeInvoice.due_date ? new Date(stripeInvoice.due_date * 1000) : undefined,
      paidAt: stripeInvoice.status_transitions?.paid_at
        ? new Date(stripeInvoice.status_transitions.paid_at * 1000)
        : undefined,
      hostedInvoiceUrl: stripeInvoice.hosted_invoice_url || undefined,
      pdfUrl: stripeInvoice.invoice_pdf || undefined,
      lineItems: stripeInvoice.lines.data.map((line) => ({
        description: line.description || 'Subscription',
        quantity: line.quantity || 1,
        unitPrice: (line.unit_amount_excluding_tax || line.amount) / 100,
        amount: line.amount / 100,
      })),
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    if (existingInvoice.rows.length > 0) {
      await this.db.query(
        `UPDATE invoices SET
          status = $1, amount_paid = $2, amount_remaining = $3,
          paid_at = $4, updated_at = NOW()
        WHERE id = $5`,
        [invoice.status, invoice.amountPaid, invoice.amountRemaining, invoice.paidAt, invoice.id]
      );
    } else {
      await this.db.query(
        `INSERT INTO invoices (
          id, tenant_id, subscription_id, stripe_invoice_id, number,
          status, currency, subtotal, tax, total, amount_due, amount_paid,
          amount_remaining, period_start, period_end, due_date, paid_at,
          hosted_invoice_url, pdf_url, line_items, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)`,
        [
          invoice.id,
          invoice.tenantId,
          invoice.subscriptionId,
          invoice.stripeInvoiceId,
          invoice.number,
          invoice.status,
          invoice.currency,
          invoice.subtotal,
          invoice.tax,
          invoice.total,
          invoice.amountDue,
          invoice.amountPaid,
          invoice.amountRemaining,
          invoice.periodStart,
          invoice.periodEnd,
          invoice.dueDate,
          invoice.paidAt,
          invoice.hostedInvoiceUrl,
          invoice.pdfUrl,
          JSON.stringify(invoice.lineItems),
          invoice.createdAt,
          invoice.updatedAt,
        ]
      );
    }

    return invoice;
  }

  // ===========================================
  // Helper Methods
  // ===========================================

  private async setQuotaForPlan(
    tenantId: string,
    planType: PlanType,
    periodStart: Date,
    periodEnd: Date
  ): Promise<void> {
    const plan = PLANS[planType];
    if (!plan) return;

    await this.db.query(
      `INSERT INTO usage_quotas (
        tenant_id, api_calls_limit, input_tokens_limit, output_tokens_limit,
        storage_bytes_limit, compute_seconds_limit, period_start, period_end
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      ON CONFLICT (tenant_id, period_start) DO UPDATE SET
        api_calls_limit = EXCLUDED.api_calls_limit,
        input_tokens_limit = EXCLUDED.input_tokens_limit,
        output_tokens_limit = EXCLUDED.output_tokens_limit,
        storage_bytes_limit = EXCLUDED.storage_bytes_limit,
        compute_seconds_limit = EXCLUDED.compute_seconds_limit`,
      [
        tenantId,
        plan.limits.apiCallsPerMonth,
        plan.limits.inputTokensPerMonth,
        plan.limits.outputTokensPerMonth,
        plan.limits.storageGb ? plan.limits.storageGb * 1024 * 1024 * 1024 : null,
        plan.limits.computeHoursPerMonth ? plan.limits.computeHoursPerMonth * 3600 : null,
        periodStart,
        periodEnd,
      ]
    );
  }

  private async recordBillingEvent(
    event: Omit<BillingEvent, 'id' | 'timestamp'>
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO billing_events (id, type, tenant_id, subscription_id, invoice_id, data, timestamp)
       VALUES ($1, $2, $3, $4, $5, $6, NOW())`,
      [uuidv4(), event.type, event.tenantId, event.subscriptionId, event.invoiceId, JSON.stringify(event.data)]
    );
  }

  private mapStripeStatus(status: Stripe.Subscription.Status): SubscriptionStatus {
    const statusMap: Record<string, SubscriptionStatus> = {
      active: SubscriptionStatus.ACTIVE,
      past_due: SubscriptionStatus.PAST_DUE,
      canceled: SubscriptionStatus.CANCELED,
      incomplete: SubscriptionStatus.INCOMPLETE,
      incomplete_expired: SubscriptionStatus.INCOMPLETE_EXPIRED,
      trialing: SubscriptionStatus.TRIALING,
      paused: SubscriptionStatus.PAUSED,
    };
    return statusMap[status] || SubscriptionStatus.ACTIVE;
  }

  private mapInvoiceStatus(status: Stripe.Invoice.Status | null): InvoiceStatus {
    const statusMap: Record<string, InvoiceStatus> = {
      draft: InvoiceStatus.DRAFT,
      open: InvoiceStatus.OPEN,
      paid: InvoiceStatus.PAID,
      void: InvoiceStatus.VOID,
      uncollectible: InvoiceStatus.UNCOLLECTIBLE,
    };
    return statusMap[status || 'draft'] || InvoiceStatus.DRAFT;
  }

  private mapSubscriptionRow(row: any): Subscription {
    return {
      id: row.id,
      tenantId: row.tenant_id,
      planId: row.plan_id,
      planType: row.plan_type,
      status: row.status,
      stripeSubscriptionId: row.stripe_subscription_id,
      stripeCustomerId: row.stripe_customer_id,
      currentPeriodStart: row.current_period_start,
      currentPeriodEnd: row.current_period_end,
      cancelAtPeriodEnd: row.cancel_at_period_end,
      canceledAt: row.canceled_at,
      trialStart: row.trial_start,
      trialEnd: row.trial_end,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private mapInvoiceRow(row: any): Invoice {
    return {
      id: row.id,
      tenantId: row.tenant_id,
      subscriptionId: row.subscription_id,
      stripeInvoiceId: row.stripe_invoice_id,
      number: row.number,
      status: row.status,
      currency: row.currency,
      subtotal: parseFloat(row.subtotal),
      tax: parseFloat(row.tax),
      total: parseFloat(row.total),
      amountDue: parseFloat(row.amount_due),
      amountPaid: parseFloat(row.amount_paid),
      amountRemaining: parseFloat(row.amount_remaining),
      periodStart: row.period_start,
      periodEnd: row.period_end,
      dueDate: row.due_date,
      paidAt: row.paid_at,
      hostedInvoiceUrl: row.hosted_invoice_url,
      pdfUrl: row.pdf_url,
      lineItems: row.line_items,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private mapPaymentMethodRow(row: any): PaymentMethod {
    return {
      id: row.id,
      tenantId: row.tenant_id,
      type: row.type,
      stripePaymentMethodId: row.stripe_payment_method_id,
      isDefault: row.is_default,
      card: row.card_brand
        ? {
            brand: row.card_brand,
            last4: row.card_last4,
            expMonth: row.card_exp_month,
            expYear: row.card_exp_year,
          }
        : undefined,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
