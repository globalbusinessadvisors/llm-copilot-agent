/**
 * Compliance Service
 *
 * Enterprise compliance platform supporting SOC 2, HIPAA, and data residency requirements.
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';
import { createComplianceRoutes } from './routes/compliance';
import { createHIPAARoutes } from './routes/hipaa';
import { createDataResidencyRoutes } from './routes/dataResidency';
import { ComplianceService } from './services/complianceService';
import { HIPAAService } from './services/hipaaService';
import { DataResidencyService } from './services/dataResidencyService';

// Configuration
const config = {
  port: parseInt(process.env.PORT || '3009', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  database: {
    host: process.env.DB_HOST || 'localhost',
    port: parseInt(process.env.DB_PORT || '5432', 10),
    database: process.env.DB_NAME || 'compliance',
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
  const complianceService = new ComplianceService(db, redis);
  const hipaaService = new HIPAAService(db, redis);
  const dataResidencyService = new DataResidencyService(db, redis);

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
      service: 'compliance',
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

    const overallStatus = Object.values(checks).every(c => c.status === 'healthy')
      ? 'healthy'
      : 'degraded';

    res.status(overallStatus === 'healthy' ? 200 : 503).json({
      status: overallStatus,
      service: 'compliance',
      version: process.env.npm_package_version || '1.0.0',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  // Mount routes
  app.use('/api/v1/compliance', createComplianceRoutes(complianceService));
  app.use('/api/v1/hipaa', createHIPAARoutes(hipaaService));
  app.use('/api/v1/data-residency', createDataResidencyRoutes(dataResidencyService));

  // Root endpoint
  app.get('/', (_req: Request, res: Response) => {
    res.json({
      service: 'compliance',
      version: '1.0.0',
      description: 'Enterprise Compliance Platform',
      features: [
        'SOC 2 Type I/II compliance management',
        'HIPAA compliance and PHI access logging',
        'Data residency policy enforcement',
        'Audit management and findings tracking',
        'Compliance reporting and dashboards',
      ],
      endpoints: {
        health: '/health',
        compliance: '/api/v1/compliance',
        hipaa: '/api/v1/hipaa',
        dataResidency: '/api/v1/data-residency',
      },
    });
  });

  // API documentation endpoint
  app.get('/api/v1', (_req: Request, res: Response) => {
    res.json({
      version: 'v1',
      modules: {
        compliance: {
          base: '/api/v1/compliance',
          endpoints: [
            { method: 'GET', path: '/controls', description: 'List compliance controls' },
            { method: 'POST', path: '/controls', description: 'Create control' },
            { method: 'GET', path: '/controls/:controlId', description: 'Get control' },
            { method: 'PATCH', path: '/controls/:controlId/status', description: 'Update control status' },
            { method: 'POST', path: '/controls/:controlId/test', description: 'Record control test' },
            { method: 'GET', path: '/audits', description: 'List audits' },
            { method: 'POST', path: '/audits', description: 'Create audit' },
            { method: 'GET', path: '/findings', description: 'List findings' },
            { method: 'POST', path: '/findings', description: 'Create finding' },
            { method: 'POST', path: '/reports', description: 'Generate compliance report' },
            { method: 'GET', path: '/dashboard', description: 'Get dashboard metrics' },
          ],
        },
        hipaa: {
          base: '/api/v1/hipaa',
          endpoints: [
            { method: 'POST', path: '/phi-access', description: 'Log PHI access' },
            { method: 'GET', path: '/phi-access', description: 'Get PHI access logs' },
            { method: 'POST', path: '/phi-access/report', description: 'Generate access report' },
            { method: 'GET', path: '/baa', description: 'List BAAs' },
            { method: 'POST', path: '/baa', description: 'Create BAA' },
            { method: 'GET', path: '/requirements', description: 'Get HIPAA requirements' },
            { method: 'GET', path: '/assessment', description: 'Assess HIPAA compliance' },
            { method: 'POST', path: '/breaches', description: 'Report a breach' },
          ],
        },
        dataResidency: {
          base: '/api/v1/data-residency',
          endpoints: [
            { method: 'GET', path: '/policies', description: 'List data residency policies' },
            { method: 'POST', path: '/policies', description: 'Create policy' },
            { method: 'GET', path: '/assets', description: 'List data assets' },
            { method: 'POST', path: '/assets', description: 'Register data asset' },
            { method: 'GET', path: '/assets/:assetId/compliance', description: 'Check asset compliance' },
            { method: 'GET', path: '/transfers', description: 'List transfer requests' },
            { method: 'POST', path: '/transfers', description: 'Request data transfer' },
            { method: 'POST', path: '/transfers/:requestId/approve', description: 'Approve transfer' },
            { method: 'GET', path: '/report', description: 'Generate data residency report' },
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
    console.log(`Compliance service listening on port ${config.port}`);
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
  console.error('Failed to start Compliance service:', error);
  process.exit(1);
});
