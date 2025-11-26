/**
 * Billing Service Entry Point
 *
 * Main application server for the billing and metering service.
 */

import express, { Express, Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import { Pool } from 'pg';
import { createClient, RedisClientType } from 'redis';

import { config, isProduction } from './utils/config';
import { logger } from './utils/logger';

// Services
import { UsageService } from './services/usageService';
import { StripeService } from './services/stripeService';

// Routes
import { createUsageRouter } from './routes/usage';
import { createSubscriptionsRouter } from './routes/subscriptions';
import { createInvoicesRouter } from './routes/invoices';
import { createPaymentMethodsRouter } from './routes/paymentMethods';
import { createWebhooksRouter } from './routes/webhooks';
import { createHealthRouter } from './routes/health';

// Middleware
import { errorHandler, notFoundHandler } from './middleware/errorHandler';
import { requestLogger } from './middleware/requestLogger';
import { createRateLimiter } from './middleware/rateLimit';

class BillingServer {
  private app: Express;
  private db: Pool;
  private redis: RedisClientType;
  private usageService!: UsageService;
  private stripeService!: StripeService;

  constructor() {
    this.app = express();
    this.db = new Pool({
      connectionString: config.databaseUrl,
      min: config.databasePoolMin,
      max: config.databasePoolMax,
    });
    this.redis = createClient({
      url: config.redisUrl,
    }) as RedisClientType;
  }

  async initialize(): Promise<void> {
    // Connect to Redis
    await this.redis.connect();
    logger.info('Connected to Redis');

    // Test database connection
    await this.db.query('SELECT 1');
    logger.info('Connected to PostgreSQL');

    // Initialize services
    this.usageService = new UsageService(this.db, this.redis);
    this.stripeService = new StripeService(
      this.db,
      config.stripeApiKey,
      config.stripeWebhookSecret
    );

    // Configure middleware
    this.setupMiddleware();

    // Configure routes
    this.setupRoutes();

    // Error handling
    this.setupErrorHandling();
  }

  private setupMiddleware(): void {
    // Security middleware
    this.app.use(helmet({
      contentSecurityPolicy: isProduction(),
    }));

    // CORS configuration
    this.app.use(cors({
      origin: config.corsOrigins === '*' ? '*' : config.corsOrigins.split(','),
      credentials: true,
    }));

    // Compression
    this.app.use(compression());

    // Request logging
    this.app.use(requestLogger);

    // Trust proxy for accurate IP detection
    if (isProduction()) {
      this.app.set('trust proxy', 1);
    }

    // Body parsing - JSON for most routes
    this.app.use('/api', express.json({ limit: '1mb' }));

    // Raw body for Stripe webhooks (must come before JSON parser)
    this.app.use('/webhooks/stripe', express.raw({ type: 'application/json' }));

    // Rate limiting
    this.app.use('/api', createRateLimiter(this.redis));
  }

  private setupRoutes(): void {
    // Health check routes (no auth required)
    this.app.use('/', createHealthRouter(this.db, this.redis));

    // Webhook routes (special handling)
    this.app.use('/webhooks', createWebhooksRouter(this.stripeService));

    // API routes
    this.app.use('/api/v1/usage', createUsageRouter(this.usageService));
    this.app.use('/api/v1/subscriptions', createSubscriptionsRouter(this.stripeService));
    this.app.use('/api/v1/invoices', createInvoicesRouter(this.stripeService));
    this.app.use('/api/v1/payment-methods', createPaymentMethodsRouter(this.stripeService));

    // API documentation endpoint
    this.app.get('/api', (_req: Request, res: Response) => {
      res.json({
        service: 'billing',
        version: '1.0.0',
        endpoints: {
          usage: '/api/v1/usage',
          subscriptions: '/api/v1/subscriptions',
          invoices: '/api/v1/invoices',
          paymentMethods: '/api/v1/payment-methods',
        },
        documentation: '/docs',
      });
    });
  }

  private setupErrorHandling(): void {
    // 404 handler
    this.app.use(notFoundHandler);

    // Global error handler
    this.app.use(errorHandler);
  }

  async start(): Promise<void> {
    await this.initialize();

    const server = this.app.listen(config.port, () => {
      logger.info(`Billing service started`, {
        port: config.port,
        environment: config.nodeEnv,
      });
    });

    // Graceful shutdown
    const shutdown = async (signal: string) => {
      logger.info(`${signal} received, shutting down gracefully`);

      server.close(async () => {
        logger.info('HTTP server closed');

        try {
          await this.redis.quit();
          logger.info('Redis connection closed');
        } catch (error) {
          logger.error('Error closing Redis connection', { error });
        }

        try {
          await this.db.end();
          logger.info('Database pool closed');
        } catch (error) {
          logger.error('Error closing database pool', { error });
        }

        process.exit(0);
      });

      // Force close after 30 seconds
      setTimeout(() => {
        logger.error('Forced shutdown after timeout');
        process.exit(1);
      }, 30000);
    };

    process.on('SIGTERM', () => shutdown('SIGTERM'));
    process.on('SIGINT', () => shutdown('SIGINT'));

    // Handle uncaught errors
    process.on('uncaughtException', (error) => {
      logger.error('Uncaught exception', { error: error.message, stack: error.stack });
      process.exit(1);
    });

    process.on('unhandledRejection', (reason, promise) => {
      logger.error('Unhandled rejection', { reason, promise });
    });
  }
}

// Start the server
const server = new BillingServer();
server.start().catch((error) => {
  logger.error('Failed to start billing service', { error });
  process.exit(1);
});

export { BillingServer };
