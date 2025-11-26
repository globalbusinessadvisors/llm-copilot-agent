/**
 * Alert Rules Routes
 *
 * REST API endpoints for alert rule management.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { z } from 'zod';
import { AlertService } from '../services/alertService';
import {
  AlertSeverity,
  AlertConditionType,
  AlertChannel,
  CreateAlertRuleInput,
} from '../models/alert';

const router = Router();

// Request validation schemas
const CreateRuleSchema = z.object({
  name: z.string().min(1).max(255),
  description: z.string().optional(),
  conditionType: z.nativeEnum(AlertConditionType),
  condition: z.object({
    metric: z.string().min(1),
    operator: z.enum(['gt', 'gte', 'lt', 'lte', 'eq', 'neq']),
    threshold: z.number(),
    duration: z.number().optional(),
    aggregation: z.enum(['avg', 'sum', 'min', 'max', 'count']).optional(),
  }),
  severity: z.nativeEnum(AlertSeverity),
  tags: z.record(z.string()).optional(),
  escalationPolicyId: z.string().uuid().optional(),
  notificationChannels: z.array(z.nativeEnum(AlertChannel)).min(1),
  cooldownPeriod: z.number().min(0).default(300),
});

const UpdateRuleSchema = CreateRuleSchema.partial();

export function createRuleRoutes(alertService: AlertService): Router {
  /**
   * Create a new alert rule
   * POST /api/v1/rules
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const input = CreateRuleSchema.parse(req.body);
      const rule = await alertService.createAlertRule(input as CreateAlertRuleInput);

      res.status(201).json({
        success: true,
        data: rule,
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
   * Get all alert rules
   * GET /api/v1/rules
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const rules = await alertService.getAlertRules();

      res.json({
        success: true,
        data: rules,
        meta: {
          total: rules.length,
        },
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get alert rule by ID
   * GET /api/v1/rules/:ruleId
   */
  router.get('/:ruleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ruleId } = req.params;
      const rule = await alertService.getAlertRule(ruleId);

      if (!rule) {
        res.status(404).json({
          success: false,
          error: 'Alert rule not found',
        });
        return;
      }

      res.json({
        success: true,
        data: rule,
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Enable an alert rule
   * POST /api/v1/rules/:ruleId/enable
   */
  router.post('/:ruleId/enable', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ruleId } = req.params;

      const rule = await alertService.getAlertRule(ruleId);
      if (!rule) {
        res.status(404).json({
          success: false,
          error: 'Alert rule not found',
        });
        return;
      }

      await alertService.setRuleEnabled(ruleId, true);

      res.json({
        success: true,
        message: 'Alert rule enabled',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Disable an alert rule
   * POST /api/v1/rules/:ruleId/disable
   */
  router.post('/:ruleId/disable', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ruleId } = req.params;

      const rule = await alertService.getAlertRule(ruleId);
      if (!rule) {
        res.status(404).json({
          success: false,
          error: 'Alert rule not found',
        });
        return;
      }

      await alertService.setRuleEnabled(ruleId, false);

      res.json({
        success: true,
        message: 'Alert rule disabled',
      });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Test an alert rule (dry run)
   * POST /api/v1/rules/:ruleId/test
   */
  router.post('/:ruleId/test', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { ruleId } = req.params;

      const rule = await alertService.getAlertRule(ruleId);
      if (!rule) {
        res.status(404).json({
          success: false,
          error: 'Alert rule not found',
        });
        return;
      }

      // Evaluate the rule condition against provided test data
      const testData = req.body.testData || {};
      const metric = testData[rule.condition.metric];

      if (metric === undefined) {
        res.status(400).json({
          success: false,
          error: `Test data must include metric: ${rule.condition.metric}`,
        });
        return;
      }

      let wouldTrigger = false;
      const threshold = rule.condition.threshold;

      switch (rule.condition.operator) {
        case 'gt':
          wouldTrigger = metric > threshold;
          break;
        case 'gte':
          wouldTrigger = metric >= threshold;
          break;
        case 'lt':
          wouldTrigger = metric < threshold;
          break;
        case 'lte':
          wouldTrigger = metric <= threshold;
          break;
        case 'eq':
          wouldTrigger = metric === threshold;
          break;
        case 'neq':
          wouldTrigger = metric !== threshold;
          break;
      }

      res.json({
        success: true,
        data: {
          rule: {
            id: rule.id,
            name: rule.name,
            condition: rule.condition,
          },
          testData: {
            metric: rule.condition.metric,
            value: metric,
            threshold,
            operator: rule.condition.operator,
          },
          result: {
            wouldTrigger,
            message: wouldTrigger
              ? `Alert would trigger: ${metric} ${rule.condition.operator} ${threshold}`
              : `Alert would NOT trigger: ${metric} is not ${rule.condition.operator} ${threshold}`,
          },
        },
      });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
