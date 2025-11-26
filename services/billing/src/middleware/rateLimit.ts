/**
 * Rate Limiting Middleware
 *
 * Provides rate limiting with Redis-backed storage for distributed environments.
 */

import { Request, Response, NextFunction } from 'express';
import { RedisClientType } from 'redis';
import { RateLimitError } from '../utils/errors';
import { config } from '../utils/config';
import { AuthenticatedRequest } from './auth';

interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
  keyPrefix?: string;
}

/**
 * Create a rate limiter with Redis storage
 */
export function createRateLimiter(
  redis: RedisClientType,
  options: Partial<RateLimitConfig> = {}
) {
  const windowMs = options.windowMs || config.rateLimitWindowMs;
  const maxRequests = options.maxRequests || config.rateLimitMaxRequests;
  const keyPrefix = options.keyPrefix || 'ratelimit';

  return async (
    req: AuthenticatedRequest,
    res: Response,
    next: NextFunction
  ): Promise<void> => {
    try {
      // Get identifier - tenant ID, user ID, or IP address
      const identifier =
        req.tenantId ||
        req.user?.userId ||
        req.ip ||
        req.headers['x-forwarded-for']?.toString() ||
        'unknown';

      const key = `${keyPrefix}:${identifier}`;
      const windowSeconds = Math.ceil(windowMs / 1000);

      // Increment counter
      const count = await redis.incr(key);

      // Set expiry on first request
      if (count === 1) {
        await redis.expire(key, windowSeconds);
      }

      // Get TTL for retry-after header
      const ttl = await redis.ttl(key);

      // Set rate limit headers
      res.setHeader('X-RateLimit-Limit', maxRequests.toString());
      res.setHeader('X-RateLimit-Remaining', Math.max(0, maxRequests - count).toString());
      res.setHeader('X-RateLimit-Reset', (Date.now() + ttl * 1000).toString());

      if (count > maxRequests) {
        res.setHeader('Retry-After', ttl.toString());
        return next(new RateLimitError(ttl));
      }

      next();
    } catch (error) {
      // If Redis fails, allow the request (fail open)
      next();
    }
  };
}

/**
 * Create a rate limiter for specific endpoints (more restrictive)
 */
export function createEndpointRateLimiter(
  redis: RedisClientType,
  endpoint: string,
  maxRequests: number,
  windowMs: number = 60000
) {
  return createRateLimiter(redis, {
    windowMs,
    maxRequests,
    keyPrefix: `ratelimit:${endpoint}`,
  });
}

/**
 * Create a usage-based rate limiter that considers quota
 */
export function createQuotaAwareRateLimiter(
  redis: RedisClientType,
  getQuota: (tenantId: string) => Promise<{ limit: number; used: number } | null>
) {
  return async (
    req: AuthenticatedRequest,
    res: Response,
    next: NextFunction
  ): Promise<void> => {
    try {
      if (!req.tenantId) {
        return next();
      }

      const quota = await getQuota(req.tenantId);

      if (!quota) {
        return next();
      }

      res.setHeader('X-Quota-Limit', quota.limit.toString());
      res.setHeader('X-Quota-Remaining', Math.max(0, quota.limit - quota.used).toString());

      if (quota.used >= quota.limit) {
        return next(new RateLimitError(3600)); // Retry in 1 hour
      }

      next();
    } catch (error) {
      next();
    }
  };
}

/**
 * In-memory rate limiter for development/testing
 */
export function createMemoryRateLimiter(options: Partial<RateLimitConfig> = {}) {
  const windowMs = options.windowMs || config.rateLimitWindowMs;
  const maxRequests = options.maxRequests || config.rateLimitMaxRequests;
  const store = new Map<string, { count: number; resetAt: number }>();

  // Cleanup expired entries periodically
  setInterval(() => {
    const now = Date.now();
    for (const [key, value] of store.entries()) {
      if (value.resetAt < now) {
        store.delete(key);
      }
    }
  }, 60000);

  return (
    req: AuthenticatedRequest,
    res: Response,
    next: NextFunction
  ): void => {
    const identifier =
      req.tenantId ||
      req.user?.userId ||
      req.ip ||
      'unknown';

    const now = Date.now();
    let record = store.get(identifier);

    if (!record || record.resetAt < now) {
      record = { count: 0, resetAt: now + windowMs };
      store.set(identifier, record);
    }

    record.count++;

    const remaining = Math.max(0, maxRequests - record.count);
    const retryAfter = Math.ceil((record.resetAt - now) / 1000);

    res.setHeader('X-RateLimit-Limit', maxRequests.toString());
    res.setHeader('X-RateLimit-Remaining', remaining.toString());
    res.setHeader('X-RateLimit-Reset', record.resetAt.toString());

    if (record.count > maxRequests) {
      res.setHeader('Retry-After', retryAfter.toString());
      return next(new RateLimitError(retryAfter));
    }

    next();
  };
}
