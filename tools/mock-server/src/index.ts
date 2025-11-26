/**
 * LLM CoPilot Mock Server
 *
 * A mock server implementation of the CoPilot API for SDK testing.
 * Provides deterministic responses for all API endpoints.
 */

import express, { Request, Response, NextFunction } from 'express';
import { v4 as uuidv4 } from 'uuid';

const app = express();
const PORT = process.env.PORT || 8081;
const MOCK_DELAY_MS = parseInt(process.env.MOCK_DELAY_MS || '0', 10);

// Middleware
app.use(express.json());

// Simulate network delay
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

const withDelay = async (req: Request, res: Response, next: NextFunction) => {
  if (MOCK_DELAY_MS > 0) {
    await delay(MOCK_DELAY_MS);
  }
  next();
};

app.use(withDelay);

// In-memory storage
const storage = {
  conversations: new Map<string, any>(),
  messages: new Map<string, any[]>(),
  workflows: new Map<string, any>(),
  workflowRuns: new Map<string, any>(),
  contextItems: new Map<string, any>(),
  users: new Map<string, any>(),
  tokens: new Map<string, any>(),
};

// Initialize default user
storage.users.set('user-default', {
  id: 'user-default',
  username: 'testuser',
  email: 'test@example.com',
  roles: ['user'],
  tenant_id: 'tenant-default',
  is_active: true,
  email_verified: true,
  created_at: new Date().toISOString(),
});

// Auth middleware
const authenticate = (req: Request, res: Response, next: NextFunction) => {
  const apiKey = req.headers['x-api-key'];
  const authHeader = req.headers.authorization;

  if (apiKey === 'invalid-key' || authHeader === 'Bearer invalid-token') {
    return res.status(401).json({
      code: 'UNAUTHORIZED',
      message: 'Invalid credentials',
      request_id: uuidv4(),
    });
  }

  // Mock authenticated user
  (req as any).user = storage.users.get('user-default');
  next();
};

// ===========================================
// Health Endpoint
// ===========================================

app.get('/health', (req, res) => {
  res.json({
    status: 'healthy',
    version: '1.0.0-mock',
    uptime_seconds: Math.floor(process.uptime()),
    components: {
      database: 'healthy',
      cache: 'healthy',
      llm: 'healthy',
    },
  });
});

// ===========================================
// Authentication Endpoints
// ===========================================

app.post('/api/v1/auth/login', (req, res) => {
  const { username_or_email, password } = req.body;

  if (password === 'wrong-password') {
    return res.status(401).json({
      code: 'UNAUTHORIZED',
      message: 'Invalid credentials',
      request_id: uuidv4(),
    });
  }

  const accessToken = `mock-access-${uuidv4()}`;
  const refreshToken = `mock-refresh-${uuidv4()}`;

  storage.tokens.set(accessToken, { user_id: 'user-default' });

  res.json({
    access_token: accessToken,
    refresh_token: refreshToken,
    token_type: 'Bearer',
    expires_in: 3600,
    refresh_expires_in: 86400,
    user: storage.users.get('user-default'),
  });
});

app.post('/api/v1/auth/refresh', authenticate, (req, res) => {
  const accessToken = `mock-access-${uuidv4()}`;
  const refreshToken = `mock-refresh-${uuidv4()}`;

  res.json({
    access_token: accessToken,
    refresh_token: refreshToken,
    token_type: 'Bearer',
    expires_in: 3600,
  });
});

app.post('/api/v1/auth/logout', authenticate, (req, res) => {
  res.status(204).send();
});

app.get('/api/v1/auth/me', authenticate, (req, res) => {
  res.json((req as any).user);
});

// ===========================================
// Conversation Endpoints
// ===========================================

