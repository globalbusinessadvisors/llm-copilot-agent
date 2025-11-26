/**
 * Request Logging Middleware
 *
 * Logs incoming HTTP requests with timing and metadata.
 */

import { Request, Response, NextFunction } from 'express';
import { v4 as uuidv4 } from 'uuid';
import { logger, logRequest } from '../utils/logger';
import { AuthenticatedRequest } from './auth';

/**
 * Add request ID and timing to all requests
 */
export function requestLogger(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  // Generate unique request ID
  const requestId = (req.headers['x-request-id'] as string) || uuidv4();
  req.headers['x-request-id'] = requestId;
  res.setHeader('X-Request-ID', requestId);

  // Record start time
  const startTime = Date.now();

  // Log response when finished
  res.on('finish', () => {
    const duration = Date.now() - startTime;

    logRequest(req.method, req.path, res.statusCode, duration, {
      requestId,
      tenantId: req.tenantId,
      userId: req.user?.userId,
      userAgent: req.headers['user-agent'],
      contentLength: res.getHeader('content-length'),
      ip: req.ip || req.headers['x-forwarded-for'],
    });
  });

  next();
}

/**
 * Skip logging for health check endpoints
 */
export function skipHealthCheckLogs(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  if (req.path === '/health' || req.path === '/ready') {
    // Don't log health checks
    res.on('finish', () => {});
  }
  next();
}

/**
 * Log request body for debugging (use sparingly in production)
 */
export function debugRequestBody(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  if (process.env.DEBUG_REQUESTS === 'true') {
    logger.debug('Request body', {
      method: req.method,
      path: req.path,
      body: req.body,
      query: req.query,
      params: req.params,
    });
  }
  next();
}
