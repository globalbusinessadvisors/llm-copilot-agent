/**
 * Configuration Module
 *
 * Centralized configuration management with environment variable parsing and validation.
 */

import { z } from 'zod';

const ConfigSchema = z.object({
  // Server
  port: z.number().default(3001),
  nodeEnv: z.enum(['development', 'production', 'test']).default('development'),

  // Database
  databaseUrl: z.string(),
  databasePoolMin: z.number().default(2),
  databasePoolMax: z.number().default(10),

  // Redis
  redisUrl: z.string(),

  // Stripe
  stripeApiKey: z.string(),
  stripeWebhookSecret: z.string(),
  stripePublishableKey: z.string().optional(),

  // Security
  jwtSecret: z.string(),
  apiKeyHeader: z.string().default('x-api-key'),

  // Rate Limiting
  rateLimitWindowMs: z.number().default(60000), // 1 minute
  rateLimitMaxRequests: z.number().default(100),

  // Logging
  logLevel: z.string().default('info'),
  logFile: z.string().optional(),
  errorLogFile: z.string().optional(),

  // CORS
  corsOrigins: z.string().default('*'),

  // Service URLs
  authServiceUrl: z.string().optional(),
  notificationServiceUrl: z.string().optional(),
});

export type Config = z.infer<typeof ConfigSchema>;

function loadConfig(): Config {
  const rawConfig = {
    port: parseInt(process.env.PORT || '3001', 10),
    nodeEnv: process.env.NODE_ENV || 'development',
    databaseUrl: process.env.DATABASE_URL || 'postgresql://localhost:5432/billing',
    databasePoolMin: parseInt(process.env.DB_POOL_MIN || '2', 10),
    databasePoolMax: parseInt(process.env.DB_POOL_MAX || '10', 10),
    redisUrl: process.env.REDIS_URL || 'redis://localhost:6379',
    stripeApiKey: process.env.STRIPE_API_KEY || '',
    stripeWebhookSecret: process.env.STRIPE_WEBHOOK_SECRET || '',
    stripePublishableKey: process.env.STRIPE_PUBLISHABLE_KEY,
    jwtSecret: process.env.JWT_SECRET || 'development-secret',
    apiKeyHeader: process.env.API_KEY_HEADER || 'x-api-key',
    rateLimitWindowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '60000', 10),
    rateLimitMaxRequests: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100', 10),
    logLevel: process.env.LOG_LEVEL || 'info',
    logFile: process.env.LOG_FILE,
    errorLogFile: process.env.ERROR_LOG_FILE,
    corsOrigins: process.env.CORS_ORIGINS || '*',
    authServiceUrl: process.env.AUTH_SERVICE_URL,
    notificationServiceUrl: process.env.NOTIFICATION_SERVICE_URL,
  };

  const result = ConfigSchema.safeParse(rawConfig);
  if (!result.success) {
    console.error('Configuration validation failed:');
    console.error(result.error.format());
    throw new Error('Invalid configuration');
  }

  return result.data;
}

export const config = loadConfig();

/**
 * Check if running in production
 */
export function isProduction(): boolean {
  return config.nodeEnv === 'production';
}

/**
 * Check if running in development
 */
export function isDevelopment(): boolean {
  return config.nodeEnv === 'development';
}

/**
 * Check if running in test
 */
export function isTest(): boolean {
  return config.nodeEnv === 'test';
}