app.post('/api/v1/conversations', authenticate, (req, res) => {
  const conversation = {
    id: `conv-${uuidv4()}`,
    title: req.body.title || null,
    user_id: (req as any).user.id,
    tenant_id: (req as any).user.tenant_id,
    metadata: req.body.metadata || {},
    system_prompt: req.body.system_prompt || null,
    message_count: 0,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  storage.conversations.set(conversation.id, conversation);
  storage.messages.set(conversation.id, []);

  res.status(201).json(conversation);
});

app.get('/api/v1/conversations', authenticate, (req, res) => {
  const limit = parseInt(req.query.limit as string) || 20;
  const offset = parseInt(req.query.offset as string) || 0;

  const allConversations = Array.from(storage.conversations.values());
  const items = allConversations.slice(offset, offset + limit);

  res.json({
    items,
    total: allConversations.length,
    limit,
    offset,
    has_more: offset + limit < allConversations.length,
  });
});

app.get('/api/v1/conversations/:id', authenticate, (req, res) => {
  const conversation = storage.conversations.get(req.params.id);

  if (!conversation) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Conversation not found',
      request_id: uuidv4(),
    });
  }

  res.json(conversation);
});

app.delete('/api/v1/conversations/:id', authenticate, (req, res) => {
  if (!storage.conversations.has(req.params.id)) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Conversation not found',
      request_id: uuidv4(),
    });
  }

  storage.conversations.delete(req.params.id);
  storage.messages.delete(req.params.id);

  res.status(204).send();
});

// ===========================================
// Message Endpoints
// ===========================================

app.post('/api/v1/conversations/:conversationId/messages', authenticate, (req, res) => {
  const conversation = storage.conversations.get(req.params.conversationId);

  if (!conversation) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Conversation not found',
      request_id: uuidv4(),
    });
  }

  // Create user message
  const userMessage = {
    id: `msg-${uuidv4()}`,
    conversation_id: req.params.conversationId,
    role: req.body.role || 'user',
    content: req.body.content,
    metadata: req.body.metadata || {},
    created_at: new Date().toISOString(),
  };

  const messages = storage.messages.get(req.params.conversationId) || [];
  messages.push(userMessage);

  // Generate mock assistant response
  const assistantMessage = {
    id: `msg-${uuidv4()}`,
    conversation_id: req.params.conversationId,
    role: 'assistant',
    content: `Mock response to: "${req.body.content.substring(0, 50)}..."`,
    tokens_used: Math.floor(Math.random() * 500) + 50,
    model: 'mock-model-v1',
    metadata: {},
    created_at: new Date().toISOString(),
  };

  messages.push(assistantMessage);
  storage.messages.set(req.params.conversationId, messages);

  // Update conversation
  conversation.message_count = messages.length;
  conversation.updated_at = new Date().toISOString();

  res.status(201).json(assistantMessage);
});

app.post('/api/v1/conversations/:conversationId/messages/stream', authenticate, (req, res) => {
  const conversation = storage.conversations.get(req.params.conversationId);

  if (!conversation) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Conversation not found',
      request_id: uuidv4(),
    });
  }

  res.setHeader('Content-Type', 'text/event-stream');
  res.setHeader('Cache-Control', 'no-cache');
  res.setHeader('Connection', 'keep-alive');

  const messageId = `msg-${uuidv4()}`;
  const responseText = `This is a mock streaming response to: "${req.body.content}"`;
  const words = responseText.split(' ');

  res.write(`data: {"type":"message_start","message_id":"${messageId}"}\n\n`);

  let index = 0;
  const interval = setInterval(() => {
    if (index < words.length) {
      const text = words[index] + (index < words.length - 1 ? ' ' : '');
      res.write(`data: {"type":"content_delta","delta":{"text":"${text}"}}\n\n`);
      index++;
    } else {
      res.write(`data: {"type":"message_end","message_id":"${messageId}"}\n\n`);
      res.write('data: [DONE]\n\n');
      clearInterval(interval);
      res.end();
    }
  }, 50);

  req.on('close', () => {
    clearInterval(interval);
  });
});

