import { H as HttpClient, u as CreateWorkflowInput, q as WorkflowDefinition, P as PaginationParams, d as PaginatedResponse, E as ExecuteWorkflowInput, s as WorkflowExecution, y as StreamOptions, t as StepResult, W as WorkflowStatus } from '../client-BNP-OnWr.mjs';

/**
 * Workflows API client
 */

/**
 * Workflows API client
 */
declare class WorkflowsClient {
    private readonly client;
    constructor(client: HttpClient);
    /**
     * Create a new workflow
     */
    create(input: CreateWorkflowInput): Promise<WorkflowDefinition>;
    /**
     * Get a workflow by ID
     */
    get(workflowId: string): Promise<WorkflowDefinition>;
    /**
     * List workflows
     */
    list(params?: PaginationParams): Promise<PaginatedResponse<WorkflowDefinition>>;
    /**
     * Update a workflow
     */
    update(workflowId: string, updates: Partial<CreateWorkflowInput>): Promise<WorkflowDefinition>;
    /**
     * Delete a workflow
     */
    delete(workflowId: string): Promise<void>;
    /**
     * Create a new version of a workflow
     */
    createVersion(workflowId: string, versionType?: 'major' | 'minor' | 'patch'): Promise<WorkflowDefinition>;
    /**
     * List workflow versions
     */
    listVersions(workflowId: string): Promise<PaginatedResponse<WorkflowDefinition>>;
    /**
     * Execute a workflow
     */
    execute(input: ExecuteWorkflowInput): Promise<WorkflowExecution>;
    /**
     * Execute a workflow and stream progress
     */
    executeWithStream(input: ExecuteWorkflowInput, options?: StreamOptions & {
        onStep?: (stepId: string, result: StepResult) => void;
    }): Promise<WorkflowExecution>;
    /**
     * Get execution status
     */
    getExecution(executionId: string): Promise<WorkflowExecution>;
    /**
     * List executions for a workflow
     */
    listExecutions(workflowId: string, params?: PaginationParams & {
        status?: WorkflowStatus;
    }): Promise<PaginatedResponse<WorkflowExecution>>;
    /**
     * Cancel a running execution
     */
    cancelExecution(executionId: string): Promise<WorkflowExecution>;
    /**
     * Retry a failed execution
     */
    retryExecution(executionId: string, fromStep?: string): Promise<WorkflowExecution>;
    /**
     * Pause a running execution
     */
    pauseExecution(executionId: string): Promise<WorkflowExecution>;
    /**
     * Resume a paused execution
     */
    resumeExecution(executionId: string): Promise<WorkflowExecution>;
    /**
     * Poll execution until completion
     */
    waitForCompletion(executionId: string, options?: {
        pollInterval?: number;
        timeout?: number;
        onProgress?: (execution: WorkflowExecution) => void;
    }): Promise<WorkflowExecution>;
    /**
     * Create workflow from template
     */
    createFromTemplate(templateId: string, params: Record<string, unknown>): Promise<WorkflowDefinition>;
    /**
     * List available templates
     */
    listTemplates(): Promise<PaginatedResponse<{
        id: string;
        name: string;
        description: string;
    }>>;
    /**
     * Parse execution response to ensure proper types
     */
    private parseExecution;
}

export { WorkflowsClient, WorkflowsClient as default };
