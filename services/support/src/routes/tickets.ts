/**
 * Ticket Routes
 *
 * REST API endpoints for support ticket management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { TicketService } from '../services/ticketService';
import { TicketPriority, TicketCategory, TicketStatus } from '../models/ticket';

const router = Router();

// Request validation schemas
const CreateTicketSchema = z.object({
  subject: z.string().min(1).max(255),
  description: z.string().min(1),
  category: z.nativeEnum(TicketCategory),
  priority: z.nativeEnum(TicketPriority).optional(),
  metadata: z.record(z.unknown()).optional(),
});

const UpdateTicketSchema = z.object({
  status: z.nativeEnum(TicketStatus).optional(),
  priority: z.nativeEnum(TicketPriority).optional(),
  assigneeId: z.string().uuid().optional().nullable(),
  category: z.nativeEnum(TicketCategory).optional(),
});

const AddMessageSchema = z.object({
  content: z.string().min(1),
  isInternal: z.boolean().optional(),
  attachments: z.array(z.object({
    name: z.string(),
    url: z.string().url(),
    type: z.string(),
    size: z.number(),
  })).optional(),
});

const QueryTicketsSchema = z.object({
  status: z.nativeEnum(TicketStatus).optional(),
  priority: z.nativeEnum(TicketPriority).optional(),
  category: z.nativeEnum(TicketCategory).optional(),
  assigneeId: z.string().uuid().optional(),
  tenantId: z.string().uuid().optional(),
  search: z.string().optional(),
  page: z.coerce.number().min(1).default(1),
  pageSize: z.coerce.number().min(1).max(100).default(20),
});

export function createTicketRoutes(ticketService: TicketService): Router {
  /**
   * Create a new ticket
   * POST /api/v1/tickets
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateTicketSchema.parse(req.body);
      const tenantId = (req as any).tenantId || req.body.tenantId;
      const userId = (req as any).user?.id || req.body.userId;

      if (!tenantId || !userId) {
        res.status(400).json({
          success: false,
          error: 'Tenant ID and User ID are required',
        });
        return;
      }

      const ticket = await ticketService.createTicket({
        ...input,
        tenantId,
        userId,
      });

      res.status(201).json({
        success: true,
        data: ticket,
      });
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
   * Get ticket by ID
   * GET /api/v1/tickets/:ticketId
   */
  router.get('/:ticketId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const ticket = await ticketService.getTicket(ticketId);

      if (!ticket) {
        res.status(404).json({
          success: false,
          error: 'Ticket not found',
        });
        return;
      }

      res.json({
        success: true,
        data: ticket,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Query tickets
   * GET /api/v1/tickets
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const query = QueryTicketsSchema.parse(req.query);
      const result = await ticketService.queryTickets(query);

      res.json({
        success: true,
        data: result.tickets,
        meta: {
          total: result.total,
          page: result.page,
          pageSize: result.pageSize,
          totalPages: Math.ceil(result.total / result.pageSize),
        },
      });
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
   * Update ticket
   * PATCH /api/v1/tickets/:ticketId
   */
  router.patch('/:ticketId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const updates = UpdateTicketSchema.parse(req.body);
      const userId = (req as any).user?.id || 'system';

      const ticket = await ticketService.updateTicket(ticketId, updates, userId);

      if (!ticket) {
        res.status(404).json({
          success: false,
          error: 'Ticket not found',
        });
        return;
      }

      res.json({
        success: true,
        data: ticket,
      });
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
   * Add message to ticket
   * POST /api/v1/tickets/:ticketId/messages
   */
  router.post('/:ticketId/messages', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const input = AddMessageSchema.parse(req.body);
      const userId = (req as any).user?.id || req.body.userId;
      const isStaff = (req as any).user?.isStaff || req.body.isStaff || false;

      if (!userId) {
        res.status(400).json({
          success: false,
          error: 'User ID is required',
        });
        return;
      }

      const message = await ticketService.addMessage(ticketId, {
        ...input,
        userId,
        isStaff,
      });

      res.status(201).json({
        success: true,
        data: message,
      });
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
   * Get ticket messages
   * GET /api/v1/tickets/:ticketId/messages
   */
  router.get('/:ticketId/messages', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const includeInternal = (req as any).user?.isStaff || req.query.includeInternal === 'true';

      const messages = await ticketService.getMessages(ticketId, includeInternal);

      res.json({
        success: true,
        data: messages,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get ticket activity
   * GET /api/v1/tickets/:ticketId/activity
   */
  router.get('/:ticketId/activity', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const activity = await ticketService.getActivity(ticketId);

      res.json({
        success: true,
        data: activity,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Assign ticket
   * POST /api/v1/tickets/:ticketId/assign
   */
  router.post('/:ticketId/assign', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const { assigneeId } = req.body;
      const userId = (req as any).user?.id || 'system';

      const ticket = await ticketService.updateTicket(ticketId, { assigneeId }, userId);

      if (!ticket) {
        res.status(404).json({
          success: false,
          error: 'Ticket not found',
        });
        return;
      }

      res.json({
        success: true,
        data: ticket,
        message: assigneeId ? 'Ticket assigned successfully' : 'Ticket unassigned',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Close ticket
   * POST /api/v1/tickets/:ticketId/close
   */
  router.post('/:ticketId/close', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const { resolution } = req.body;
      const userId = (req as any).user?.id || 'system';

      const ticket = await ticketService.updateTicket(
        ticketId,
        { status: TicketStatus.CLOSED },
        userId
      );

      if (!ticket) {
        res.status(404).json({
          success: false,
          error: 'Ticket not found',
        });
        return;
      }

      // Add resolution message if provided
      if (resolution) {
        await ticketService.addMessage(ticketId, {
          content: `Resolution: ${resolution}`,
          userId,
          isStaff: true,
          isInternal: false,
        });
      }

      res.json({
        success: true,
        data: ticket,
        message: 'Ticket closed successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Reopen ticket
   * POST /api/v1/tickets/:ticketId/reopen
   */
  router.post('/:ticketId/reopen', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ticketId } = req.params;
      const { reason } = req.body;
      const userId = (req as any).user?.id || 'system';

      const ticket = await ticketService.updateTicket(
        ticketId,
        { status: TicketStatus.OPEN },
        userId
      );

      if (!ticket) {
        res.status(404).json({
          success: false,
          error: 'Ticket not found',
        });
        return;
      }

      if (reason) {
        await ticketService.addMessage(ticketId, {
          content: `Ticket reopened: ${reason}`,
          userId,
          isStaff: false,
          isInternal: false,
        });
      }

      res.json({
        success: true,
        data: ticket,
        message: 'Ticket reopened successfully',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get ticket statistics
   * GET /api/v1/tickets/stats/summary
   */
  router.get('/stats/summary', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const tenantId = req.query.tenantId as string | undefined;
      const stats = await ticketService.getStats(tenantId);

      res.json({
        success: true,
        data: stats,
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