app.get('/api/v1/conversations/:conversationId/messages', authenticate, (req, res) => {
  const messages = storage.messages.get(req.params.conversationId);

  if (!messages) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Conversation not found',
      request_id: uuidv4(),
    });
  }

  const limit = parseInt(req.query.limit as string) || 50;
  const offset = parseInt(req.query.offset as string) || 0;
  const items = messages.slice(offset, offset + limit);

  res.json({
    items,
    total: messages.length,
    limit,
    offset,
    has_more: offset + limit < messages.length,
  });
});

// ===========================================
// Workflow Endpoints
// ===========================================

app.post('/api/v1/workflows', authenticate, (req, res) => {
  const workflow = {
    id: `wf-${uuidv4()}`,
    user_id: (req as any).user.id,
    tenant_id: (req as any).user.tenant_id,
    name: req.body.name,
    description: req.body.description || null,
    version: req.body.version || '1.0.0',
    steps: req.body.steps || [],
    entry_point: req.body.entry_point,
    metadata: req.body.metadata || {},
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  storage.workflows.set(workflow.id, workflow);
  res.status(201).json(workflow);
});

app.get('/api/v1/workflows', authenticate, (req, res) => {
  const items = Array.from(storage.workflows.values());
  res.json({
    items,
    total: items.length,
  });
});

app.get('/api/v1/workflows/:id', authenticate, (req, res) => {
  const workflow = storage.workflows.get(req.params.id);

  if (!workflow) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Workflow not found',
      request_id: uuidv4(),
    });
  }

  res.json(workflow);
});

app.delete('/api/v1/workflows/:id', authenticate, (req, res) => {
  if (!storage.workflows.has(req.params.id)) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Workflow not found',
      request_id: uuidv4(),
    });
  }

  storage.workflows.delete(req.params.id);
  res.status(204).send();
});

// ===========================================
// Workflow Run Endpoints
// ===========================================

app.post('/api/v1/workflows/runs', authenticate, (req, res) => {
  const workflow = storage.workflows.get(req.body.workflow_id);

  if (!workflow) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Workflow not found',
      request_id: uuidv4(),
    });
  }

  const run = {
    id: `run-${uuidv4()}`,
    workflow_id: req.body.workflow_id,
    user_id: (req as any).user.id,
    tenant_id: (req as any).user.tenant_id,
    status: 'running',
    current_step: workflow.entry_point,
    inputs: req.body.inputs || {},
    outputs: {},
    started_at: new Date().toISOString(),
    created_at: new Date().toISOString(),
  };

  storage.workflowRuns.set(run.id, run);

  // Simulate completion after a delay
  setTimeout(() => {
    const storedRun = storage.workflowRuns.get(run.id);
    if (storedRun && storedRun.status === 'running') {
      storedRun.status = 'completed';
      storedRun.current_step = null;
      storedRun.outputs = { result: 'Mock workflow completed successfully' };
      storedRun.completed_at = new Date().toISOString();
    }
  }, 1000);

  res.status(201).json(run);
});

app.get('/api/v1/workflows/runs', authenticate, (req, res) => {
  let items = Array.from(storage.workflowRuns.values());

  if (req.query.workflow_id) {
    items = items.filter(r => r.workflow_id === req.query.workflow_id);
  }

  res.json({
    items,
    total: items.length,
  });
});

app.get('/api/v1/workflows/runs/:id', authenticate, (req, res) => {
  const run = storage.workflowRuns.get(req.params.id);

  if (!run) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Workflow run not found',
      request_id: uuidv4(),
    });
  }

  res.json(run);
});

app.post('/api/v1/workflows/runs/:id/cancel', authenticate, (req, res) => {
  const run = storage.workflowRuns.get(req.params.id);

  if (!run) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Workflow run not found',
      request_id: uuidv4(),
    });
  }

  run.status = 'cancelled';
  run.completed_at = new Date().toISOString();

  res.json(run);
});

// ===========================================
// Context Endpoints
// ===========================================

