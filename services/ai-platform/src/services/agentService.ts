/**
 * Agent Service
 *
 * Manages AI agents, their configurations, and executions.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  AgentConfig,
  AgentExecution,
  AgentStatus,
  AgentType,
  CreateAgentInput,
  ExecuteAgentInput,
  ToolCall,
} from '../models/agent';

export class AgentService {
  private db: Pool;
  private redis: RedisClientType;
  private cachePrefix = 'agent:';

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Agent Configuration
  // ===========================================

  /**
   * Create a new agent
   */
  async createAgent(input: CreateAgentInput, userId: string): Promise<AgentConfig> {
    const agent: AgentConfig = {
      id: uuidv4(),
      name: input.name,
      displayName: input.displayName,
      description: input.description,
      type: input.type,
      modelId: input.modelId,
      modelConfig: input.modelConfig || {},
      tools: input.tools || [],
      capabilities: {
        canUseTools: true,
        canDelegateToAgents: false,
        canAccessMemory: true,
        canAccessContext: true,
        maxToolCalls: 10,
        maxDelegations: 5,
        ...input.capabilities,
      },
      memory: {
        type: 'buffer',
        maxMessages: 20,
        includeSystemMessages: true,
        ...input.memory,
      },
      behavior: {
        maxIterations: 10,
        stopOnToolError: false,
        returnIntermediateSteps: false,
        verbose: false,
        ...input.behavior,
      },
      version: '1.0.0',
      tags: input.tags || [],
      enabled: true,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO agents (
        id, name, display_name, description, type, model_id, model_config,
        tools, capabilities, memory, behavior, version, tags, enabled,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)`,
      [
        agent.id, agent.name, agent.displayName, agent.description, agent.type,
        agent.modelId, JSON.stringify(agent.modelConfig), JSON.stringify(agent.tools),
        JSON.stringify(agent.capabilities), JSON.stringify(agent.memory),
        JSON.stringify(agent.behavior), agent.version, JSON.stringify(agent.tags),
        agent.enabled, agent.createdAt, agent.updatedAt, agent.createdBy,
      ]
    );

    return agent;
  }

  /**
   * Get agent by ID
   */
  async getAgent(agentId: string): Promise<AgentConfig | null> {
    const cached = await this.redis.get(`${this.cachePrefix}config:${agentId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    const result = await this.db.query(
      `SELECT * FROM agents WHERE id = $1`,
      [agentId]
    );

    if (result.rows.length === 0) return null;

    const agent = this.mapAgentRow(result.rows[0]);

    await this.redis.set(
      `${this.cachePrefix}config:${agentId}`,
      JSON.stringify(agent),
      { EX: 300 }
    );

    return agent;
  }

  /**
   * List agents
   */
  async listAgents(filters?: {
    type?: AgentType;
    enabled?: boolean;
    tags?: string[];
  }): Promise<AgentConfig[]> {
    let query = `SELECT * FROM agents WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }
    if (filters?.enabled !== undefined) {
      query += ` AND enabled = $${paramIndex++}`;
      values.push(filters.enabled);
    }
    if (filters?.tags?.length) {
      query += ` AND tags ?| $${paramIndex++}`;
      values.push(filters.tags);
    }

    query += ` ORDER BY name ASC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapAgentRow);
  }

  /**
   * Update agent configuration
   */
  async updateAgent(
    agentId: string,
    updates: Partial<CreateAgentInput>
  ): Promise<AgentConfig | null> {
    const agent = await this.getAgent(agentId);
    if (!agent) return null;

    const fields: string[] = [];
    const values: unknown[] = [];
    let paramIndex = 1;

    if (updates.displayName !== undefined) {
      fields.push(`display_name = $${paramIndex++}`);
      values.push(updates.displayName);
    }
    if (updates.description !== undefined) {
      fields.push(`description = $${paramIndex++}`);
      values.push(updates.description);
    }
    if (updates.modelId !== undefined) {
      fields.push(`model_id = $${paramIndex++}`);
      values.push(updates.modelId);
    }
    if (updates.modelConfig !== undefined) {
      fields.push(`model_config = $${paramIndex++}`);
      values.push(JSON.stringify({ ...agent.modelConfig, ...updates.modelConfig }));
    }
    if (updates.tools !== undefined) {
      fields.push(`tools = $${paramIndex++}`);
      values.push(JSON.stringify(updates.tools));
    }
    if (updates.capabilities !== undefined) {
      fields.push(`capabilities = $${paramIndex++}`);
      values.push(JSON.stringify({ ...agent.capabilities, ...updates.capabilities }));
    }
    if (updates.memory !== undefined) {
      fields.push(`memory = $${paramIndex++}`);
      values.push(JSON.stringify({ ...agent.memory, ...updates.memory }));
    }
    if (updates.behavior !== undefined) {
      fields.push(`behavior = $${paramIndex++}`);
      values.push(JSON.stringify({ ...agent.behavior, ...updates.behavior }));
    }
    if (updates.tags !== undefined) {
      fields.push(`tags = $${paramIndex++}`);
      values.push(JSON.stringify(updates.tags));
    }

    if (fields.length === 0) return agent;

    fields.push('updated_at = NOW()');
    values.push(agentId);

    await this.db.query(
      `UPDATE agents SET ${fields.join(', ')} WHERE id = $${paramIndex}`,
      values
    );

    await this.redis.del(`${this.cachePrefix}config:${agentId}`);

    return this.getAgent(agentId);
  }

  /**
   * Enable/disable agent
   */
  async setAgentEnabled(agentId: string, enabled: boolean): Promise<void> {
    await this.db.query(
      `UPDATE agents SET enabled = $1, updated_at = NOW() WHERE id = $2`,
      [enabled, agentId]
    );
    await this.redis.del(`${this.cachePrefix}config:${agentId}`);
  }

  // ===========================================
  // Agent Execution
  // ===========================================

  /**
   * Execute an agent
   */
  async executeAgent(
    agentId: string,
    input: ExecuteAgentInput,
    userId: string
  ): Promise<AgentExecution> {
    const agent = await this.getAgent(agentId);
    if (!agent) {
      throw new Error('Agent not found');
    }
    if (!agent.enabled) {
      throw new Error('Agent is disabled');
    }

    const execution: AgentExecution = {
      id: uuidv4(),
      agentId,
      sessionId: input.sessionId,
      status: AgentStatus.RUNNING,
      input: {
        message: input.message,
        context: input.context,
        tools: input.tools,
      },
      steps: [],
      startedAt: new Date(),
      createdBy: userId,
    };

    // Store execution start
    await this.db.query(
      `INSERT INTO agent_executions (
        id, agent_id, session_id, status, input, steps, started_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        execution.id, execution.agentId, execution.sessionId, execution.status,
        JSON.stringify(execution.input), JSON.stringify(execution.steps),
        execution.startedAt, execution.createdBy,
      ]
    );

    // Track active execution
    await this.redis.incr(`${this.cachePrefix}active:${agentId}`);

    try {
      // Execute the agent loop
      const result = await this.runAgentLoop(agent, execution, input);
      return result;
    } catch (error) {
      // Handle execution error
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      execution.status = AgentStatus.FAILED;
      execution.error = {
        code: 'EXECUTION_ERROR',
        message: errorMessage,
      };
      execution.completedAt = new Date();

      await this.db.query(
        `UPDATE agent_executions SET
          status = $1, error = $2, completed_at = NOW()
        WHERE id = $3`,
        [execution.status, JSON.stringify(execution.error), execution.id]
      );

      throw error;
    } finally {
      await this.redis.decr(`${this.cachePrefix}active:${agentId}`);
    }
  }

  /**
   * Run the agent execution loop
   */
  private async runAgentLoop(
    agent: AgentConfig,
    execution: AgentExecution,
    input: ExecuteAgentInput
  ): Promise<AgentExecution> {
    let iteration = 0;
    let completed = false;

    // Load conversation memory if session exists
    const memory = input.sessionId
      ? await this.loadMemory(input.sessionId, agent.memory)
      : [];

    // Build initial messages
    const messages: Array<{ role: string; content: string }> = [
      ...(agent.modelConfig.systemPrompt
        ? [{ role: 'system', content: agent.modelConfig.systemPrompt }]
        : []),
      ...memory,
      { role: 'user', content: input.message },
    ];

    // Get available tools
    const tools = await this.loadTools(
      input.tools || agent.tools,
      agent.capabilities
    );

    while (!completed && iteration < agent.behavior.maxIterations) {
      iteration++;

      // Record thinking step
      execution.steps.push({
        stepNumber: execution.steps.length + 1,
        type: 'thought',
        content: `Iteration ${iteration}: Processing...`,
        timestamp: new Date(),
      });

      // Call the model
      const response = await this.callModel(
        agent.modelId,
        messages,
        tools,
        agent.modelConfig
      );

      // Check for tool calls
      if (response.toolCalls && response.toolCalls.length > 0) {
        if (!agent.capabilities.canUseTools) {
          throw new Error('Agent is not allowed to use tools');
        }

        if (response.toolCalls.length > agent.capabilities.maxToolCalls) {
          throw new Error(`Tool call limit exceeded: ${agent.capabilities.maxToolCalls}`);
        }

        // Execute each tool call
        for (const toolCall of response.toolCalls) {
          execution.steps.push({
            stepNumber: execution.steps.length + 1,
            type: 'tool_call',
            content: toolCall,
            timestamp: new Date(),
          });

          try {
            const toolResult = await this.executeTool(toolCall);

            execution.steps.push({
              stepNumber: execution.steps.length + 1,
              type: 'tool_result',
              content: toolResult,
              timestamp: new Date(),
            });

            // Add tool result to messages
            messages.push({
              role: 'tool',
              content: JSON.stringify(toolResult),
            });
          } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Tool execution failed';

            execution.steps.push({
              stepNumber: execution.steps.length + 1,
              type: 'tool_result',
              content: { error: errorMessage },
              timestamp: new Date(),
            });

            if (agent.behavior.stopOnToolError) {
              throw new Error(`Tool execution failed: ${errorMessage}`);
            }

            messages.push({
              role: 'tool',
              content: JSON.stringify({ error: errorMessage }),
            });
          }
        }
      } else if (response.content) {
        // Final response
        execution.steps.push({
          stepNumber: execution.steps.length + 1,
          type: 'response',
          content: response.content,
          timestamp: new Date(),
        });

        execution.output = {
          response: response.content,
          toolCalls: response.toolCalls,
        };

        completed = true;
      } else {
        // No response or tool calls - something went wrong
        throw new Error('Model returned empty response');
      }
    }

    if (!completed) {
      execution.status = AgentStatus.FAILED;
      execution.error = {
        code: 'MAX_ITERATIONS',
        message: `Reached maximum iterations: ${agent.behavior.maxIterations}`,
      };
    } else {
      execution.status = AgentStatus.COMPLETED;
    }

    execution.completedAt = new Date();

    // Calculate metrics
    const inputTokens = messages.reduce((sum, m) => sum + this.estimateTokens(m.content), 0);
    const outputTokens = execution.output?.response
      ? this.estimateTokens(execution.output.response)
      : 0;

    execution.metrics = {
      inputTokens,
      outputTokens,
      totalTokens: inputTokens + outputTokens,
      toolCalls: execution.steps.filter(s => s.type === 'tool_call').length,
      iterations: iteration,
      durationMs: execution.completedAt.getTime() - execution.startedAt.getTime(),
    };

    // Save execution
    await this.db.query(
      `UPDATE agent_executions SET
        status = $1, steps = $2, output = $3, metrics = $4, error = $5, completed_at = $6
      WHERE id = $7`,
      [
        execution.status,
        JSON.stringify(execution.steps),
        execution.output ? JSON.stringify(execution.output) : null,
        JSON.stringify(execution.metrics),
        execution.error ? JSON.stringify(execution.error) : null,
        execution.completedAt,
        execution.id,
      ]
    );

    // Save to memory
    if (input.sessionId) {
      await this.saveToMemory(input.sessionId, [
        { role: 'user', content: input.message },
        { role: 'assistant', content: execution.output?.response || '' },
      ]);
    }

    return execution;
  }

  /**
   * Call the LLM model
   */
  private async callModel(
    modelId: string,
    messages: Array<{ role: string; content: string }>,
    tools: any[],
    config: AgentConfig['modelConfig']
  ): Promise<{
    content?: string;
    toolCalls?: ToolCall[];
  }> {
    // In production, this would call the actual model service
    // For now, return a mock response

    // This is where you would integrate with:
    // - OpenAI API
    // - Anthropic API
    // - Local models
    // - Model router for A/B testing

    return {
      content: 'This is a placeholder response. In production, this would call the actual LLM.',
    };
  }

  /**
   * Execute a tool
   */
  private async executeTool(toolCall: ToolCall): Promise<unknown> {
    // In production, this would execute the actual tool
    // See ToolService for tool execution logic

    return {
      success: true,
      result: 'Tool execution placeholder',
    };
  }

  /**
   * Load tools by IDs
   */
  private async loadTools(
    toolIds: string[],
    capabilities: AgentConfig['capabilities']
  ): Promise<any[]> {
    if (!capabilities.canUseTools || toolIds.length === 0) {
      return [];
    }

    const result = await this.db.query(
      `SELECT * FROM tools WHERE id = ANY($1) AND enabled = true`,
      [toolIds]
    );

    return result.rows.map(row => ({
      type: 'function',
      function: {
        name: row.name,
        description: row.description,
        parameters: row.parameters,
      },
    }));
  }

  /**
   * Load conversation memory
   */
  private async loadMemory(
    sessionId: string,
    config: AgentConfig['memory']
  ): Promise<Array<{ role: string; content: string }>> {
    if (config.type === 'none') {
      return [];
    }

    const key = `${this.cachePrefix}memory:${sessionId}`;
    const messages = await this.redis.lRange(key, -config.maxMessages * 2, -1);

    return messages.map(m => JSON.parse(m));
  }

  /**
   * Save to conversation memory
   */
  private async saveToMemory(
    sessionId: string,
    messages: Array<{ role: string; content: string }>
  ): Promise<void> {
    const key = `${this.cachePrefix}memory:${sessionId}`;

    for (const msg of messages) {
      await this.redis.rPush(key, JSON.stringify(msg));
    }

    // Trim to max size
    await this.redis.lTrim(key, -100, -1);

    // Set expiry
    await this.redis.expire(key, 86400); // 24 hours
  }

  /**
   * Get execution by ID
   */
  async getExecution(executionId: string): Promise<AgentExecution | null> {
    const result = await this.db.query(
      `SELECT * FROM agent_executions WHERE id = $1`,
      [executionId]
    );

    if (result.rows.length === 0) return null;

    return this.mapExecutionRow(result.rows[0]);
  }

  /**
   * List executions for an agent
   */
  async listExecutions(
    agentId: string,
    options?: {
      sessionId?: string;
      status?: AgentStatus;
      limit?: number;
    }
  ): Promise<AgentExecution[]> {
    let query = `SELECT * FROM agent_executions WHERE agent_id = $1`;
    const values: unknown[] = [agentId];
    let paramIndex = 2;

    if (options?.sessionId) {
      query += ` AND session_id = $${paramIndex++}`;
      values.push(options.sessionId);
    }
    if (options?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(options.status);
    }

    query += ` ORDER BY started_at DESC LIMIT $${paramIndex}`;
    values.push(options?.limit || 100);

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapExecutionRow);
  }

  /**
   * Cancel an execution
   */
  async cancelExecution(executionId: string): Promise<void> {
    await this.db.query(
      `UPDATE agent_executions SET status = $1, completed_at = NOW() WHERE id = $2 AND status = $3`,
      [AgentStatus.CANCELLED, executionId, AgentStatus.RUNNING]
    );
  }

  // ===========================================
  // Helpers
  // ===========================================

  private estimateTokens(text: string): number {
    // Rough estimate: ~4 characters per token
    return Math.ceil(text.length / 4);
  }

  private mapAgentRow(row: any): AgentConfig {
    return {
      id: row.id,
      name: row.name,
      displayName: row.display_name,
      description: row.description,
      type: row.type,
      modelId: row.model_id,
      modelConfig: row.model_config || {},
      tools: row.tools || [],
      capabilities: row.capabilities,
      memory: row.memory,
      behavior: row.behavior,
      version: row.version,
      tags: row.tags || [],
      enabled: row.enabled,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
    };
  }

  private mapExecutionRow(row: any): AgentExecution {
    return {
      id: row.id,
      agentId: row.agent_id,
      sessionId: row.session_id,
      status: row.status,
      input: row.input,
      steps: row.steps || [],
      output: row.output,
      metrics: row.metrics,
      error: row.error,
      startedAt: row.started_at,
      completedAt: row.completed_at,
      createdBy: row.created_by,
    };
  }
}
