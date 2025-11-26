/**
 * Tool Service
 *
 * Manages tools/functions that agents can call.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import axios from 'axios';
import {
  ToolDefinition,
  ToolCall,
  ToolType,
  CreateToolInput,
} from '../models/agent';

export class ToolService {
  private db: Pool;
  private redis: RedisClientType;
  private cachePrefix = 'tool:';
  private builtInTools: Map<string, (args: Record<string, unknown>) => Promise<unknown>>;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
    this.builtInTools = new Map();

    // Register built-in tools
    this.registerBuiltInTools();
  }

  /**
   * Register built-in tools
   */
  private registerBuiltInTools(): void {
    // Web search tool
    this.builtInTools.set('web_search', async (args) => {
      const { query, maxResults = 5 } = args as { query: string; maxResults?: number };
      // In production, would call actual search API
      return {
        results: [],
        query,
        message: 'Web search is a placeholder. Configure a search provider.',
      };
    });

    // Calculator tool
    this.builtInTools.set('calculator', async (args) => {
      const { expression } = args as { expression: string };
      try {
        // Safe expression evaluation using Function (limited math operations)
        const safeExpression = expression.replace(/[^0-9+\-*/().%\s]/g, '');
        const result = Function(`"use strict"; return (${safeExpression})`)();
        return { result, expression };
      } catch (error) {
        throw new Error(`Invalid expression: ${expression}`);
      }
    });

    // Current time tool
    this.builtInTools.set('current_time', async (args) => {
      const { timezone = 'UTC' } = args as { timezone?: string };
      const now = new Date();
      return {
        timestamp: now.toISOString(),
        timezone,
        formatted: now.toLocaleString('en-US', { timeZone: timezone }),
      };
    });

    // JSON parser tool
    this.builtInTools.set('parse_json', async (args) => {
      const { text } = args as { text: string };
      try {
        return { result: JSON.parse(text), valid: true };
      } catch (error) {
        return { result: null, valid: false, error: 'Invalid JSON' };
      }
    });
  }

  // ===========================================
  // Tool CRUD Operations
  // ===========================================

  /**
   * Create a new tool
   */
  async createTool(input: CreateToolInput, userId: string): Promise<ToolDefinition> {
    // Validate tool name format
    if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(input.name)) {
      throw new Error('Tool name must start with a letter or underscore and contain only alphanumeric characters and underscores');
    }

    const tool: ToolDefinition = {
      id: uuidv4(),
      name: input.name,
      displayName: input.displayName,
      description: input.description,
      type: input.type,
      parameters: input.parameters,
      returns: input.returns,
      execution: {
        timeout: 30000,
        retries: 3,
        sandboxed: true,
        ...input.execution,
      },
      permissions: {
        requiresConfirmation: false,
        ...input.permissions,
      },
      version: '1.0.0',
      tags: input.tags || [],
      enabled: true,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO tools (
        id, name, display_name, description, type, parameters, returns,
        execution, permissions, version, tags, enabled, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)`,
      [
        tool.id, tool.name, tool.displayName, tool.description, tool.type,
        JSON.stringify(tool.parameters), tool.returns ? JSON.stringify(tool.returns) : null,
        JSON.stringify(tool.execution), JSON.stringify(tool.permissions),
        tool.version, JSON.stringify(tool.tags), tool.enabled,
        tool.createdAt, tool.updatedAt,
      ]
    );

    return tool;
  }

  /**
   * Get tool by ID
   */
  async getTool(toolId: string): Promise<ToolDefinition | null> {
    const cached = await this.redis.get(`${this.cachePrefix}${toolId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    const result = await this.db.query(
      `SELECT * FROM tools WHERE id = $1`,
      [toolId]
    );

    if (result.rows.length === 0) return null;

    const tool = this.mapToolRow(result.rows[0]);

    await this.redis.set(
      `${this.cachePrefix}${toolId}`,
      JSON.stringify(tool),
      { EX: 300 }
    );

    return tool;
  }

  /**
   * Get tool by name
   */
  async getToolByName(name: string): Promise<ToolDefinition | null> {
    const result = await this.db.query(
      `SELECT * FROM tools WHERE name = $1`,
      [name]
    );

    if (result.rows.length === 0) return null;

    return this.mapToolRow(result.rows[0]);
  }

  /**
   * List tools
   */
  async listTools(filters?: {
    type?: ToolType;
    enabled?: boolean;
    tags?: string[];
  }): Promise<ToolDefinition[]> {
    let query = `SELECT * FROM tools WHERE 1=1`;
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
    return result.rows.map(this.mapToolRow);
  }

  /**
   * Update tool
   */
  async updateTool(
    toolId: string,
    updates: Partial<CreateToolInput>
  ): Promise<ToolDefinition | null> {
    const tool = await this.getTool(toolId);
    if (!tool) return null;

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
    if (updates.parameters !== undefined) {
      fields.push(`parameters = $${paramIndex++}`);
      values.push(JSON.stringify(updates.parameters));
    }
    if (updates.returns !== undefined) {
      fields.push(`returns = $${paramIndex++}`);
      values.push(JSON.stringify(updates.returns));
    }
    if (updates.execution !== undefined) {
      fields.push(`execution = $${paramIndex++}`);
      values.push(JSON.stringify({ ...tool.execution, ...updates.execution }));
    }
    if (updates.permissions !== undefined) {
      fields.push(`permissions = $${paramIndex++}`);
      values.push(JSON.stringify({ ...tool.permissions, ...updates.permissions }));
    }
    if (updates.tags !== undefined) {
      fields.push(`tags = $${paramIndex++}`);
      values.push(JSON.stringify(updates.tags));
    }

    if (fields.length === 0) return tool;

    fields.push('updated_at = NOW()');
    values.push(toolId);

    await this.db.query(
      `UPDATE tools SET ${fields.join(', ')} WHERE id = $${paramIndex}`,
      values
    );

    await this.redis.del(`${this.cachePrefix}${toolId}`);

    return this.getTool(toolId);
  }

  /**
   * Enable/disable tool
   */
  async setToolEnabled(toolId: string, enabled: boolean): Promise<void> {
    await this.db.query(
      `UPDATE tools SET enabled = $1, updated_at = NOW() WHERE id = $2`,
      [enabled, toolId]
    );
    await this.redis.del(`${this.cachePrefix}${toolId}`);
  }

  /**
   * Delete tool
   */
  async deleteTool(toolId: string): Promise<boolean> {
    const result = await this.db.query(
      `DELETE FROM tools WHERE id = $1 RETURNING id`,
      [toolId]
    );

    if (result.rows.length > 0) {
      await this.redis.del(`${this.cachePrefix}${toolId}`);
      return true;
    }

    return false;
  }

  // ===========================================
  // Tool Execution
  // ===========================================

  /**
   * Execute a tool
   */
  async executeTool(
    toolId: string,
    args: Record<string, unknown>,
    context?: {
      tenantId?: string;
      userId?: string;
      executionId?: string;
    }
  ): Promise<ToolCall> {
    const tool = await this.getTool(toolId);
    if (!tool) {
      throw new Error(`Tool not found: ${toolId}`);
    }
    if (!tool.enabled) {
      throw new Error(`Tool is disabled: ${tool.name}`);
    }

    // Check permissions
    if (tool.permissions.allowedTenants?.length && context?.tenantId) {
      if (!tool.permissions.allowedTenants.includes(context.tenantId)) {
        throw new Error(`Tenant not allowed to use tool: ${tool.name}`);
      }
    }
    if (tool.permissions.deniedTenants?.length && context?.tenantId) {
      if (tool.permissions.deniedTenants.includes(context.tenantId)) {
        throw new Error(`Tenant denied from using tool: ${tool.name}`);
      }
    }

    // Check rate limit
    if (tool.permissions.rateLimit && context?.tenantId) {
      const rateLimitKey = `${this.cachePrefix}ratelimit:${toolId}:${context.tenantId}`;
      const count = await this.redis.incr(rateLimitKey);
      if (count === 1) {
        await this.redis.expire(rateLimitKey, 60);
      }
      if (count > tool.permissions.rateLimit) {
        throw new Error(`Rate limit exceeded for tool: ${tool.name}`);
      }
    }

    // Validate arguments
    this.validateArguments(tool.parameters, args);

    const toolCall: ToolCall = {
      id: uuidv4(),
      toolId,
      toolName: tool.name,
      arguments: args,
      status: 'running',
      startedAt: new Date(),
    };

    try {
      // Execute based on tool type
      let result: unknown;
      const startTime = Date.now();

      const timeout = tool.execution.timeout || 30000;
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Tool execution timed out')), timeout);
      });

      const executionPromise = this.executeToolInternal(tool, args);
      result = await Promise.race([executionPromise, timeoutPromise]);

      toolCall.result = result;
      toolCall.status = 'completed';
      toolCall.completedAt = new Date();
      toolCall.durationMs = Date.now() - startTime;

      // Log execution
      await this.logToolExecution(toolCall, context);

      return toolCall;
    } catch (error) {
      toolCall.status = 'failed';
      toolCall.error = error instanceof Error ? error.message : 'Unknown error';
      toolCall.completedAt = new Date();
      toolCall.durationMs = toolCall.startedAt
        ? Date.now() - toolCall.startedAt.getTime()
        : 0;

      // Log failed execution
      await this.logToolExecution(toolCall, context);

      // Retry logic
      if (tool.execution.retries && tool.execution.retries > 0) {
        // In production, would implement proper retry with backoff
        console.warn(`Tool execution failed, retries remaining: ${tool.execution.retries}`);
      }

      throw error;
    }
  }

  /**
   * Execute tool internally based on type
   */
  private async executeToolInternal(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    switch (tool.type) {
      case ToolType.FUNCTION:
        return this.executeFunctionTool(tool, args);

      case ToolType.API:
        return this.executeApiTool(tool, args);

      case ToolType.DATABASE:
        return this.executeDatabaseTool(tool, args);

      case ToolType.SEARCH:
        return this.executeSearchTool(tool, args);

      case ToolType.CODE_EXECUTION:
        return this.executeCodeTool(tool, args);

      case ToolType.CUSTOM:
        return this.executeCustomTool(tool, args);

      default:
        throw new Error(`Unsupported tool type: ${tool.type}`);
    }
  }

  /**
   * Execute a function tool
   */
  private async executeFunctionTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    // Check if it's a built-in function
    const builtIn = this.builtInTools.get(tool.name);
    if (builtIn) {
      return builtIn(args);
    }

    // Otherwise, look for registered handler
    if (tool.execution.handler) {
      // In production, would dynamically load and execute handler
      throw new Error(`Custom function handlers not implemented: ${tool.execution.handler}`);
    }

    throw new Error(`No handler found for function tool: ${tool.name}`);
  }

  /**
   * Execute an API tool
   */
  private async executeApiTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    if (!tool.execution.endpoint) {
      throw new Error('API tool requires endpoint configuration');
    }

    const { method = 'POST', headers = {}, body } = args as {
      method?: string;
      headers?: Record<string, string>;
      body?: unknown;
    };

    const response = await axios({
      method: method as any,
      url: tool.execution.endpoint,
      headers,
      data: body || args,
      timeout: tool.execution.timeout,
    });

    return {
      status: response.status,
      data: response.data,
      headers: response.headers,
    };
  }

  /**
   * Execute a database tool
   */
  private async executeDatabaseTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    // In production, would execute against configured database
    // with proper security measures (parameterized queries, sandboxing)
    throw new Error('Database tools not implemented');
  }

  /**
   * Execute a search tool
   */
  private async executeSearchTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    const { query, maxResults = 10 } = args as { query: string; maxResults?: number };

    // Use built-in web search or configured search provider
    return this.builtInTools.get('web_search')!({ query, maxResults });
  }

  /**
   * Execute a code execution tool
   */
  private async executeCodeTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    // In production, would execute in sandboxed environment
    // (e.g., Docker container, E2B, AWS Lambda)
    throw new Error('Code execution tools require sandbox environment');
  }

  /**
   * Execute a custom tool
   */
  private async executeCustomTool(
    tool: ToolDefinition,
    args: Record<string, unknown>
  ): Promise<unknown> {
    if (!tool.execution.handler) {
      throw new Error('Custom tool requires handler configuration');
    }

    // In production, would load and execute custom handler
    throw new Error('Custom tool handlers not implemented');
  }

  /**
   * Validate tool arguments against schema
   */
  private validateArguments(
    schema: ToolDefinition['parameters'],
    args: Record<string, unknown>
  ): void {
    const required = schema.required || [];

    // Check required arguments
    for (const requiredArg of required) {
      if (!(requiredArg in args)) {
        throw new Error(`Missing required argument: ${requiredArg}`);
      }
    }

    // Validate argument types
    for (const [key, value] of Object.entries(args)) {
      const paramSchema = schema.properties[key];
      if (!paramSchema) {
        // Unknown argument - could be strict or lenient
        continue;
      }

      // Type checking
      const actualType = Array.isArray(value) ? 'array' : typeof value;
      if (paramSchema.type !== actualType && value !== null && value !== undefined) {
        // Allow null/undefined for optional params
        if (!required.includes(key)) {
          continue;
        }
        throw new Error(`Invalid type for argument ${key}: expected ${paramSchema.type}, got ${actualType}`);
      }

      // Enum validation
      if (paramSchema.enum && !paramSchema.enum.includes(value)) {
        throw new Error(`Invalid value for argument ${key}: must be one of ${paramSchema.enum.join(', ')}`);
      }
    }
  }

  /**
   * Log tool execution for audit/analytics
   */
  private async logToolExecution(
    toolCall: ToolCall,
    context?: {
      tenantId?: string;
      userId?: string;
      executionId?: string;
    }
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO tool_executions (
        id, tool_id, tool_name, arguments, status, result, error,
        started_at, completed_at, duration_ms, tenant_id, user_id, execution_id
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
      [
        toolCall.id, toolCall.toolId, toolCall.toolName,
        JSON.stringify(toolCall.arguments), toolCall.status,
        toolCall.result ? JSON.stringify(toolCall.result) : null,
        toolCall.error, toolCall.startedAt, toolCall.completedAt,
        toolCall.durationMs, context?.tenantId, context?.userId, context?.executionId,
      ]
    );
  }

  // ===========================================
  // Tool Discovery
  // ===========================================

  /**
   * Get tools formatted for LLM function calling
   */
  async getToolsForLLM(toolIds: string[]): Promise<any[]> {
    const tools = await Promise.all(toolIds.map(id => this.getTool(id)));

    return tools
      .filter((t): t is ToolDefinition => t !== null && t.enabled)
      .map(tool => ({
        type: 'function',
        function: {
          name: tool.name,
          description: tool.description,
          parameters: tool.parameters,
        },
      }));
  }

  /**
   * Search tools by capability
   */
  async searchTools(query: string): Promise<ToolDefinition[]> {
    const result = await this.db.query(
      `SELECT * FROM tools
       WHERE enabled = true AND (
         name ILIKE $1 OR
         display_name ILIKE $1 OR
         description ILIKE $1
       )
       ORDER BY name ASC
       LIMIT 20`,
      [`%${query}%`]
    );

    return result.rows.map(this.mapToolRow);
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapToolRow(row: any): ToolDefinition {
    return {
      id: row.id,
      name: row.name,
      displayName: row.display_name,
      description: row.description,
      type: row.type,
      parameters: row.parameters,
      returns: row.returns,
      execution: row.execution,
      permissions: row.permissions || {},
      version: row.version,
      tags: row.tags || [],
      enabled: row.enabled,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
