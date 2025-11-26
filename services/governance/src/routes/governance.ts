/**
 * Governance Routes
 *
 * API endpoints for content filtering, policies, audit trail, and data lineage.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { ContentFilterService } from '../services/contentFilterService';
import { PolicyService } from '../services/policyService';
import { AuditService } from '../services/auditService';
import { DataLineageService } from '../services/dataLineageService';
import {
  ContentCategory,
  FilterAction,
  FilterDirection,
  PolicyType,
  PolicyScope,
  PolicyEnforcement,
  AuditEventType,
  AuditSeverity,
  LineageNodeType,
  LineageEdgeType,
} from '../models/governance';

export function createGovernanceRoutes(
  contentFilterService: ContentFilterService,
  policyService: PolicyService,
  auditService: AuditService,
  dataLineageService: DataLineageService
): Router {
  const router = Router();

  // ===========================================
  // Content Filter Routes
  // ===========================================

  /**
   * List content filter rules
   */
  router.get('/filters/rules', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { category, direction, enabled } = req.query;
      const rules = await contentFilterService.listRules({
        category: category as ContentCategory,
        direction: direction as FilterDirection,
        enabled: enabled === undefined ? undefined : enabled === 'true',
      });
      res.json({ rules, count: rules.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get content filter rule by ID
   */
  router.get('/filters/rules/:ruleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const rule = await contentFilterService.getRule(req.params.ruleId);
      if (!rule) {
        return res.status(404).json({ error: 'Rule not found' });
      }
      res.json({ rule });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create content filter rule
   */
  router.post('/filters/rules', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const rule = await contentFilterService.createRule(req.body, userId);
      res.status(201).json({ rule });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update content filter rule
   */
  router.patch('/filters/rules/:ruleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const rule = await contentFilterService.updateRule(req.params.ruleId, req.body);
      res.json({ rule });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete content filter rule
   */
  router.delete('/filters/rules/:ruleId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await contentFilterService.deleteRule(req.params.ruleId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Filter content
   */
  router.post('/filters/analyze', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { content, direction, context } = req.body;

      if (!content) {
        return res.status(400).json({ error: 'content is required' });
      }
      if (!direction || !Object.values(FilterDirection).includes(direction)) {
        return res.status(400).json({ error: 'Valid direction is required (input, output, both)' });
      }

      const userId = (req as any).user?.id;
      const result = await contentFilterService.filterContent({
        content,
        direction,
        userId,
        context,
      });
      res.json({ result });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get content filter statistics
   */
  router.get('/filters/statistics', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { startDate, endDate, groupBy } = req.query;
      const statistics = await contentFilterService.getStatistics({
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        groupBy: groupBy as 'hour' | 'day' | 'week',
      });
      res.json(statistics);
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Policy Routes
  // ===========================================

  /**
   * List policies
   */
  router.get('/policies', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { type, scope, status, enforcement } = req.query;
      const policies = await policyService.listPolicies({
        type: type as PolicyType,
        scope: scope as PolicyScope,
        status: status as any,
        enforcement: enforcement as PolicyEnforcement,
      });
      res.json({ policies, count: policies.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get policy by ID
   */
  router.get('/policies/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await policyService.getPolicy(req.params.policyId);
      if (!policy) {
        return res.status(404).json({ error: 'Policy not found' });
      }
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create policy
   */
  router.post('/policies', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const policy = await policyService.createPolicy(req.body, userId);
      res.status(201).json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update policy
   */
  router.patch('/policies/:policyId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await policyService.updatePolicy(req.params.policyId, req.body);
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Activate policy
   */
  router.post('/policies/:policyId/activate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await policyService.activatePolicy(req.params.policyId);
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Deprecate policy
   */
  router.post('/policies/:policyId/deprecate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const policy = await policyService.deprecatePolicy(req.params.policyId);
      res.json({ policy });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Evaluate policy
   */
  router.post('/policies/evaluate', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { userId, action, resource, context } = req.body;

      if (!userId || !action || !resource) {
        return res.status(400).json({ error: 'userId, action, and resource are required' });
      }

      const result = await policyService.evaluatePolicy({
        userId,
        action,
        resource,
        context,
      });
      res.json(result);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get policy violations
   */
  router.get('/policies/violations', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { policyId, userId, blocked, startDate, endDate, limit } = req.query;
      const violations = await policyService.getViolations({
        policyId: policyId as string,
        userId: userId as string,
        blocked: blocked === undefined ? undefined : blocked === 'true',
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        limit: limit ? parseInt(limit as string, 10) : undefined,
      });
      res.json({ violations, count: violations.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get policy statistics
   */
  router.get('/policies/statistics', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { policyId, startDate, endDate } = req.query;
      const statistics = await policyService.getStatistics({
        policyId: policyId as string,
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
      });
      res.json(statistics);
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Audit Routes
  // ===========================================

  /**
   * Record audit event
   */
  router.post('/audit/events', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const event = await auditService.recordEvent(req.body);
      res.status(201).json({ event });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get audit event by ID
   */
  router.get('/audit/events/:eventId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const event = await auditService.getEvent(req.params.eventId);
      if (!event) {
        return res.status(404).json({ error: 'Event not found' });
      }
      res.json({ event });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Search audit events
   */
  router.get('/audit/events', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const {
        types, severity, actorId, actorType, resourceType, resourceId,
        outcome, startDate, endDate, search, limit, offset,
      } = req.query;

      const result = await auditService.searchEvents({
        types: types ? (types as string).split(',') as AuditEventType[] : undefined,
        severity: severity ? (severity as string).split(',') as AuditSeverity[] : undefined,
        actorId: actorId as string,
        actorType: actorType as any,
        resourceType: resourceType as string,
        resourceId: resourceId as string,
        outcome: outcome as any,
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        search: search as string,
        limit: limit ? parseInt(limit as string, 10) : undefined,
        offset: offset ? parseInt(offset as string, 10) : undefined,
      });
      res.json(result);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get resource history
   */
  router.get('/audit/resources/:resourceType/:resourceId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { limit } = req.query;
      const events = await auditService.getResourceHistory(
        req.params.resourceType,
        req.params.resourceId,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ events, count: events.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get actor history
   */
  router.get('/audit/actors/:actorId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { limit } = req.query;
      const events = await auditService.getActorHistory(
        req.params.actorId,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ events, count: events.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get audit statistics
   */
  router.get('/audit/statistics', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { startDate, endDate, groupBy } = req.query;
      const statistics = await auditService.getStatistics({
        startDate: startDate ? new Date(startDate as string) : undefined,
        endDate: endDate ? new Date(endDate as string) : undefined,
        groupBy: groupBy as 'hour' | 'day' | 'week',
      });
      res.json(statistics);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Detect anomalies
   */
  router.get('/audit/anomalies', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { windowMinutes } = req.query;
      const anomalies = await auditService.detectAnomalies({
        windowMinutes: windowMinutes ? parseInt(windowMinutes as string, 10) : undefined,
      });
      res.json({ anomalies, count: anomalies.length });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Data Lineage Routes
  // ===========================================

  /**
   * List lineage nodes
   */
  router.get('/lineage/nodes', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { type, system, tags } = req.query;
      const nodes = await dataLineageService.listNodes({
        type: type as LineageNodeType,
        system: system as string,
        tags: tags ? (tags as string).split(',') : undefined,
      });
      res.json({ nodes, count: nodes.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get lineage node by ID
   */
  router.get('/lineage/nodes/:nodeId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const node = await dataLineageService.getNode(req.params.nodeId);
      if (!node) {
        return res.status(404).json({ error: 'Node not found' });
      }
      res.json({ node });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create lineage node
   */
  router.post('/lineage/nodes', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const node = await dataLineageService.createNode(req.body);
      res.status(201).json({ node });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update lineage node
   */
  router.patch('/lineage/nodes/:nodeId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const node = await dataLineageService.updateNode(req.params.nodeId, req.body);
      res.json({ node });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete lineage node
   */
  router.delete('/lineage/nodes/:nodeId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await dataLineageService.deleteNode(req.params.nodeId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * List lineage edges
   */
  router.get('/lineage/edges', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { sourceNodeId, targetNodeId, type } = req.query;
      const edges = await dataLineageService.listEdges({
        sourceNodeId: sourceNodeId as string,
        targetNodeId: targetNodeId as string,
        type: type as LineageEdgeType,
      });
      res.json({ edges, count: edges.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create lineage edge
   */
  router.post('/lineage/edges', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const edge = await dataLineageService.createEdge(req.body);
      res.status(201).json({ edge });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete lineage edge
   */
  router.delete('/lineage/edges/:edgeId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await dataLineageService.deleteEdge(req.params.edgeId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get lineage graph for a node
   */
  router.get('/lineage/nodes/:nodeId/graph', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { direction, depth } = req.query;
      const graph = await dataLineageService.getLineageGraph(req.params.nodeId, {
        direction: direction as 'upstream' | 'downstream' | 'both',
        depth: depth ? parseInt(depth as string, 10) : undefined,
      });
      res.json({ graph });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Analyze impact for a node
   */
  router.get('/lineage/nodes/:nodeId/impact', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const impact = await dataLineageService.analyzeImpact(req.params.nodeId);
      res.json({ impact });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Find path between nodes
   */
  router.get('/lineage/path', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { sourceId, targetId } = req.query;

      if (!sourceId || !targetId) {
        return res.status(400).json({ error: 'sourceId and targetId are required' });
      }

      const path = await dataLineageService.findPath(sourceId as string, targetId as string);
      res.json(path);
    } catch (error) {
      next(error);
    }
  });

  /**
   * Search lineage nodes
   */
  router.get('/lineage/search', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { query, limit } = req.query;

      if (!query) {
        return res.status(400).json({ error: 'query is required' });
      }

      const nodes = await dataLineageService.searchNodes(
        query as string,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ nodes, count: nodes.length });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get lineage statistics
   */
  router.get('/lineage/statistics', async (_req: Request, res: Response, next: NextFunction) => {
    try {
      const statistics = await dataLineageService.getStatistics();
      res.json(statistics);
    } catch (error) {
      next(error);
    }
  });

  return router;
}
