/**
 * Workflows API client
 */

import { HttpClient, CopilotError } from '../client';
import type {
  WorkflowDefinition,
  WorkflowExecution,
  CreateWorkflowInput,
  ExecuteWorkflowInput,
  WorkflowStatus,
  StepResult,
  PaginatedResponse,
  PaginationParams,
  StreamOptions,
  StreamEvent,
} from '../types';

/**
 * Workflow execution event
 */
interface WorkflowEvent {
  type: 'step_started' | 'step_completed' | 'step_failed' | 'workflow_completed';
  stepId?: string;
  result?: StepResult;
  error?: string;
}

/**
 * Workflows API client
 */
export class WorkflowsClient {
  constructor(private readonly client: HttpClient) {}

  // ============================================================================
  // Workflow Definitions
  // ============================================================================

  /**
   * Create a new workflow
   */
  async create(input: CreateWorkflowInput): Promise<WorkflowDefinition> {
    const response = await this.client.post<WorkflowDefinition>(
      '/api/v1/workflows',
      input
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to create workflow', 'CREATE_FAILED');
    }

    return response.data;
  }

  /**
   * Get a workflow by ID
   */
  async get(workflowId: string): Promise<WorkflowDefinition> {
    const response = await this.client.get<WorkflowDefinition>(
      `/api/v1/workflows/${workflowId}`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Workflow not found', 'NOT_FOUND', 404);
    }

    return response.data;
  }

  /**
   * List workflows
   */
  async list(
    params?: PaginationParams
  ): Promise<PaginatedResponse<WorkflowDefinition>> {
    const response = await this.client.paginate<WorkflowDefinition>(
      '/api/v1/workflows',
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list workflows', 'LIST_FAILED');
    }

    return response.data;
  }

  /**
   * Update a workflow
   */
  async update(
    workflowId: string,
    updates: Partial<CreateWorkflowInput>
  ): Promise<WorkflowDefinition> {
    const response = await this.client.patch<WorkflowDefinition>(
      `/api/v1/workflows/${workflowId}`,
      updates
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to update workflow', 'UPDATE_FAILED');
    }

    return response.data;
  }

  /**
   * Delete a workflow
   */
  async delete(workflowId: string): Promise<void> {
    const response = await this.client.delete(
      `/api/v1/workflows/${workflowId}`
    );

    if (!response.success) {
      throw new CopilotError('Failed to delete workflow', 'DELETE_FAILED');
    }
  }

