/**
 * Type definitions for Admin Dashboard
 */

// User and Authentication
export interface User {
  id: string;
  email: string;
  name: string;
  role: 'admin' | 'operator' | 'viewer';
  tenantId?: string;
  createdAt: string;
  lastLoginAt?: string;
}

export interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

// Tenant Management
export interface Tenant {
  id: string;
  name: string;
  email: string;
  planType: PlanType;
  status: TenantStatus;
  stripeCustomerId?: string;
  createdAt: string;
  updatedAt: string;
}

export type TenantStatus = 'active' | 'suspended' | 'deleted';

// Subscriptions
export interface Subscription {
  id: string;
  tenantId: string;
  planId: string;
  planType: PlanType;
  status: SubscriptionStatus;
  currentPeriodStart: string;
  currentPeriodEnd: string;
  cancelAtPeriodEnd: boolean;
  trialEnd?: string;
  createdAt: string;
}

export type PlanType = 'free' | 'starter' | 'professional' | 'enterprise' | 'custom';
export type SubscriptionStatus = 'active' | 'past_due' | 'canceled' | 'trialing' | 'paused';

// Usage and Billing
export interface UsageSummary {
  tenantId: string;
  periodStart: string;
  periodEnd: string;
  apiCalls: number;
  inputTokens: number;
  outputTokens: number;
  storageBytes: number;
  computeSeconds: number;
  embeddingTokens: number;
  workflowRuns: number;
  contextSearches: number;
}

export interface UsageAggregation {
  period: string;
  type: UsageType;
  totalQuantity: number;
  count: number;
  avgQuantity: number;
  maxQuantity: number;
}

export type UsageType =
  | 'api_call'
  | 'token_input'
  | 'token_output'
  | 'storage'
  | 'compute'
  | 'embedding'
  | 'workflow_run'
  | 'context_search';

// Invoices
export interface Invoice {
  id: string;
  tenantId: string;
  number: string;
  status: InvoiceStatus;
  total: number;
  amountPaid: number;
  amountDue: number;
  periodStart: string;
  periodEnd: string;
  dueDate?: string;
  paidAt?: string;
  pdfUrl?: string;
  createdAt: string;
}

export type InvoiceStatus = 'draft' | 'open' | 'paid' | 'void' | 'uncollectible';

// Dashboard Stats
export interface DashboardStats {
  totalTenants: number;
  activeTenants: number;
  totalRevenue: number;
  monthlyRevenue: number;
  totalApiCalls: number;
  totalTokens: number;
  systemHealth: {
    status: 'healthy' | 'degraded' | 'down';
    uptime: number;
    services: ServiceHealth[];
  };
}

export interface ServiceHealth {
  name: string;
  status: 'healthy' | 'degraded' | 'down';
  latency: number;
  lastCheck: string;
}

// API Response types
export interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
}

export interface PaginatedResponse<T> {
  success: boolean;
  data: T[];
  pagination: {
    total: number;
    page: number;
    pageSize: number;
    totalPages: number;
  };
}

// Support Tickets
export interface Ticket {
  id: string;
  tenantId: string;
  userId: string;
  subject: string;
  description: string;
  status: TicketStatus;
  priority: TicketPriority;
  category: string;
  assigneeId?: string;
  createdAt: string;
  updatedAt: string;
  resolvedAt?: string;
}

export type TicketStatus = 'open' | 'in_progress' | 'waiting' | 'resolved' | 'closed';
export type TicketPriority = 'low' | 'medium' | 'high' | 'urgent';

// Incidents
export interface Incident {
  id: string;
  title: string;
  description: string;
  status: IncidentStatus;
  severity: IncidentSeverity;
  affectedServices: string[];
  createdAt: string;
  updatedAt: string;
  resolvedAt?: string;
  updates: IncidentUpdate[];
}

export type IncidentStatus = 'investigating' | 'identified' | 'monitoring' | 'resolved';
export type IncidentSeverity = 'minor' | 'major' | 'critical';

export interface IncidentUpdate {
  id: string;
  incidentId: string;
  message: string;
  status: IncidentStatus;
  createdAt: string;
  createdBy: string;
}