app.post('/api/v1/context', authenticate, (req, res) => {
  const item = {
    id: `ctx-${uuidv4()}`,
    user_id: (req as any).user.id,
    tenant_id: (req as any).user.tenant_id,
    name: req.body.name,
    type: req.body.type,
    content: req.body.content || null,
    url: req.body.url || null,
    metadata: req.body.metadata || {},
    token_count: req.body.content ? Math.ceil(req.body.content.length / 4) : 0,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  storage.contextItems.set(item.id, item);
  res.status(201).json(item);
});

app.get('/api/v1/context', authenticate, (req, res) => {
  const items = Array.from(storage.contextItems.values());
  res.json({
    items,
    total: items.length,
  });
});

app.get('/api/v1/context/:id', authenticate, (req, res) => {
  const item = storage.contextItems.get(req.params.id);

  if (!item) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Context item not found',
      request_id: uuidv4(),
    });
  }

  res.json(item);
});

app.delete('/api/v1/context/:id', authenticate, (req, res) => {
  if (!storage.contextItems.has(req.params.id)) {
    return res.status(404).json({
      code: 'NOT_FOUND',
      message: 'Context item not found',
      request_id: uuidv4(),
    });
  }

  storage.contextItems.delete(req.params.id);
  res.status(204).send();
});

// ===========================================
// Error Simulation Endpoints
// ===========================================

app.get('/api/v1/_test/error/:statusCode', (req, res) => {
  const statusCode = parseInt(req.params.statusCode);
  const errorCodes: Record<number, string> = {
    400: 'BAD_REQUEST',
    401: 'UNAUTHORIZED',
    403: 'FORBIDDEN',
    404: 'NOT_FOUND',
    429: 'RATE_LIMITED',
    500: 'SERVER_ERROR',
    502: 'BAD_GATEWAY',
    503: 'SERVICE_UNAVAILABLE',
  };

  res.status(statusCode).json({
    code: errorCodes[statusCode] || 'ERROR',
    message: `Simulated ${statusCode} error`,
    request_id: uuidv4(),
  });
});

// ===========================================
// Metrics Endpoint
// ===========================================

app.get('/metrics', (req, res) => {
  const metrics = `
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/health"} ${Math.floor(Math.random() * 1000)}
http_requests_total{method="POST",path="/api/v1/conversations"} ${Math.floor(Math.random() * 500)}

# HELP http_request_duration_seconds HTTP request duration
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.01"} ${Math.floor(Math.random() * 100)}
http_request_duration_seconds_bucket{le="0.1"} ${Math.floor(Math.random() * 200)}
http_request_duration_seconds_bucket{le="1"} ${Math.floor(Math.random() * 300)}
http_request_duration_seconds_bucket{le="+Inf"} ${Math.floor(Math.random() * 400)}

# HELP conversations_total Total conversations created
# TYPE conversations_total counter
conversations_total ${storage.conversations.size}

# HELP messages_total Total messages sent
# TYPE messages_total counter
messages_total ${Array.from(storage.messages.values()).reduce((sum, msgs) => sum + msgs.length, 0)}
`.trim();

  res.setHeader('Content-Type', 'text/plain');
  res.send(metrics);
});

// ===========================================
// 404 Handler
// ===========================================

app.use((req, res) => {
  res.status(404).json({
    code: 'NOT_FOUND',
    message: `Endpoint not found: ${req.method} ${req.path}`,
    request_id: uuidv4(),
  });
});

// ===========================================
// Error Handler
// ===========================================

app.use((err: Error, req: Request, res: Response, next: NextFunction) => {
  console.error('Error:', err);
  res.status(500).json({
    code: 'SERVER_ERROR',
    message: 'Internal server error',
    request_id: uuidv4(),
  });
});

// ===========================================
// Start Server
// ===========================================

app.listen(PORT, () => {
  console.log(`🚀 Mock server running on http://localhost:${PORT}`);
  console.log(`   Health check: http://localhost:${PORT}/health`);
  console.log(`   Delay: ${MOCK_DELAY_MS}ms`);
});

export default app;
