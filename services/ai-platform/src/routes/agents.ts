/**
 * Agent and Team Routes
 *
 * API endpoints for agent management, execution, and multi-agent collaboration.
 */

import { Router, Request, Response, NextFunction } from 'express';
import { AgentService } from '../services/agentService';
import { TeamService } from '../services/teamService';
import { ToolService } from '../services/toolService';
import { AgentType, CollaborationPattern } from '../models/agent';

export function createAgentRoutes(
  agentService: AgentService,
  teamService: TeamService,
  toolService: ToolService
): Router {
  const router = Router();

  // ===========================================
  // Agent Routes
  // ===========================================

  /**
   * List agents
   */
  router.get('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, type } = req.query;
      const agents = await agentService.listAgents({
        status: status as any,
        type: type as AgentType,
      });
      res.json({ agents });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get agent by ID
   */
  router.get('/:agentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const agent = await agentService.getAgent(req.params.agentId);
      if (!agent) {
        return res.status(404).json({ error: 'Agent not found' });
      }
      res.json({ agent });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create agent
   */
  router.post('/', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const agent = await agentService.createAgent(req.body, userId);
      res.status(201).json({ agent });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update agent
   */
  router.patch('/:agentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const agent = await agentService.updateAgent(req.params.agentId, req.body);
      res.json({ agent });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Archive agent
   */
  router.delete('/:agentId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await agentService.archiveAgent(req.params.agentId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Execute agent
   */
  router.post('/:agentId/execute', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const execution = await agentService.executeAgent(
        req.params.agentId,
        req.body,
        userId
      );
      res.status(201).json({ execution });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get agent execution
   */
  router.get('/executions/:executionId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const execution = await agentService.getExecution(req.params.executionId);
      if (!execution) {
        return res.status(404).json({ error: 'Execution not found' });
      }
      res.json({ execution });
    } catch (error) {
      next(error);
    }
  });

  /**
   * List agent executions
   */
  router.get('/:agentId/executions', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { limit } = req.query;
      const executions = await agentService.listExecutions(
        req.params.agentId,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ executions });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Cancel agent execution
   */
  router.post('/executions/:executionId/cancel', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await agentService.cancelExecution(req.params.executionId);
      res.json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Team Routes
  // ===========================================

  /**
   * List teams
   */
  router.get('/teams', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, pattern } = req.query;
      const teams = await teamService.listTeams({
        status: status as any,
        pattern: pattern as CollaborationPattern,
      });
      res.json({ teams });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get team by ID
   */
  router.get('/teams/:teamId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const team = await teamService.getTeam(req.params.teamId);
      if (!team) {
        return res.status(404).json({ error: 'Team not found' });
      }
      res.json({ team });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create team
   */
  router.post('/teams', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const team = await teamService.createTeam(req.body, userId);
      res.status(201).json({ team });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update team
   */
  router.patch('/teams/:teamId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const team = await teamService.updateTeam(req.params.teamId, req.body);
      res.json({ team });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Archive team
   */
  router.delete('/teams/:teamId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await teamService.archiveTeam(req.params.teamId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Execute team
   */
  router.post('/teams/:teamId/execute', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const execution = await teamService.executeTeam(
        req.params.teamId,
        req.body,
        userId
      );
      res.status(201).json({ execution });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get team execution
   */
  router.get('/teams/executions/:executionId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const execution = await teamService.getExecution(req.params.executionId);
      if (!execution) {
        return res.status(404).json({ error: 'Execution not found' });
      }
      res.json({ execution });
    } catch (error) {
      next(error);
    }
  });

  /**
   * List team executions
   */
  router.get('/teams/:teamId/executions', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { limit } = req.query;
      const executions = await teamService.listExecutions(
        req.params.teamId,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ executions });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Cancel team execution
   */
  router.post('/teams/executions/:executionId/cancel', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await teamService.cancelExecution(req.params.executionId);
      res.json({ success: true });
    } catch (error) {
      next(error);
    }
  });

  // ===========================================
  // Tool Routes
  // ===========================================

  /**
   * List tools
   */
  router.get('/tools', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { status, type } = req.query;
      const tools = await toolService.listTools({
        status: status as any,
        type: type as any,
      });
      res.json({ tools });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get tool by ID
   */
  router.get('/tools/:toolId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const tool = await toolService.getTool(req.params.toolId);
      if (!tool) {
        return res.status(404).json({ error: 'Tool not found' });
      }
      res.json({ tool });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Create tool
   */
  router.post('/tools', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const userId = (req as any).user?.id || 'system';
      const tool = await toolService.createTool(req.body, userId);
      res.status(201).json({ tool });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Update tool
   */
  router.patch('/tools/:toolId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const tool = await toolService.updateTool(req.params.toolId, req.body);
      res.json({ tool });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Delete tool
   */
  router.delete('/tools/:toolId', async (req: Request, res: Response, next: NextFunction) => {
    try {
      await toolService.deleteTool(req.params.toolId);
      res.status(204).send();
    } catch (error) {
      next(error);
    }
  });

  /**
   * Execute tool
   */
  router.post('/tools/:toolId/execute', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { args, context } = req.body;
      const result = await toolService.executeTool(req.params.toolId, args, context);
      res.json({ result });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get tools for LLM (OpenAI function calling format)
   */
  router.post('/tools/llm-format', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { toolIds } = req.body;
      const tools = await toolService.getToolsForLLM(toolIds);
      res.json({ tools });
    } catch (error) {
      next(error);
    }
  });

  /**
   * Get tool call history
   */
  router.get('/tools/:toolId/calls', async (req: Request, res: Response, next: NextFunction) => {
    try {
      const { limit } = req.query;
      const calls = await toolService.getToolCallHistory(
        req.params.toolId,
        limit ? parseInt(limit as string, 10) : undefined
      );
      res.json({ calls });
    } catch (error) {
      next(error);
    }
  });

  return router;
}
