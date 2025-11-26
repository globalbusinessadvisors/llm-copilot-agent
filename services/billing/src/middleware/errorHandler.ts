/**
 * Error Handling Middleware
 *
 * Centralized error handling for Express with proper logging and response formatting.
 */

import { Request, Response, NextFunction } from 'express';
import { ZodError } from 'zod';
import { BillingError, ValidationError, toBillingError } from '../utils/errors';
import { logger } from '../utils/logger';
import { config } from '../utils/config';

/**
 * Not found handler
 */
export function notFoundHandler(
  req: Request,
  res: Response,
  _next: NextFunction
): void {
  res.status(404).json({
    error: {
      code: 'NOT_FOUND',
      message: `Route not found: ${req.method} ${req.path}`,
    },
  });
}

/**
 * Global error handler
 */
export function errorHandler(
  err: Error,
  req: Request,
  res: Response,
  _next: NextFunction
): void {
  // Handle Zod validation errors
  if (err instanceof ZodError) {
    const validationError = new ValidationError('Validation failed', {
      errors: err.errors.map(e => ({
        path: e.path.join('.'),
        message: e.message,
      })),
    });

    logger.warn('Validation error', {
      path: req.path,
      method: req.method,
      errors: err.errors,
    });

    res.status(400).json(validationError.toJSON());
    return;
  }

  // Handle BillingError instances
  if (err instanceof BillingError) {
    if (err.statusCode >= 500) {
      logger.error('Billing error', {
        code: err.code,
        message: err.message,
        statusCode: err.statusCode,
        path: req.path,
        method: req.method,
        stack: err.stack,
      });
    } else {
      logger.warn('Billing error', {
        code: err.code,
        message: err.message,
        statusCode: err.statusCode,
        path: req.path,
        method: req.method,
      });
    }

    res.status(err.statusCode).json(err.toJSON());
    return;
  }

  // Handle unknown errors
  const billingError = toBillingError(err);

  logger.error('Unhandled error', {
    message: err.message,
    name: err.name,
    path: req.path,
    method: req.method,
    stack: err.stack,
  });

  // In production, don't expose internal error details
  if (config.nodeEnv === 'production') {
    res.status(500).json({
      error: {
        code: 'INTERNAL_ERROR',
        message: 'An unexpected error occurred',
      },
    });
  } else {
    res.status(billingError.statusCode).json({
      error: {
        code: billingError.code,
        message: err.message,
        stack: err.stack,
      },
    });
  }
}

/**
 * Async handler wrapper - catches async errors and passes them to error handler
 */
export function asyncHandler<T>(
  fn: (req: Request, res: Response, next: NextFunction) => Promise<T>
) {
  return (req: Request, res: Response, next: NextFunction): void => {
    Promise.resolve(fn(req, res, next)).catch(next);
  };
}
