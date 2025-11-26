// src/client.ts
var CopilotError = class _CopilotError extends Error {
  constructor(message, code, status, details, requestId) {
    super(message);
    this.code = code;
    this.status = status;
    this.details = details;
    this.requestId = requestId;
    this.name = "CopilotError";
  }
  static fromApiError(error, status) {
    return new _CopilotError(
      error.message,
      error.code,
      status,
      error.details,
      error.requestId
    );
  }
};

// src/workflows/index.ts
var WorkflowsClient = class {
  constructor(client) {
    this.client = client;
  }
  // ============================================================================
  // Workflow Definitions
  // ============================================================================
  /**
   * Create a new workflow
   */
  async create(input) {
    const response = await this.client.post(
      "/api/v1/workflows",
      input
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to create workflow", "CREATE_FAILED");
    }
    return response.data;
  }
  /**
   * Get a workflow by ID
   */
  async get(workflowId) {
    const response = await this.client.get(
      `/api/v1/workflows/${workflowId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Workflow not found", "NOT_FOUND", 404);
    }
    return response.data;
  }
  /**
   * List workflows
   */
  async list(params) {
    const response = await this.client.paginate(
      "/api/v1/workflows",
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list workflows", "LIST_FAILED");
    }
    return response.data;
  }
  /**
   * Update a workflow
   */
  async update(workflowId, updates) {
    const response = await this.client.patch(
      `/api/v1/workflows/${workflowId}`,
      updates
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to update workflow", "UPDATE_FAILED");
    }
    return response.data;
  }
  /**
   * Delete a workflow
   */
  async delete(workflowId) {
    const response = await this.client.delete(
      `/api/v1/workflows/${workflowId}`
    );
    if (!response.success) {
      throw new CopilotError("Failed to delete workflow", "DELETE_FAILED");
    }
  }
  /**
   * Create a new version of a workflow
   */
  async createVersion(workflowId, versionType = "minor") {
    const response = await this.client.post(
      `/api/v1/workflows/${workflowId}/versions`,
      { versionType }
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to create workflow version",
        "VERSION_FAILED"
      );
    }
    return response.data;
  }
  /**
   * List workflow versions
   */
  async listVersions(workflowId) {
    const response = await this.client.paginate(
      `/api/v1/workflows/${workflowId}/versions`
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to list workflow versions",
        "LIST_VERSIONS_FAILED"
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
  async execute(input) {
    const { workflowId, ...rest } = input;
    const response = await this.client.post(
      `/api/v1/workflows/${workflowId}/execute`,
      rest
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to execute workflow", "EXECUTE_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Execute a workflow and stream progress
   */
  async executeWithStream(input, options = {}) {
    const { workflowId, ...rest } = input;
    const { onChunk, onEvent, onError, onComplete, onStep, signal } = options;
    let execution = {};
    return new Promise((resolve, reject) => {
      this.client.stream(
        `/api/v1/workflows/${workflowId}/execute/stream`,
        { ...rest, stream: true },
        {
          onChunk: (data) => {
            try {
              const event = JSON.parse(data);
              if (event.type === "message_start") {
                execution = event.data;
              }
              const workflowEvent = event.data;
              switch (workflowEvent.type) {
                case "step_started":
                  onChunk?.(`Step started: ${workflowEvent.stepId}`);
                  break;
                case "step_completed":
                  if (workflowEvent.stepId && workflowEvent.result) {
                    onStep?.(workflowEvent.stepId, workflowEvent.result);
                  }
                  onChunk?.(`Step completed: ${workflowEvent.stepId}`);
                  break;
                case "step_failed":
                  onChunk?.(
                    `Step failed: ${workflowEvent.stepId} - ${workflowEvent.error}`
                  );
                  break;
                case "workflow_completed":
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
              execution
            );
            onComplete?.(finalExecution);
            resolve(finalExecution);
          }
        },
        signal
      ).catch(reject);
    });
  }
  /**
   * Get execution status
   */
  async getExecution(executionId) {
    const response = await this.client.get(
      `/api/v1/executions/${executionId}`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Execution not found", "NOT_FOUND", 404);
    }
    return this.parseExecution(response.data);
  }
  /**
   * List executions for a workflow
   */
  async listExecutions(workflowId, params) {
    const response = await this.client.paginate(
      `/api/v1/workflows/${workflowId}/executions`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list executions", "LIST_FAILED");
    }
    return {
      ...response.data,
      items: response.data.items.map((e) => this.parseExecution(e))
    };
  }
  /**
   * Cancel a running execution
   */
  async cancelExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/cancel`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to cancel execution", "CANCEL_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Retry a failed execution
   */
  async retryExecution(executionId, fromStep) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/retry`,
      { fromStep }
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to retry execution", "RETRY_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Pause a running execution
   */
  async pauseExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/pause`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to pause execution", "PAUSE_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Resume a paused execution
   */
  async resumeExecution(executionId) {
    const response = await this.client.post(
      `/api/v1/executions/${executionId}/resume`
    );
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to resume execution", "RESUME_FAILED");
    }
    return this.parseExecution(response.data);
  }
  /**
   * Poll execution until completion
   */
  async waitForCompletion(executionId, options = {}) {
    const { pollInterval = 1e3, timeout = 3e5, onProgress } = options;
    const startTime = Date.now();
    while (true) {
      const execution = await this.getExecution(executionId);
      onProgress?.(execution);
      if (execution.status === "completed" || execution.status === "failed" || execution.status === "cancelled") {
        return execution;
      }
      if (Date.now() - startTime > timeout) {
        throw new CopilotError(
          "Workflow execution timeout",
          "EXECUTION_TIMEOUT"
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
  async createFromTemplate(templateId, params) {
    const response = await this.client.post(
      `/api/v1/workflows/templates/${templateId}/instantiate`,
      params
    );
    if (!response.success || !response.data) {
      throw new CopilotError(
        "Failed to create workflow from template",
        "TEMPLATE_FAILED"
      );
    }
    return response.data;
  }
  /**
   * List available templates
   */
  async listTemplates() {
    const response = await this.client.paginate("/api/v1/workflows/templates");
    if (!response.success || !response.data) {
      throw new CopilotError("Failed to list templates", "LIST_TEMPLATES_FAILED");
    }
    return response.data;
  }
  /**
   * Parse execution response to ensure proper types
   */
  parseExecution(data) {
    return {
      ...data,
      createdAt: new Date(data.createdAt),
      updatedAt: new Date(data.updatedAt),
      startedAt: data.startedAt ? new Date(data.startedAt) : void 0,
      completedAt: data.completedAt ? new Date(data.completedAt) : void 0
    };
  }
};
var workflows_default = WorkflowsClient;

export { WorkflowsClient, workflows_default as default };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map