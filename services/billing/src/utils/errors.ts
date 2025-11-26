/**
 * Custom Error Classes
 *
 * Provides typed errors for consistent error handling across the billing service.
 */

export class BillingError extends Error {
  public readonly code: string;
  public readonly statusCode: number;
  public readonly details?: Record<string, unknown>;

  constructor(
    message: string,
    code: string,
    statusCode: number = 500,
    details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'BillingError';
    this.code = code;
    this.statusCode = statusCode;
    this.details = details;
    Error.captureStackTrace(this, this.constructor);
  }

  toJSON(): Record<string, unknown> {
    return {
      error: {
        code: this.code,
        message: this.message,
        details: this.details,
      },
    };
  }
}

export class NotFoundError extends BillingError {
  constructor(resource: string, identifier: string) {
    super(
      `${resource} not found: ${identifier}`,
      'NOT_FOUND',
      404,
      { resource, identifier }
    );
    this.name = 'NotFoundError';
  }
}

export class ValidationError extends BillingError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, 'VALIDATION_ERROR', 400, details);
    this.name = 'ValidationError';
  }
}

export class AuthenticationError extends BillingError {
  constructor(message: string = 'Authentication required') {
    super(message, 'AUTHENTICATION_ERROR', 401);
    this.name = 'AuthenticationError';
  }
}

export class AuthorizationError extends BillingError {
  constructor(message: string = 'Insufficient permissions') {
    super(message, 'AUTHORIZATION_ERROR', 403);
    this.name = 'AuthorizationError';
  }
}

export class QuotaExceededError extends BillingError {
  constructor(metric: string, used: number, limit: number) {
    super(
      `Quota exceeded for ${metric}: ${used}/${limit}`,
      'QUOTA_EXCEEDED',
      429,
      { metric, used, limit }
    );
    this.name = 'QuotaExceededError';
  }
}

export class PaymentRequiredError extends BillingError {
  constructor(message: string = 'Payment required') {
    super(message, 'PAYMENT_REQUIRED', 402);
    this.name = 'PaymentRequiredError';
  }
}

export class SubscriptionError extends BillingError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, 'SUBSCRIPTION_ERROR', 400, details);
    this.name = 'SubscriptionError';
  }
}

export class StripeError extends BillingError {
  constructor(message: string, stripeErrorCode?: string) {
    super(
      message,
      'STRIPE_ERROR',
      502,
      stripeErrorCode ? { stripeErrorCode } : undefined
    );
    this.name = 'StripeError';
  }
}

export class RateLimitError extends BillingError {
  constructor(retryAfter: number) {
    super(
      'Too many requests',
      'RATE_LIMIT_ERROR',
      429,
      { retryAfter }
    );
    this.name = 'RateLimitError';
  }
}

export class ConflictError extends BillingError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, 'CONFLICT_ERROR', 409, details);
    this.name = 'ConflictError';
  }
}

/**
 * Check if an error is a BillingError
 */
export function isBillingError(error: unknown): error is BillingError {
  return error instanceof BillingError;
}

/**
 * Convert any error to a BillingError
 */
export function toBillingError(error: unknown): BillingError {
  if (isBillingError(error)) {
    return error;
  }

  if (error instanceof Error) {
    return new BillingError(error.message, 'INTERNAL_ERROR', 500);
  }

  return new BillingError('An unexpected error occurred', 'INTERNAL_ERROR', 500);
}
