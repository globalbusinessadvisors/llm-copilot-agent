/**
 * Alerting Service
 *
 * Main entry point for the alerting and on-call management service.
 */

import express, { Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';
import { CronJob } from 'cron';
import winston from 'winston';

import { AlertService } from './services/alertService';
import { NotificationService } from './services/notificationService';
import { OnCallService } from './services/onCallService';

import { createAlertRoutes } from './routes/alerts';
import { createRuleRoutes } from './routes/rules';
import { createEscalationPolicyRoutes } from './routes/escalationPolicies';
import { createOnCallRoutes } from './routes/onCall';
import { createNotificationRoutes } from './routes/notifications';
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
  defaultMeta: { service: 'alerting-service' },
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
  port: parseInt(process.env.PORT || '3006', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  database: {
    host: process.env.DATABASE_HOST || 'localhost',
    port: parseInt(process.env.DATABASE_PORT || '5432', 10),
    database: process.env.DATABASE_NAME || 'llm_copilot_alerting',
    user: process.env.DATABASE_USER || 'postgres',
    password: process.env.DATABASE_PASSWORD || 'postgres',
    max: parseInt(process.env.DATABASE_POOL_MAX || '20', 10),
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 2000,
  },
  redis: {
    url: process.env.REDIS_URL || 'redis://localhost:6379',
  },
  escalation: {
    checkIntervalSeconds: parseInt(process.env.ESCALATION_CHECK_INTERVAL || '60', 10),
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

// Security middleware
app.use(helmet());
app.use(cors({
  origin: process.env.CORS_ORIGINS?.split(',') || ['http://localhost:3000'],
  credentials: true,
}));

// Body parsing
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true }));

// Compression
app.use(compression());

// Request logging
app.use((req: Request, res: Response, next: NextFunction) => {
  const startTime = Date.now();

  res.on('finish', () => {
    const duration = Date.now() - startTime;
    const logLevel = res.statusCode >= 400 ? 'warn' : 'info';

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
// Escalation Processing Job
// ===========================================

let escalationJob: CronJob | null = null;

async function processEscalations(alertService: AlertService): Promise<void> {
  try {
    // Get all triggered alerts
    const activeAlerts = await alertService.getActiveAlerts();

    for (const alert of activeAlerts) {
      if (alert.status === 'triggered') {
        // Check if escalation is due
        const escalationKey = `alert:escalation:${alert.id}`;
        const escalationData = await redis.get(escalationKey);

        if (!escalationData) {
          // No scheduled escalation, skip
          continue;
        }

        const scheduledTime = parseInt(escalationData, 10);
        const ttl = await redis.ttl(escalationKey);

        if (ttl <= 0) {
          // TTL expired, process escalation
          logger.info('Processing escalation', { alertId: alert.id });
          await alertService.processEscalation(alert.id);
        }
      }
    }
  } catch (error) {
    logger.error('Error processing escalations', { error });
  }
}

function startEscalationJob(alertService: AlertService): void {
  const cronExpression = `*/${config.escalation.checkIntervalSeconds} * * * * *`;

  escalationJob = new CronJob(cronExpression, () => {
    processEscalations(alertService);
  });

  escalationJob.start();
  logger.info('Escalation job started', { interval: config.escalation.checkIntervalSeconds });
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
    const notificationService = new NotificationService(db, {});
    const alertService = new AlertService(db, redis, notificationService);
    const onCallService = new OnCallService(db, redis);

    // Mount routes
    app.use('/health', createHealthRoutes({ db, redis }));
    app.use('/api/v1/alerts', createAlertRoutes(alertService));
    app.use('/api/v1/rules', createRuleRoutes(alertService));
    app.use('/api/v1/escalation-policies', createEscalationPolicyRoutes(db));
    app.use('/api/v1/on-call', createOnCallRoutes(onCallService));
    app.use('/api/v1/notifications', createNotificationRoutes(notificationService));

    // API documentation endpoint
    app.get('/api/v1', (req: Request, res: Response) => {
      res.json({
        service: 'alerting-service',
        version: '1.0.0',
        endpoints: {
          alerts: '/api/v1/alerts',
          rules: '/api/v1/rules',
          escalationPolicies: '/api/v1/escalation-policies',
          onCall: '/api/v1/on-call',
          notifications: '/api/v1/notifications',
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

    // Start escalation processing job
    startEscalationJob(alertService);

    // Start server
    app.listen(config.port, () => {
      logger.info(`Alerting service started`, {
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

  // Stop escalation job
  if (escalationJob) {
    escalationJob.stop();
    logger.info('Escalation job stopped');
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