  /**
   * Create a new version of a workflow
   */
  async createVersion(
    workflowId: string,
    versionType: 'major' | 'minor' | 'patch' = 'minor'
  ): Promise<WorkflowDefinition> {
    const response = await this.client.post<WorkflowDefinition>(
      `/api/v1/workflows/${workflowId}/versions`,
      { versionType }
    );

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to create workflow version',
        'VERSION_FAILED'
      );
    }

    return response.data;
  }

  /**
   * List workflow versions
   */
  async listVersions(
    workflowId: string
  ): Promise<PaginatedResponse<WorkflowDefinition>> {
    const response = await this.client.paginate<WorkflowDefinition>(
      `/api/v1/workflows/${workflowId}/versions`
    );

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to list workflow versions',
        'LIST_VERSIONS_FAILED'
      );
    }

    return response.data;
  }

  // ============================================================================
  // Workflow Execution
  // ============================================================================

  /**
   * Execute a workflow
   */
  async execute(input: ExecuteWorkflowInput): Promise<WorkflowExecution> {
    const { workflowId, ...rest } = input;

    const response = await this.client.post<WorkflowExecution>(
      `/api/v1/workflows/${workflowId}/execute`,
      rest
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to execute workflow', 'EXECUTE_FAILED');
    }

    return this.parseExecution(response.data);
  }

  /**
   * Execute a workflow and stream progress
   */
  async executeWithStream(
    input: ExecuteWorkflowInput,
    options: StreamOptions & {
      onStep?: (stepId: string, result: StepResult) => void;
    } = {}
  ): Promise<WorkflowExecution> {
    const { workflowId, ...rest } = input;
    const { onChunk, onEvent, onError, onComplete, onStep, signal } = options;

    let execution: Partial<WorkflowExecution> = {};

    return new Promise<WorkflowExecution>((resolve, reject) => {
      this.client
        .stream(
          `/api/v1/workflows/${workflowId}/execute/stream`,
          { ...rest, stream: true },
          {
            onChunk: (data) => {
              try {
                const event = JSON.parse(data) as StreamEvent<WorkflowEvent>;

                if (event.type === 'message_start') {
                  execution = event.data as Partial<WorkflowExecution>;
                }

                const workflowEvent = event.data as WorkflowEvent;

                switch (workflowEvent.type) {
                  case 'step_started':
                    onChunk?.(`Step started: ${workflowEvent.stepId}`);
                    break;

                  case 'step_completed':
                    if (workflowEvent.stepId && workflowEvent.result) {
                      onStep?.(workflowEvent.stepId, workflowEvent.result);
                    }
                    onChunk?.(`Step completed: ${workflowEvent.stepId}`);
                    break;

                  case 'step_failed':
                    onChunk?.(
                      `Step failed: ${workflowEvent.stepId} - ${workflowEvent.error}`
                    );
                    break;

                  case 'workflow_completed':
                    break;
                }

                onEvent?.(event);
              } catch {
                onChunk?.(data);
              }
            },
            onError: (error) => {
              onError?.(error);
              reject(error);
            },
            onComplete: () => {
              const finalExecution = this.parseExecution(
                execution as WorkflowExecution
              );
              onComplete?.(finalExecution as never);
              resolve(finalExecution);
            },
          },
          signal
        )
        .catch(reject);
    });
  }

  /**
   * Get execution status
   */
  async getExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.get<WorkflowExecution>(
      `/api/v1/executions/${executionId}`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Execution not found', 'NOT_FOUND', 404);
    }

    return this.parseExecution(response.data);
  }

  /**
   * List executions for a workflow
   */
  async listExecutions(
    workflowId: string,
    params?: PaginationParams & { status?: WorkflowStatus }
  ): Promise<PaginatedResponse<WorkflowExecution>> {
    const response = await this.client.paginate<WorkflowExecution>(
      `/api/v1/workflows/${workflowId}/executions`,
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list executions', 'LIST_FAILED');
    }

    return {
      ...response.data,
      items: response.data.items.map((e) => this.parseExecution(e)),
    };
  }

  /**
   * Cancel a running execution
   */
  async cancelExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.post<WorkflowExecution>(
      `/api/v1/executions/${executionId}/cancel`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to cancel execution', 'CANCEL_FAILED');
    }

    return this.parseExecution(response.data);
  }

  /**
   * Retry a failed execution
   */
  async retryExecution(
    executionId: string,
    fromStep?: string
  ): Promise<WorkflowExecution> {
    const response = await this.client.post<WorkflowExecution>(
      `/api/v1/executions/${executionId}/retry`,
      { fromStep }
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to retry execution', 'RETRY_FAILED');
    }

    return this.parseExecution(response.data);
  }

  /**
   * Pause a running execution
   */
  async pauseExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.post<WorkflowExecution>(
      `/api/v1/executions/${executionId}/pause`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to pause execution', 'PAUSE_FAILED');
    }

    return this.parseExecution(response.data);
  }

  /**
   * Resume a paused execution
   */
  async resumeExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.post<WorkflowExecution>(
      `/api/v1/executions/${executionId}/resume`
    );

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to resume execution', 'RESUME_FAILED');
    }

    return this.parseExecution(response.data);
  }

  /**
   * Poll execution until completion
   */
  async waitForCompletion(
    executionId: string,
    options: {
      pollInterval?: number;
      timeout?: number;
      onProgress?: (execution: WorkflowExecution) => void;
    } = {}
  ): Promise<WorkflowExecution> {
    const { pollInterval = 1000, timeout = 300000, onProgress } = options;
    const startTime = Date.now();

    while (true) {
      const execution = await this.getExecution(executionId);

      onProgress?.(execution);

      if (
        execution.status === 'completed' ||
        execution.status === 'failed' ||
        execution.status === 'cancelled'
      ) {
        return execution;
      }

      if (Date.now() - startTime > timeout) {
        throw new CopilotError(
          'Workflow execution timeout',
          'EXECUTION_TIMEOUT'
        );
      }

      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }
  }

  // ============================================================================
  // Templates
  // ============================================================================

  /**
   * Create workflow from template
   */
  async createFromTemplate(
    templateId: string,
    params: Record<string, unknown>
  ): Promise<WorkflowDefinition> {
    const response = await this.client.post<WorkflowDefinition>(
      `/api/v1/workflows/templates/${templateId}/instantiate`,
      params
    );

    if (!response.success || !response.data) {
      throw new CopilotError(
        'Failed to create workflow from template',
        'TEMPLATE_FAILED'
      );
    }

    return response.data;
  }

  /**
   * List available templates
   */
  async listTemplates(): Promise<
    PaginatedResponse<{ id: string; name: string; description: string }>
  > {
    const response = await this.client.paginate<{
      id: string;
      name: string;
      description: string;
    }>('/api/v1/workflows/templates');

    if (!response.success || !response.data) {
      throw new CopilotError('Failed to list templates', 'LIST_TEMPLATES_FAILED');
    }

    return response.data;
  }

  /**
   * Parse execution response to ensure proper types
   */
  private parseExecution(data: WorkflowExecution): WorkflowExecution {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
      startedAt: data.startedAt ? new Date(data.startedAt) : undefined,
      completedAt: data.completedAt ? new Date(data.completedAt) : undefined,
    };
  }
}

export default WorkflowsClient;
