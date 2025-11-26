/**
 * Subscription and Billing Models
 *
 * Models for subscriptions, invoices, and payment processing.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum SubscriptionStatus {
  ACTIVE = 'active',
  PAST_DUE = 'past_due',
  CANCELED = 'canceled',
  INCOMPLETE = 'incomplete',
  INCOMPLETE_EXPIRED = 'incomplete_expired',
  TRIALING = 'trialing',
  PAUSED = 'paused',
}

export enum PlanType {
  FREE = 'free',
  STARTER = 'starter',
  PROFESSIONAL = 'professional',
  ENTERPRISE = 'enterprise',
  CUSTOM = 'custom',
}

export enum InvoiceStatus {
  DRAFT = 'draft',
  OPEN = 'open',
  PAID = 'paid',
  VOID = 'void',
  UNCOLLECTIBLE = 'uncollectible',
}

export enum PaymentMethodType {
  CARD = 'card',
  BANK_TRANSFER = 'bank_transfer',
  INVOICE = 'invoice',
}

// ===========================================
// Plan Definitions
// ===========================================

export interface PlanLimits {
  apiCallsPerMonth: number | null;
  inputTokensPerMonth: number | null;
  outputTokensPerMonth: number | null;
  storageGb: number | null;
  computeHoursPerMonth: number | null;
  workflowsPerMonth: number | null;
  usersIncluded: number;
  maxUsers: number | null;
}

export interface Plan {
  id: string;
  name: string;
  type: PlanType;
  description: string;
  priceMonthly: number;
  priceYearly: number;
  limits: PlanLimits;
  features: string[];
  isActive: boolean;
  stripePriceIdMonthly?: string;
  stripePriceIdYearly?: string;
}

export const PLANS: Record<PlanType, Plan> = {
  [PlanType.FREE]: {
    id: 'plan_free',
    name: 'Free',
    type: PlanType.FREE,
    description: 'Get started with basic features',
    priceMonthly: 0,
    priceYearly: 0,
    limits: {
      apiCallsPerMonth: 1000,
      inputTokensPerMonth: 100000,
      outputTokensPerMonth: 50000,
      storageGb: 1,
      computeHoursPerMonth: 1,
      workflowsPerMonth: 10,
      usersIncluded: 1,
      maxUsers: 1,
    },
    features: [
      'Basic AI assistance',
      '1,000 API calls/month',
      '100K input tokens/month',
      '1 GB storage',
      'Community support',
    ],
    isActive: true,
  },
  [PlanType.STARTER]: {
    id: 'plan_starter',
    name: 'Starter',
    type: PlanType.STARTER,
    description: 'For individuals and small teams',
    priceMonthly: 29,
    priceYearly: 290,
    limits: {
      apiCallsPerMonth: 10000,
      inputTokensPerMonth: 1000000,
      outputTokensPerMonth: 500000,
      storageGb: 10,
      computeHoursPerMonth: 10,
      workflowsPerMonth: 100,
      usersIncluded: 3,
      maxUsers: 5,
    },
    features: [
      'Everything in Free',
      '10,000 API calls/month',
      '1M input tokens/month',
      '10 GB storage',
      'Email support',
      'Basic analytics',
    ],
    isActive: true,
  },
  [PlanType.PROFESSIONAL]: {
    id: 'plan_professional',
    name: 'Professional',
    type: PlanType.PROFESSIONAL,
    description: 'For growing teams',
    priceMonthly: 99,
    priceYearly: 990,
    limits: {
      apiCallsPerMonth: 100000,
      inputTokensPerMonth: 10000000,
      outputTokensPerMonth: 5000000,
      storageGb: 100,
      computeHoursPerMonth: 100,
      workflowsPerMonth: 1000,
      usersIncluded: 10,
      maxUsers: 25,
    },
    features: [
      'Everything in Starter',
      '100,000 API calls/month',
      '10M input tokens/month',
      '100 GB storage',
      'Priority support',
      'Advanced analytics',
      'Custom workflows',
      'API access',
    ],
    isActive: true,
  },
  [PlanType.ENTERPRISE]: {
    id: 'plan_enterprise',
    name: 'Enterprise',
    type: PlanType.ENTERPRISE,
    description: 'For large organizations',
    priceMonthly: 499,
    priceYearly: 4990,
    limits: {
      apiCallsPerMonth: null,
      inputTokensPerMonth: null,
      outputTokensPerMonth: null,
      storageGb: null,
      computeHoursPerMonth: null,
      workflowsPerMonth: null,
      usersIncluded: 50,
      maxUsers: null,
    },
    features: [
      'Everything in Professional',
      'Unlimited API calls',
      'Unlimited tokens',
      'Unlimited storage',
      'Dedicated support',
      'SLA guarantee',
      'Custom integrations',
      'SSO/SAML',
      'Audit logs',
      'Custom contracts',
    ],
    isActive: true,
  },
  [PlanType.CUSTOM]: {
    id: 'plan_custom',
    name: 'Custom',
    type: PlanType.CUSTOM,
    description: 'Tailored to your needs',
    priceMonthly: 0,
    priceYearly: 0,
    limits: {
      apiCallsPerMonth: null,
      inputTokensPerMonth: null,
      outputTokensPerMonth: null,
      storageGb: null,
      computeHoursPerMonth: null,
      workflowsPerMonth: null,
      usersIncluded: 0,
      maxUsers: null,
    },
    features: ['Custom pricing', 'Custom limits', 'Custom features'],
    isActive: true,
  },
};

// ===========================================
// Schemas
// ===========================================

export const SubscriptionSchema = z.object({
  id: z.string().uuid(),
  tenantId: z.string().uuid(),
  planId: z.string(),
  planType: z.nativeEnum(PlanType),
  status: z.nativeEnum(SubscriptionStatus),
  stripeSubscriptionId: z.string().optional(),
  stripeCustomerId: z.string().optional(),
  currentPeriodStart: z.date(),
  currentPeriodEnd: z.date(),
  cancelAtPeriodEnd: z.boolean(),
  canceledAt: z.date().optional(),
  trialStart: z.date().optional(),
  trialEnd: z.date().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const InvoiceSchema = z.object({
  id: z.string().uuid(),
  tenantId: z.string().uuid(),
  subscriptionId: z.string().uuid(),
  stripeInvoiceId: z.string().optional(),
  number: z.string(),
  status: z.nativeEnum(InvoiceStatus),
  currency: z.string().default('usd'),
  subtotal: z.number(),
  tax: z.number().default(0),
  total: z.number(),
  amountDue: z.number(),
  amountPaid: z.number(),
  amountRemaining: z.number(),
  periodStart: z.date(),
  periodEnd: z.date(),
  dueDate: z.date().optional(),
  paidAt: z.date().optional(),
  hostedInvoiceUrl: z.string().optional(),
  pdfUrl: z.string().optional(),
  lineItems: z.array(z.object({
    description: z.string(),
    quantity: z.number(),
    unitPrice: z.number(),
    amount: z.number(),
  })),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const PaymentMethodSchema = z.object({
  id: z.string().uuid(),
  tenantId: z.string().uuid(),
  type: z.nativeEnum(PaymentMethodType),
  stripePaymentMethodId: z.string().optional(),
  isDefault: z.boolean(),
  card: z.object({
    brand: z.string(),
    last4: z.string(),
    expMonth: z.number(),
    expYear: z.number(),
  }).optional(),
  bankAccount: z.object({
    bankName: z.string(),
    last4: z.string(),
  }).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

// ===========================================
// Types
// ===========================================

export type Subscription = z.infer<typeof SubscriptionSchema>;
export type Invoice = z.infer<typeof InvoiceSchema>;
export type PaymentMethod = z.infer<typeof PaymentMethodSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateSubscriptionInput {
  tenantId: string;
  planType: PlanType;
  paymentMethodId?: string;
  trialDays?: number;
}

export interface UpdateSubscriptionInput {
  planType?: PlanType;
  cancelAtPeriodEnd?: boolean;
}

export interface CreatePaymentMethodInput {
  tenantId: string;
  stripePaymentMethodId: string;
  setAsDefault?: boolean;
}

// ===========================================
// Billing Events
// ===========================================

export enum BillingEventType {
  SUBSCRIPTION_CREATED = 'subscription.created',
  SUBSCRIPTION_UPDATED = 'subscription.updated',
  SUBSCRIPTION_CANCELED = 'subscription.canceled',
  SUBSCRIPTION_RENEWED = 'subscription.renewed',
  INVOICE_CREATED = 'invoice.created',
  INVOICE_PAID = 'invoice.paid',
  INVOICE_FAILED = 'invoice.failed',
  PAYMENT_SUCCEEDED = 'payment.succeeded',
  PAYMENT_FAILED = 'payment.failed',
  QUOTA_WARNING = 'quota.warning',
  QUOTA_EXCEEDED = 'quota.exceeded',
}

export interface BillingEvent {
  id: string;
  type: BillingEventType;
  tenantId: string;
  subscriptionId?: string;
  invoiceId?: string;
  data: Record<string, unknown>;
  timestamp: Date;
}
