/**
 * Status Page Service
 *
 * Main entry point for the public status page service.
 */

import express, { Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';
import { CronJob } from 'cron';
import winston from 'winston';

import { StatusService } from './services/statusService';
import { createStatusRoutes } from './routes/status';
import { createHealthRoutes } from './routes/health';

// ===========================================
// Logger Setup
// ===========================================

const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.errors({ stack: true }),
    winston.format.json()
  ),
  defaultMeta: { service: 'status-page-service' },
  transports: [
    new winston.transports.Console({
      format: winston.format.combine(
        winston.format.colorize(),
        winston.format.simple()
      ),
    }),
  ],
});

// ===========================================
// Configuration
// ===========================================

const config = {
  port: parseInt(process.env.PORT || '3007', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  database: {
    host: process.env.DATABASE_HOST || 'localhost',
    port: parseInt(process.env.DATABASE_PORT || '5432', 10),
    database: process.env.DATABASE_NAME || 'llm_copilot_status',
    user: process.env.DATABASE_USER || 'postgres',
    password: process.env.DATABASE_PASSWORD || 'postgres',
    max: parseInt(process.env.DATABASE_POOL_MAX || '20', 10),
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 2000,
  },
  redis: {
    url: process.env.REDIS_URL || 'redis://localhost:6379',
  },
  healthCheck: {
    intervalSeconds: parseInt(process.env.HEALTH_CHECK_INTERVAL || '60', 10),
  },
};

// ===========================================
// Database & Redis Setup
// ===========================================

const db = new Pool(config.database);

db.on('error', (err) => {
  logger.error('Unexpected database error', { error: err.message });
});

db.on('connect', () => {
  logger.debug('New database connection established');
});

let redis: RedisClientType;

async function initializeRedis(): Promise<RedisClientType> {
  const client = createClient({ url: config.redis.url });

  client.on('error', (err) => {
    logger.error('Redis error', { error: err.message });
  });

  client.on('connect', () => {
    logger.info('Connected to Redis');
  });

  await client.connect();
  return client as RedisClientType;
}

// ===========================================
// Express App Setup
// ===========================================

const app = express();

// Security middleware - more permissive for public status page
app.use(helmet({
  crossOriginResourcePolicy: { policy: 'cross-origin' },
}));

// CORS - allow all origins for public status page
app.use(cors({
  origin: '*',
  credentials: false,
}));

// Body parsing
app.use(express.json({ limit: '1mb' }));
app.use(express.urlencoded({ extended: true }));

// Compression
app.use(compression());

// Request logging
app.use((req: Request, res: Response, next: NextFunction) => {
  const startTime = Date.now();

  res.on('finish', () => {
    const duration = Date.now() - startTime;
    const logLevel = res.statusCode >= 400 ? 'warn' : 'debug';

    logger[logLevel]('HTTP request', {
      method: req.method,
      path: req.path,
      statusCode: res.statusCode,
      duration,
      userAgent: req.get('user-agent'),
      ip: req.ip,
    });
  });

  next();
});

// ===========================================
// Health Check Job
// ===========================================

let healthCheckJob: CronJob | null = null;

async function runHealthChecks(statusService: StatusService): Promise<void> {
  try {
    const services = await statusService.getServices();

    for (const service of services) {
      if (service.healthCheckUrl) {
        try {
          const result = await statusService.healthCheck(service);
          logger.debug('Health check completed', {
            service: service.name,
            success: result.success,
            latency: result.latency,
          });
        } catch (error) {
          logger.error('Health check failed', {
            service: service.name,
            error: error instanceof Error ? error.message : 'Unknown error',
          });
        }
      }
    }
  } catch (error) {
    logger.error('Error running health checks', { error });
  }
}

function startHealthCheckJob(statusService: StatusService): void {
  const cronExpression = `*/${config.healthCheck.intervalSeconds} * * * * *`;

  healthCheckJob = new CronJob(cronExpression, () => {
    runHealthChecks(statusService);
  });

  healthCheckJob.start();
  logger.info('Health check job started', { interval: config.healthCheck.intervalSeconds });
}

// ===========================================
// Main Server Startup
// ===========================================

async function startServer(): Promise<void> {
  try {
    // Initialize Redis
    redis = await initializeRedis();

    // Test database connection
    await db.query('SELECT NOW()');
    logger.info('Database connection verified');

    // Initialize services
    const statusService = new StatusService(db, redis);

    // Mount routes
    app.use('/health', createHealthRoutes({ db, redis }));
    app.use('/api/v1/status', createStatusRoutes(statusService));

    // Root endpoint - redirect to status
    app.get('/', (req: Request, res: Response) => {
      res.redirect('/api/v1/status');
    });

    // API documentation endpoint
    app.get('/api/v1', (req: Request, res: Response) => {
      res.json({
        service: 'status-page-service',
        version: '1.0.0',
        endpoints: {
          status: '/api/v1/status',
          services: '/api/v1/status/services',
          incidents: '/api/v1/status/incidents',
          maintenance: '/api/v1/status/maintenance',
          health: '/health',
        },
        documentation: '/api/v1/docs',
      });
    });

    // 404 handler
    app.use((req: Request, res: Response) => {
      res.status(404).json({
        success: false,
        error: 'Not found',
        path: req.path,
      });
    });

    // Global error handler
    app.use((err: Error, req: Request, res: Response, next: NextFunction) => {
      logger.error('Unhandled error', {
        error: err.message,
        stack: err.stack,
        path: req.path,
        method: req.method,
      });

      res.status(500).json({
        success: false,
        error: config.nodeEnv === 'production' ? 'Internal server error' : err.message,
      });
    });

    // Start health check job
    startHealthCheckJob(statusService);

    // Start server
    app.listen(config.port, () => {
      logger.info(`Status page service started`, {
        port: config.port,
        environment: config.nodeEnv,
      });
    });
  } catch (error) {
    logger.error('Failed to start server', { error });
    process.exit(1);
  }
}

// ===========================================
// Graceful Shutdown
// ===========================================

async function shutdown(signal: string): Promise<void> {
  logger.info(`Received ${signal}, shutting down gracefully`);

  // Stop health check job
  if (healthCheckJob) {
    healthCheckJob.stop();
    logger.info('Health check job stopped');
  }

  // Close Redis connection
  if (redis) {
    await redis.quit();
    logger.info('Redis connection closed');
  }

  // Close database pool
  await db.end();
  logger.info('Database pool closed');

  process.exit(0);
}

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));

process.on('unhandledRejection', (reason, promise) => {
  logger.error('Unhandled rejection', { reason, promise });
});

process.on('uncaughtException', (error) => {
  logger.error('Uncaught exception', { error: error.message, stack: error.stack });
  process.exit(1);
});

// Start the server
startServer();
