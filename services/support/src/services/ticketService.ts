/**
 * Ticket Service
 *
 * Handles support ticket operations.
 */

import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import { marked } from 'marked';
import sanitizeHtml from 'sanitize-html';
import {
  Ticket,
  TicketMessage,
  TicketActivity,
  TicketStatus,
  TicketPriority,
  TicketCategory,
  CreateTicketInput,
  UpdateTicketInput,
  CreateMessageInput,
  TicketQueryParams,
  TicketStats,
} from '../models/ticket';

export class TicketService {
  private db: Pool;

  constructor(db: Pool) {
    this.db = db;
  }

  /**
   * Create a new support ticket
   */
  async createTicket(input: CreateTicketInput): Promise<Ticket> {
    const ticket: Ticket = {
      id: uuidv4(),
      tenantId: input.tenantId,
      userId: input.userId,
      subject: input.subject,
      description: input.description,
      status: TicketStatus.OPEN,
      priority: input.priority || TicketPriority.MEDIUM,
      category: input.category,
      tags: input.tags || [],
      metadata: input.metadata || {},
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO tickets (
        id, tenant_id, user_id, subject, description, status, priority,
        category, tags, metadata, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        ticket.id,
        ticket.tenantId,
        ticket.userId,
        ticket.subject,
        ticket.description,
        ticket.status,
        ticket.priority,
        ticket.category,
        JSON.stringify(ticket.tags),
        JSON.stringify(ticket.metadata),
        ticket.createdAt,
        ticket.updatedAt,
      ]
    );

    // Log activity
    await this.logActivity(ticket.id, ticket.userId, 'User', 'created', undefined, 'Ticket created');

    return ticket;
  }

  /**
   * Get ticket by ID
   */
  async getTicket(ticketId: string): Promise<Ticket | null> {
    const result = await this.db.query(
      `SELECT * FROM tickets WHERE id = $1`,
      [ticketId]
    );

    if (result.rows.length === 0) return null;

    return this.mapTicketRow(result.rows[0]);
  }

  /**
   * Update a ticket
   */
  async updateTicket(
    ticketId: string,
    input: UpdateTicketInput,
    actorId: string,
    actorName: string
  ): Promise<Ticket | null> {
    const ticket = await this.getTicket(ticketId);
    if (!ticket) return null;

    const updates: string[] = [];
    const values: unknown[] = [];
    let paramIndex = 1;

    // Track changes for activity log
    const changes: Array<{ field: string; old: string; new: string }> = [];

    if (input.status !== undefined && input.status !== ticket.status) {
      updates.push(`status = $${paramIndex++}`);
      values.push(input.status);
      changes.push({ field: 'status', old: ticket.status, new: input.status });

      if (input.status === TicketStatus.RESOLVED) {
        updates.push(`resolved_at = NOW()`);
      }
      if (input.status === TicketStatus.CLOSED) {
        updates.push(`closed_at = NOW()`);
      }
    }

    if (input.priority !== undefined && input.priority !== ticket.priority) {
      updates.push(`priority = $${paramIndex++}`);
      values.push(input.priority);
      changes.push({ field: 'priority', old: ticket.priority, new: input.priority });
    }

    if (input.category !== undefined && input.category !== ticket.category) {
      updates.push(`category = $${paramIndex++}`);
      values.push(input.category);
      changes.push({ field: 'category', old: ticket.category, new: input.category });
    }

    if (input.assigneeId !== undefined) {
      updates.push(`assignee_id = $${paramIndex++}`);
      values.push(input.assigneeId);
      changes.push({
        field: 'assignee',
        old: ticket.assigneeId || 'unassigned',
        new: input.assigneeId || 'unassigned',
      });
    }

    if (input.tags !== undefined) {
      updates.push(`tags = $${paramIndex++}`);
      values.push(JSON.stringify(input.tags));
    }

    if (updates.length === 0) {
      return ticket;
    }

    updates.push(`updated_at = NOW()`);
    values.push(ticketId);

    await this.db.query(
      `UPDATE tickets SET ${updates.join(', ')} WHERE id = $${paramIndex}`,
      values
    );

    // Log activities
    for (const change of changes) {
      await this.logActivity(ticketId, actorId, actorName, `changed_${change.field}`, change.old, change.new);
    }

    return this.getTicket(ticketId);
  }

