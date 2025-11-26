/**
 * Ticket Models
 *
 * Data models for support tickets and messages.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum TicketStatus {
  OPEN = 'open',
  IN_PROGRESS = 'in_progress',
  WAITING = 'waiting',
  RESOLVED = 'resolved',
  CLOSED = 'closed',
}

export enum TicketPriority {
  LOW = 'low',
  MEDIUM = 'medium',
  HIGH = 'high',
  URGENT = 'urgent',
}

export enum TicketCategory {
  TECHNICAL = 'technical',
  BILLING = 'billing',
  ACCOUNT = 'account',
  FEATURE_REQUEST = 'feature_request',
  BUG_REPORT = 'bug_report',
  DOCUMENTATION = 'documentation',
  OTHER = 'other',
}

// ===========================================
// Schemas
// ===========================================

export const TicketSchema = z.object({
  id: z.string().uuid(),
  tenantId: z.string().uuid(),
  userId: z.string().uuid(),
  subject: z.string().min(1).max(255),
  description: z.string().min(1),
  status: z.nativeEnum(TicketStatus),
  priority: z.nativeEnum(TicketPriority),
  category: z.nativeEnum(TicketCategory),
  assigneeId: z.string().uuid().optional(),
  tags: z.array(z.string()).default([]),
  metadata: z.record(z.unknown()).default({}),
  createdAt: z.date(),
  updatedAt: z.date(),
  resolvedAt: z.date().optional(),
  closedAt: z.date().optional(),
  firstResponseAt: z.date().optional(),
});

export const TicketMessageSchema = z.object({
  id: z.string().uuid(),
  ticketId: z.string().uuid(),
  senderId: z.string().uuid(),
  senderType: z.enum(['user', 'agent', 'system']),
  senderName: z.string(),
  content: z.string().min(1),
  contentHtml: z.string().optional(),
  attachments: z.array(z.object({
    id: z.string(),
    filename: z.string(),
    size: z.number(),
    mimeType: z.string(),
    url: z.string(),
  })).default([]),
  isInternal: z.boolean().default(false),
  createdAt: z.date(),
});

export const TicketActivitySchema = z.object({
  id: z.string().uuid(),
  ticketId: z.string().uuid(),
  actorId: z.string().uuid(),
  actorName: z.string(),
  action: z.string(),
  oldValue: z.string().optional(),
  newValue: z.string().optional(),
  createdAt: z.date(),
});

// ===========================================
// Types
// ===========================================

export type Ticket = z.infer<typeof TicketSchema>;
export type TicketMessage = z.infer<typeof TicketMessageSchema>;
export type TicketActivity = z.infer<typeof TicketActivitySchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateTicketInput {
  tenantId: string;
  userId: string;
  subject: string;
  description: string;
  priority?: TicketPriority;
  category: TicketCategory;
  tags?: string[];
  metadata?: Record<string, unknown>;
}

export interface UpdateTicketInput {
  status?: TicketStatus;
  priority?: TicketPriority;
  category?: TicketCategory;
  assigneeId?: string | null;
  tags?: string[];
}

export interface CreateMessageInput {
  ticketId: string;
  senderId: string;
  senderType: 'user' | 'agent' | 'system';
  senderName: string;
  content: string;
  isInternal?: boolean;
  attachments?: Array<{
    filename: string;
    size: number;
    mimeType: string;
    url: string;
  }>;
}

// ===========================================
// Query Types
// ===========================================

export interface TicketQueryParams {
  tenantId?: string;
  userId?: string;
  assigneeId?: string;
  status?: TicketStatus | TicketStatus[];
  priority?: TicketPriority | TicketPriority[];
  category?: TicketCategory;
  search?: string;
  page?: number;
  pageSize?: number;
  sortBy?: 'createdAt' | 'updatedAt' | 'priority';
  sortOrder?: 'asc' | 'desc';
}

export interface TicketStats {
  total: number;
  open: number;
  inProgress: number;
  waiting: number;
  resolved: number;
  closed: number;
  avgFirstResponseTime: number;
  avgResolutionTime: number;
}
