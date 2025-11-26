/**
 * Logger Utility
 *
 * Provides structured logging with levels, formatting, and context.
 */

import winston from 'winston';

const logFormat = winston.format.combine(
  winston.format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' }),
  winston.format.errors({ stack: true }),
  winston.format.json()
);

const consoleFormat = winston.format.combine(
  winston.format.colorize(),
  winston.format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss' }),
  winston.format.printf(({ timestamp, level, message, ...meta }) => {
    const metaStr = Object.keys(meta).length ? JSON.stringify(meta) : '';
    return `${timestamp} [${level}]: ${message} ${metaStr}`;
  })
);

const transports: winston.transport[] = [];

// Console transport for development
if (process.env.NODE_ENV !== 'production') {
  transports.push(
    new winston.transports.Console({
      format: consoleFormat,
    })
  );
} else {
  // JSON format for production (better for log aggregation)
  transports.push(
    new winston.transports.Console({
      format: logFormat,
    })
  );
}

// File transports for production
if (process.env.LOG_FILE) {
  transports.push(
    new winston.transports.File({
      filename: process.env.LOG_FILE,
      format: logFormat,
      maxsize: 10 * 1024 * 1024, // 10MB
      maxFiles: 5,
    })
  );
}

if (process.env.ERROR_LOG_FILE) {
  transports.push(
    new winston.transports.File({
      filename: process.env.ERROR_LOG_FILE,
      level: 'error',
      format: logFormat,
      maxsize: 10 * 1024 * 1024,
      maxFiles: 5,
    })
  );
}

export const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  defaultMeta: {
    service: 'billing-service',
    environment: process.env.NODE_ENV || 'development',
  },
  transports,
});

/**
 * Create a child logger with additional context
 */
export function createLogger(context: Record<string, unknown>): winston.Logger {
  return logger.child(context);
}

/**
 * Request logger for HTTP requests
 */
export function logRequest(
  method: string,
  path: string,
  statusCode: number,
  durationMs: number,
  metadata?: Record<string, unknown>
): void {
  const level = statusCode >= 500 ? 'error' : statusCode >= 400 ? 'warn' : 'info';
  logger.log(level, 'HTTP Request', {
    method,
    path,
    statusCode,
    durationMs,
    ...metadata,
  });
}

/**
 * Audit logger for billing events
 */
export function logAudit(
  action: string,
  tenantId: string,
  userId: string | undefined,
  metadata?: Record<string, unknown>
): void {
  logger.info('Audit', {
    action,
    tenantId,
    userId,
    timestamp: new Date().toISOString(),
    ...metadata,
  });
}
