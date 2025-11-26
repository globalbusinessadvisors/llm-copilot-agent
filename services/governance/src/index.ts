/**
 * Governance Service
 *
 * Enterprise governance platform for content filtering, usage policies, audit trail,
 * and data lineage tracking.
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';
import { createGovernanceRoutes } from './routes/governance';
import { ContentFilterService } from './services/contentFilterService';
import { PolicyService } from './services/policyService';
import { AuditService } from './services/auditService';
import { DataLineageService } from './services/dataLineageService';

// Configuration
const config = {
  port: parseInt(process.env.PORT || '3010', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  database: {
    host: process.env.DB_HOST || 'localhost',
    port: parseInt(process.env.DB_PORT || '5432', 10),
    database: process.env.DB_NAME || 'governance',
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
  const contentFilterService = new ContentFilterService(db, redis);
  const policyService = new PolicyService(db, redis);
  const auditService = new AuditService(db, redis);
  const dataLineageService = new DataLineageService(db, redis);

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

  // Health check routes
  app.get('/health', (_req: Request, res: Response) => {
    res.json({
      status: 'healthy',
      service: 'governance',
      timestamp: new Date().toISOString(),
    });
  });

  app.get('/health/detailed', async (_req: Request, res: Response) => {
    const checks: Record<string, { status: string; latency?: number; error?: string }> = {};

    // Check database
    try {
      const startDb = Date.now();
      await db.query('SELECT 1');
      checks.database = {
        status: 'healthy',
        latency: Date.now() - startDb,
      };
    } catch (error) {
      checks.database = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check Redis
    try {
      const startRedis = Date.now();
      await redis.ping();
      checks.redis = {
        status: 'healthy',
        latency: Date.now() - startRedis,
      };
    } catch (error) {
      checks.redis = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check OpenAI (for content moderation)
    if (process.env.OPENAI_API_KEY) {
      checks.openai = { status: 'configured' };
    } else {
      checks.openai = { status: 'not_configured' };
    }

    const overallStatus = Object.values(checks).every(
      c => c.status === 'healthy' || c.status === 'configured' || c.status === 'not_configured'
    ) ? 'healthy' : 'degraded';

    res.status(overallStatus === 'healthy' ? 200 : 503).json({
      status: overallStatus,
      service: 'governance',
      version: process.env.npm_package_version || '1.0.0',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  // Mount routes
  app.use('/api/v1/governance', createGovernanceRoutes(
    contentFilterService,
    policyService,
    auditService,
    dataLineageService
  ));

  // Root endpoint
  app.get('/', (_req: Request, res: Response) => {
    res.json({
      service: 'governance',
      version: '1.0.0',
      description: 'Enterprise Governance Platform',
      features: [
        'Content filtering and safety',
        'Usage policy management and enforcement',
        'Comprehensive audit trail',
        'Data lineage tracking',
      ],
      endpoints: {
        health: '/health',
        governance: '/api/v1/governance',
      },
    });
  });

  // API documentation endpoint
  app.get('/api/v1', (_req: Request, res: Response) => {
    res.json({
      version: 'v1',
      modules: {
        contentFiltering: {
          base: '/api/v1/governance/filters',
          endpoints: [
            { method: 'GET', path: '/rules', description: 'List content filter rules' },
            { method: 'POST', path: '/rules', description: 'Create filter rule' },
            { method: 'GET', path: '/rules/:ruleId', description: 'Get filter rule' },
            { method: 'PATCH', path: '/rules/:ruleId', description: 'Update filter rule' },
            { method: 'DELETE', path: '/rules/:ruleId', description: 'Delete filter rule' },
            { method: 'POST', path: '/analyze', description: 'Filter content' },
            { method: 'GET', path: '/statistics', description: 'Get filter statistics' },
          ],
        },
        policies: {
          base: '/api/v1/governance/policies',
          endpoints: [
            { method: 'GET', path: '/', description: 'List policies' },
            { method: 'POST', path: '/', description: 'Create policy' },
            { method: 'GET', path: '/:policyId', description: 'Get policy' },
            { method: 'PATCH', path: '/:policyId', description: 'Update policy' },
            { method: 'POST', path: '/:policyId/activate', description: 'Activate policy' },
            { method: 'POST', path: '/:policyId/deprecate', description: 'Deprecate policy' },
            { method: 'POST', path: '/evaluate', description: 'Evaluate policy' },
            { method: 'GET', path: '/violations', description: 'Get violations' },
            { method: 'GET', path: '/statistics', description: 'Get policy statistics' },
          ],
        },
        audit: {
          base: '/api/v1/governance/audit',
          endpoints: [
            { method: 'POST', path: '/events', description: 'Record audit event' },
            { method: 'GET', path: '/events', description: 'Search audit events' },
            { method: 'GET', path: '/events/:eventId', description: 'Get audit event' },
            { method: 'GET', path: '/resources/:resourceType/:resourceId', description: 'Get resource history' },
            { method: 'GET', path: '/actors/:actorId', description: 'Get actor history' },
            { method: 'GET', path: '/statistics', description: 'Get audit statistics' },
            { method: 'GET', path: '/anomalies', description: 'Detect anomalies' },
          ],
        },
        lineage: {
          base: '/api/v1/governance/lineage',
          endpoints: [
            { method: 'GET', path: '/nodes', description: 'List lineage nodes' },
            { method: 'POST', path: '/nodes', description: 'Create lineage node' },
            { method: 'GET', path: '/nodes/:nodeId', description: 'Get lineage node' },
            { method: 'PATCH', path: '/nodes/:nodeId', description: 'Update lineage node' },
            { method: 'DELETE', path: '/nodes/:nodeId', description: 'Delete lineage node' },
            { method: 'GET', path: '/nodes/:nodeId/graph', description: 'Get lineage graph' },
            { method: 'GET', path: '/nodes/:nodeId/impact', description: 'Analyze impact' },
            { method: 'GET', path: '/edges', description: 'List lineage edges' },
            { method: 'POST', path: '/edges', description: 'Create lineage edge' },
            { method: 'DELETE', path: '/edges/:edgeId', description: 'Delete lineage edge' },
            { method: 'GET', path: '/path', description: 'Find path between nodes' },
            { method: 'GET', path: '/search', description: 'Search lineage nodes' },
            { method: 'GET', path: '/statistics', description: 'Get lineage statistics' },
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
    console.log(`Governance service listening on port ${config.port}`);
    console.log(`Environment: ${config.nodeEnv}`);
  });

  // Graceful shutdown
  const shutdown = async (): Promise<void> => {
    console.log('Shutting down gracefully...');

    // Stop audit service flush interval
    auditService.stopFlushInterval();

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
  console.error('Failed to start Governance service:', error);
  process.exit(1);
});
