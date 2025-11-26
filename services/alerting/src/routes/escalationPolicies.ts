/**
 * Escalation Policy Routes
 *
 * REST API endpoints for escalation policy management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { Pool } from 'pg';
import { v4 as uuidv4 } from 'uuid';
import { AlertChannel, EscalationPolicy, CreateEscalationPolicyInput } from '../models/alert';

const router = Router();

// Request validation schemas
const CreatePolicySchema = z.object({
  name: z.string().min(1).max(255),
  description: z.string().optional(),
  steps: z.array(z.object({
    order: z.number().min(0),
    delayMinutes: z.number().min(0),
    targets: z.array(z.object({
      type: z.enum(['user', 'schedule', 'webhook']),
      id: z.string(),
      channels: z.array(z.nativeEnum(AlertChannel)).min(1),
    })).min(1),
  })).min(1),
  repeatAfterMinutes: z.number().min(0).optional(),
});

const UpdatePolicySchema = CreatePolicySchema.partial();

export function createEscalationPolicyRoutes(db: Pool): Router {
  /**
   * Create a new escalation policy
   * POST /api/v1/escalation-policies
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreatePolicySchema.parse(req.body);

      const policy: EscalationPolicy = {
        id: uuidv4(),
        name: input.name,
        description: input.description,
        steps: input.steps,
        repeatAfterMinutes: input.repeatAfterMinutes,
        createdAt: new Date(),
        updatedAt: new Date(),
      };

      await db.query(
        `INSERT INTO escalation_policies (
          id, name, description, steps, repeat_after_minutes, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)`,
        [
          policy.id,
          policy.name,
          policy.description,
          JSON.stringify(policy.steps),
          policy.repeatAfterMinutes,
          policy.createdAt,
          policy.updatedAt,
        ]
      );

      res.status(201).json({
        success: true,
        data: policy,
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
   * Get all escalation policies
   * GET /api/v1/escalation-policies
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const result = await db.query(
        `SELECT * FROM escalation_policies ORDER BY name`
      );

      const policies = result.rows.map(mapPolicyRow);

      res.json({
        success: true,
        data: policies,
        meta: {
          total: policies.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get escalation policy by ID
   * GET /api/v1/escalation-policies/:policyId
   */
  router.get('/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { policyId } = req.params;

      const result = await db.query(
        `SELECT * FROM escalation_policies WHERE id = $1`,
        [policyId]
      );

      if (result.rows.length === 0) {
        res.status(404).json({
          success: false,
          error: 'Escalation policy not found',
        });
        return;
      }

      res.json({
        success: true,
        data: mapPolicyRow(result.rows[0]),
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update escalation policy
   * PUT /api/v1/escalation-policies/:policyId
   */
  router.put('/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { policyId } = req.params;
      const updates = UpdatePolicySchema.parse(req.body);

      // Check if policy exists
      const existingResult = await db.query(
        `SELECT * FROM escalation_policies WHERE id = $1`,
        [policyId]
      );

      if (existingResult.rows.length === 0) {
        res.status(404).json({
          success: false,
          error: 'Escalation policy not found',
        });
        return;
      }

      const existing = mapPolicyRow(existingResult.rows[0]);
      const updated: EscalationPolicy = {
        ...existing,
        name: updates.name ?? existing.name,
        description: updates.description ?? existing.description,
        steps: updates.steps ?? existing.steps,
        repeatAfterMinutes: updates.repeatAfterMinutes ?? existing.repeatAfterMinutes,
        updatedAt: new Date(),
      };

      await db.query(
        `UPDATE escalation_policies SET
          name = $1, description = $2, steps = $3, repeat_after_minutes = $4, updated_at = NOW()
        WHERE id = $5`,
        [
          updated.name,
          updated.description,
          JSON.stringify(updated.steps),
          updated.repeatAfterMinutes,
          policyId,
        ]
      );

      res.json({
        success: true,
        data: updated,
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
   * Delete escalation policy
   * DELETE /api/v1/escalation-policies/:policyId
   */
  router.delete('/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { policyId } = req.params;

      // Check if policy is in use
      const inUseResult = await db.query(
        `SELECT COUNT(*) FROM alert_rules WHERE escalation_policy_id = $1`,
        [policyId]
      );

      if (parseInt(inUseResult.rows[0].count, 10) > 0) {
        res.status(400).json({
          success: false,
          error: 'Cannot delete escalation policy that is in use by alert rules',
        });
        return;
      }

      const result = await db.query(
        `DELETE FROM escalation_policies WHERE id = $1 RETURNING id`,
        [policyId]
      );

      if (result.rows.length === 0) {
        res.status(404).json({
          success: false,
          error: 'Escalation policy not found',
        });
        return;
      }

      res.json({
        success: true,
        message: 'Escalation policy deleted',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Validate escalation policy configuration
   * POST /api/v1/escalation-policies/validate
   */
  router.post('/validate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreatePolicySchema.parse(req.body);

      const warnings: string[] = [];
      const errors: string[] = [];

      // Validate step order is sequential
      const sortedSteps = [...input.steps].sort((a, b) => a.order - b.order);
      for (let i = 0; i < sortedSteps.length; i++) {
        if (sortedSteps[i].order !== i) {
          warnings.push(`Step orders should be sequential starting from 0. Found gap at order ${sortedSteps[i].order}`);
        }
      }

      // Validate targets exist
      for (const step of input.steps) {
        for (const target of step.targets) {
          if (target.type === 'user') {
            const userResult = await db.query(
              `SELECT id FROM on_call_users WHERE id = $1`,
              [target.id]
            );
            if (userResult.rows.length === 0) {
              errors.push(`User not found: ${target.id}`);
            }
          } else if (target.type === 'schedule') {
            const scheduleResult = await db.query(
              `SELECT id FROM on_call_schedules WHERE id = $1`,
              [target.id]
            );
            if (scheduleResult.rows.length === 0) {
              errors.push(`Schedule not found: ${target.id}`);
            }
          }
        }
      }

      // Warn if delay is too short
      for (const step of input.steps) {
        if (step.delayMinutes < 5 && step.order > 0) {
          warnings.push(`Step ${step.order} has a delay of only ${step.delayMinutes} minutes. Consider increasing for better response time.`);
        }
      }

      res.json({
        success: true,
        data: {
          valid: errors.length === 0,
          errors,
          warnings,
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

  return router;
}

function mapPolicyRow(row: any): EscalationPolicy {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    steps: row.steps,
    repeatAfterMinutes: row.repeat_after_minutes,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}
