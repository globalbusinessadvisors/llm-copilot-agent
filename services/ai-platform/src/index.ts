/**
 * AI Platform Service
 *
 * Enterprise-grade AI/ML platform providing model management, agent orchestration,
 * tool/function calling, and RAG capabilities.
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';
import { createModelRoutes } from './routes/models';
import { createAgentRoutes } from './routes/agents';
import { createRAGRoutes } from './routes/rag';
import { createHealthRoutes } from './routes/health';
import { ModelService } from './services/modelService';
import { ABTestService } from './services/abTestService';
import { FineTuneService } from './services/fineTuneService';
import { AgentService } from './services/agentService';
import { ToolService } from './services/toolService';
import { TeamService } from './services/teamService';
import { RAGService } from './services/ragService';

// Configuration
const config = {
  port: parseInt(process.env.PORT || '3008', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  database: {
    host: process.env.DB_HOST || 'localhost',
    port: parseInt(process.env.DB_PORT || '5432', 10),
    database: process.env.DB_NAME || 'ai_platform',
    user: process.env.DB_USER || 'postgres',
    password: process.env.DB_PASSWORD || 'postgres',
    max: parseInt(process.env.DB_POOL_SIZE || '20', 10),
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 2000,
  },
  redis: {
    url: process.env.REDIS_URL || 'redis://localhost:6379',
  },
  cors: {
    origin: process.env.CORS_ORIGIN?.split(',') || ['http://localhost:3000'],
    credentials: true,
  },
};

// Error handling middleware
interface ApiError extends Error {
  statusCode?: number;
  code?: string;
}

function errorHandler(
  err: ApiError,
  _req: Request,
  res: Response,
  _next: NextFunction
): void {
  console.error('Error:', err);

  const statusCode = err.statusCode || 500;
  const message = err.message || 'Internal Server Error';
  const code = err.code || 'INTERNAL_ERROR';

  res.status(statusCode).json({
    error: {
      message,
      code,
      ...(config.nodeEnv === 'development' && { stack: err.stack }),
    },
  });
}

// Request logging middleware
function requestLogger(req: Request, _res: Response, next: NextFunction): void {
  const start = Date.now();
  _res.on('finish', () => {
    const duration = Date.now() - start;
    console.log(`${req.method} ${req.path} ${_res.statusCode} ${duration}ms`);
  });
  next();
}

async function main(): Promise<void> {
  // Initialize database connection
  const db = new Pool(config.database);

  // Test database connection
  try {
    await db.query('SELECT 1');
    console.log('Database connected successfully');
  } catch (error) {
    console.error('Failed to connect to database:', error);
    process.exit(1);
  }

  // Initialize Redis connection
  const redis = createClient({
    url: config.redis.url,
  }) as RedisClientType;

  redis.on('error', (err) => console.error('Redis Client Error:', err));

  try {
    await redis.connect();
    console.log('Redis connected successfully');
  } catch (error) {
    console.error('Failed to connect to Redis:', error);
    process.exit(1);
  }

  // Initialize services
  const modelService = new ModelService(db, redis);
  const abTestService = new ABTestService(db, redis);
  const fineTuneService = new FineTuneService(db, redis);
  const toolService = new ToolService(db, redis);
  const agentService = new AgentService(db, redis, toolService);
  const teamService = new TeamService(db, redis, agentService);
  const ragService = new RAGService(db, redis);

  // Create Express app
  const app: Express = express();

  // Apply middleware
  app.use(helmet({
    contentSecurityPolicy: config.nodeEnv === 'production',
  }));
  app.use(cors(config.cors));
  app.use(compression());
  app.use(express.json({ limit: '10mb' }));
  app.use(express.urlencoded({ extended: true }));
  app.use(requestLogger);

  // Mount routes
  app.use('/health', createHealthRoutes(db, redis));
  app.use('/api/v1/models', createModelRoutes(modelService, abTestService, fineTuneService));
  app.use('/api/v1/agents', createAgentRoutes(agentService, teamService, toolService));
  app.use('/api/v1/rag', createRAGRoutes(ragService));

  // Root endpoint
  app.get('/', (_req: Request, res: Response) => {
    res.json({
      service: 'ai-platform',
      version: '1.0.0',
      description: 'Enterprise AI/ML Platform',
      endpoints: {
        health: '/health',
        models: '/api/v1/models',
        agents: '/api/v1/agents',
        rag: '/api/v1/rag',
      },
    });
  });

  // API documentation endpoint
  app.get('/api/v1', (_req: Request, res: Response) => {
    res.json({
      version: 'v1',
      resources: {
        models: {
          base: '/api/v1/models',
          endpoints: [
            { method: 'GET', path: '/', description: 'List models' },
            { method: 'POST', path: '/', description: 'Create model' },
            { method: 'GET', path: '/:modelId', description: 'Get model' },
            { method: 'PATCH', path: '/:modelId/status', description: 'Update model status' },
            { method: 'GET', path: '/:modelId/metrics', description: 'Get model metrics' },
            { method: 'POST', path: '/:modelId/metrics', description: 'Record metrics' },
            { method: 'GET', path: '/:modelId/versions', description: 'List versions' },
            { method: 'POST', path: '/:modelId/versions', description: 'Create version' },
            { method: 'GET', path: '/:modelId/deployments', description: 'List deployments' },
            { method: 'POST', path: '/:modelId/deployments', description: 'Create deployment' },
            { method: 'GET', path: '/:modelId/ab-tests', description: 'List A/B tests' },
            { method: 'POST', path: '/:modelId/ab-tests', description: 'Create A/B test' },
            { method: 'GET', path: '/fine-tune/jobs', description: 'List fine-tune jobs' },
            { method: 'POST', path: '/fine-tune/jobs', description: 'Create fine-tune job' },
          ],
        },
        agents: {
          base: '/api/v1/agents',
          endpoints: [
            { method: 'GET', path: '/', description: 'List agents' },
            { method: 'POST', path: '/', description: 'Create agent' },
            { method: 'GET', path: '/:agentId', description: 'Get agent' },
            { method: 'POST', path: '/:agentId/execute', description: 'Execute agent' },
            { method: 'GET', path: '/teams', description: 'List teams' },
            { method: 'POST', path: '/teams', description: 'Create team' },
            { method: 'POST', path: '/teams/:teamId/execute', description: 'Execute team' },
            { method: 'GET', path: '/tools', description: 'List tools' },
            { method: 'POST', path: '/tools', description: 'Create tool' },
            { method: 'POST', path: '/tools/:toolId/execute', description: 'Execute tool' },
          ],
        },
        rag: {
          base: '/api/v1/rag',
          endpoints: [
            { method: 'GET', path: '/collections', description: 'List collections' },
            { method: 'POST', path: '/collections', description: 'Create collection' },
            { method: 'GET', path: '/collections/:collectionId', description: 'Get collection' },
            { method: 'POST', path: '/collections/:collectionId/documents', description: 'Ingest document' },
            { method: 'POST', path: '/collections/:collectionId/retrieve', description: 'Retrieve from collection' },
            { method: 'GET', path: '/documents/:documentId', description: 'Get document' },
            { method: 'DELETE', path: '/documents/:documentId', description: 'Delete document' },
            { method: 'POST', path: '/search', description: 'Search across collections' },
            { method: 'GET', path: '/pipelines', description: 'List RAG pipelines' },
            { method: 'POST', path: '/pipelines', description: 'Create RAG pipeline' },
            { method: 'POST', path: '/pipelines/:pipelineId/query', description: 'Execute RAG query' },
            { method: 'POST', path: '/query', description: 'Simple RAG query' },
          ],
        },
      },
    });
  });

  // 404 handler
  app.use((_req: Request, res: Response) => {
    res.status(404).json({
      error: {
        message: 'Not Found',
        code: 'NOT_FOUND',
      },
    });
  });

  // Error handler
  app.use(errorHandler);

  // Start server
  const server = app.listen(config.port, () => {
    console.log(`AI Platform service listening on port ${config.port}`);
    console.log(`Environment: ${config.nodeEnv}`);
  });

  // Graceful shutdown
  const shutdown = async (): Promise<void> => {
    console.log('Shutting down gracefully...');

    server.close(async () => {
      console.log('HTTP server closed');

      try {
        await redis.quit();
        console.log('Redis connection closed');
      } catch (error) {
        console.error('Error closing Redis connection:', error);
      }

      try {
        await db.end();
        console.log('Database connection closed');
      } catch (error) {
        console.error('Error closing database connection:', error);
      }

      process.exit(0);
    });

    // Force shutdown after 10 seconds
    setTimeout(() => {
      console.error('Could not close connections in time, forcefully shutting down');
      process.exit(1);
    }, 10000);
  };

  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);
}

main().catch((error) => {
  console.error('Failed to start AI Platform service:', error);
  process.exit(1);
});
