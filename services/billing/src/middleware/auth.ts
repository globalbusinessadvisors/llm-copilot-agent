/**
 * Authentication Middleware
 *
 * Handles JWT validation, API key authentication, and tenant context.
 */

import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { config } from '../utils/config';
import { AuthenticationError, AuthorizationError } from '../utils/errors';
import { logger } from '../utils/logger';

export interface AuthenticatedUser {
  userId: string;
  tenantId: string;
  email: string;
  roles: string[];
}

export interface AuthenticatedRequest extends Request {
  user?: AuthenticatedUser;
  tenantId?: string;
}

/**
 * JWT Authentication middleware
 */
export function authenticate(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  try {
    const authHeader = req.headers.authorization;

    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      throw new AuthenticationError('Missing or invalid authorization header');
    }

    const token = authHeader.substring(7);

    const decoded = jwt.verify(token, config.jwtSecret) as AuthenticatedUser;

    req.user = decoded;
    req.tenantId = decoded.tenantId;

    next();
  } catch (error) {
    if (error instanceof jwt.JsonWebTokenError) {
      next(new AuthenticationError('Invalid token'));
    } else if (error instanceof jwt.TokenExpiredError) {
      next(new AuthenticationError('Token expired'));
    } else {
      next(error);
    }
  }
}

/**
 * API Key Authentication middleware
 */
export function authenticateApiKey(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  try {
    const apiKey = req.headers[config.apiKeyHeader] as string;

    if (!apiKey) {
      throw new AuthenticationError('Missing API key');
    }

    // In production, this would validate against a database
    // For now, we extract tenant info from the API key format
    // Format: sk_tenant_{tenantId}_{randomKey}
    const parts = apiKey.split('_');
    if (parts.length < 3 || parts[0] !== 'sk' || parts[1] !== 'tenant') {
      throw new AuthenticationError('Invalid API key format');
    }

    req.tenantId = parts[2];

    next();
  } catch (error) {
    next(error);
  }
}

/**
 * Combined authentication (JWT or API Key)
 */
export function authenticateAny(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  const authHeader = req.headers.authorization;
  const apiKey = req.headers[config.apiKeyHeader] as string;

  if (authHeader && authHeader.startsWith('Bearer ')) {
    return authenticate(req, res, next);
  }

  if (apiKey) {
    return authenticateApiKey(req, res, next);
  }

  next(new AuthenticationError('No authentication credentials provided'));
}

/**
 * Role-based authorization middleware
 */
export function authorize(...allowedRoles: string[]) {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction): void => {
    if (!req.user) {
      return next(new AuthenticationError('User not authenticated'));
    }

    const hasRole = req.user.roles.some(role => allowedRoles.includes(role));

    if (!hasRole) {
      logger.warn('Authorization failed', {
        userId: req.user.userId,
        requiredRoles: allowedRoles,
        userRoles: req.user.roles,
      });
      return next(new AuthorizationError('Insufficient permissions'));
    }

    next();
  };
}

/**
 * Tenant isolation middleware - ensures user can only access their own tenant data
 */
export function requireTenantAccess(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  const requestedTenantId = req.params.tenantId || req.body?.tenantId || req.query.tenantId;

  if (!requestedTenantId) {
    return next();
  }

  if (req.user && req.user.tenantId !== requestedTenantId) {
    // Allow admin users to access any tenant
    if (!req.user.roles.includes('admin')) {
      return next(new AuthorizationError('Cannot access data from another tenant'));
    }
  }

  next();
}

/**
 * Admin-only middleware
 */
export function requireAdmin(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  if (!req.user) {
    return next(new AuthenticationError('User not authenticated'));
  }

  if (!req.user.roles.includes('admin')) {
    return next(new AuthorizationError('Admin access required'));
  }

  next();
}

/**
 * Internal service authentication (for service-to-service calls)
 */
export function authenticateInternal(
  req: AuthenticatedRequest,
  res: Response,
  next: NextFunction
): void {
  const internalKey = req.headers['x-internal-key'] as string;

  if (!internalKey || internalKey !== process.env.INTERNAL_SERVICE_KEY) {
    return next(new AuthenticationError('Invalid internal service key'));
  }

  // Set a system user context
  req.user = {
    userId: 'system',
    tenantId: req.headers['x-tenant-id'] as string || '',
    email: 'system@internal',
    roles: ['system'],
  };
  req.tenantId = req.user.tenantId;

  next();
}