  /**
   * Query tickets with filters
   */
  async queryTickets(params: TicketQueryParams): Promise<{ tickets: Ticket[]; total: number }> {
    const conditions: string[] = [];
    const values: unknown[] = [];
    let paramIndex = 1;

    if (params.tenantId) {
      conditions.push(`tenant_id = $${paramIndex++}`);
      values.push(params.tenantId);
    }

    if (params.userId) {
      conditions.push(`user_id = $${paramIndex++}`);
      values.push(params.userId);
    }

    if (params.assigneeId) {
      conditions.push(`assignee_id = $${paramIndex++}`);
      values.push(params.assigneeId);
    }

    if (params.status) {
      if (Array.isArray(params.status)) {
        conditions.push(`status = ANY($${paramIndex++})`);
        values.push(params.status);
      } else {
        conditions.push(`status = $${paramIndex++}`);
        values.push(params.status);
      }
    }

    if (params.priority) {
      if (Array.isArray(params.priority)) {
        conditions.push(`priority = ANY($${paramIndex++})`);
        values.push(params.priority);
      } else {
        conditions.push(`priority = $${paramIndex++}`);
        values.push(params.priority);
      }
    }

    if (params.category) {
      conditions.push(`category = $${paramIndex++}`);
      values.push(params.category);
    }

    if (params.search) {
      conditions.push(`(subject ILIKE $${paramIndex} OR description ILIKE $${paramIndex})`);
      values.push(`%${params.search}%`);
      paramIndex++;
    }

    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';

    // Count total
    const countResult = await this.db.query(
      `SELECT COUNT(*) FROM tickets ${whereClause}`,
      values
    );
    const total = parseInt(countResult.rows[0].count, 10);

    // Get paginated results
    const page = params.page || 1;
    const pageSize = Math.min(params.pageSize || 20, 100);
    const offset = (page - 1) * pageSize;
    const sortBy = params.sortBy || 'createdAt';
    const sortOrder = params.sortOrder || 'desc';

    const sortColumn = {
      createdAt: 'created_at',
      updatedAt: 'updated_at',
      priority: 'priority',
    }[sortBy];

    const result = await this.db.query(
      `SELECT * FROM tickets ${whereClause}
       ORDER BY ${sortColumn} ${sortOrder.toUpperCase()}
       LIMIT $${paramIndex++} OFFSET $${paramIndex}`,
      [...values, pageSize, offset]
    );

    return {
      tickets: result.rows.map(this.mapTicketRow),
      total,
    };
  }

  /**
   * Add a message to a ticket
   */
  async addMessage(input: CreateMessageInput): Promise<TicketMessage> {
    const contentHtml = sanitizeHtml(await marked(input.content));

    const message: TicketMessage = {
      id: uuidv4(),
      ticketId: input.ticketId,
      senderId: input.senderId,
      senderType: input.senderType,
      senderName: input.senderName,
      content: input.content,
      contentHtml,
      attachments: input.attachments?.map(a => ({ id: uuidv4(), ...a })) || [],
      isInternal: input.isInternal || false,
      createdAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO ticket_messages (
        id, ticket_id, sender_id, sender_type, sender_name,
        content, content_html, attachments, is_internal, created_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        message.id,
        message.ticketId,
        message.senderId,
        message.senderType,
        message.senderName,
        message.content,
        message.contentHtml,
        JSON.stringify(message.attachments),
        message.isInternal,
        message.createdAt,
      ]
    );

    // Update ticket timestamp and first response time
    const ticket = await this.getTicket(input.ticketId);
    if (ticket) {
      const updates = ['updated_at = NOW()'];

      // Track first response from agent
      if (input.senderType === 'agent' && !ticket.firstResponseAt) {
        updates.push('first_response_at = NOW()');
      }

      // Auto-update status if agent responds to open ticket
      if (input.senderType === 'agent' && ticket.status === TicketStatus.OPEN) {
        updates.push(`status = '${TicketStatus.IN_PROGRESS}'`);
      }

      // If user responds to waiting ticket, set back to in_progress
      if (input.senderType === 'user' && ticket.status === TicketStatus.WAITING) {
        updates.push(`status = '${TicketStatus.IN_PROGRESS}'`);
      }

      await this.db.query(
        `UPDATE tickets SET ${updates.join(', ')} WHERE id = $1`,
        [input.ticketId]
      );
    }

    return message;
  }

  /**
   * Get messages for a ticket
   */
  async getMessages(ticketId: string, includeInternal: boolean = false): Promise<TicketMessage[]> {
    const query = includeInternal
      ? `SELECT * FROM ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC`
      : `SELECT * FROM ticket_messages WHERE ticket_id = $1 AND is_internal = false ORDER BY created_at ASC`;

    const result = await this.db.query(query, [ticketId]);

    return result.rows.map((row): TicketMessage => ({
      id: row.id,
      ticketId: row.ticket_id,
      senderId: row.sender_id,
      senderType: row.sender_type,
      senderName: row.sender_name,
      content: row.content,
      contentHtml: row.content_html,
      attachments: row.attachments,
      isInternal: row.is_internal,
      createdAt: row.created_at,
    }));
  }

  /**
   * Get ticket activity history
   */
  async getActivity(ticketId: string): Promise<TicketActivity[]> {
    const result = await this.db.query(
      `SELECT * FROM ticket_activities WHERE ticket_id = $1 ORDER BY created_at DESC`,
      [ticketId]
    );

    return result.rows.map((row): TicketActivity => ({
      id: row.id,
      ticketId: row.ticket_id,
      actorId: row.actor_id,
      actorName: row.actor_name,
      action: row.action,
      oldValue: row.old_value,
      newValue: row.new_value,
      createdAt: row.created_at,
    }));
  }

  /**
   * Get ticket statistics
   */
  async getStats(tenantId?: string): Promise<TicketStats> {
    const condition = tenantId ? 'WHERE tenant_id = $1' : '';
    const values = tenantId ? [tenantId] : [];

    const result = await this.db.query(
      `SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE status = 'open') as open,
        COUNT(*) FILTER (WHERE status = 'in_progress') as in_progress,
        COUNT(*) FILTER (WHERE status = 'waiting') as waiting,
        COUNT(*) FILTER (WHERE status = 'resolved') as resolved,
        COUNT(*) FILTER (WHERE status = 'closed') as closed,
        AVG(EXTRACT(EPOCH FROM (first_response_at - created_at))) FILTER (WHERE first_response_at IS NOT NULL) as avg_first_response,
        AVG(EXTRACT(EPOCH FROM (resolved_at - created_at))) FILTER (WHERE resolved_at IS NOT NULL) as avg_resolution
      FROM tickets ${condition}`,
      values
    );

    const row = result.rows[0];
    return {
      total: parseInt(row.total, 10),
      open: parseInt(row.open, 10),
      inProgress: parseInt(row.in_progress, 10),
      waiting: parseInt(row.waiting, 10),
      resolved: parseInt(row.resolved, 10),
      closed: parseInt(row.closed, 10),
      avgFirstResponseTime: parseFloat(row.avg_first_response) || 0,
      avgResolutionTime: parseFloat(row.avg_resolution) || 0,
    };
  }

  // ===========================================
  // Private Methods
  // ===========================================

  private async logActivity(
    ticketId: string,
    actorId: string,
    actorName: string,
    action: string,
    oldValue?: string,
    newValue?: string
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO ticket_activities (id, ticket_id, actor_id, actor_name, action, old_value, new_value, created_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())`,
      [uuidv4(), ticketId, actorId, actorName, action, oldValue, newValue]
    );
  }

  private mapTicketRow(row: any): Ticket {
    return {
      id: row.id,
      tenantId: row.tenant_id,
      userId: row.user_id,
      subject: row.subject,
      description: row.description,
      status: row.status,
      priority: row.priority,
      category: row.category,
      assigneeId: row.assignee_id,
      tags: row.tags,
      metadata: row.metadata,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      resolvedAt: row.resolved_at,
      closedAt: row.closed_at,
      firstResponseAt: row.first_response_at,
    };
  }
}
