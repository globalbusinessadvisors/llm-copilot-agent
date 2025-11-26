/**
 * Health Check Routes
 *
 * Endpoints for service health monitoring.
 */

import { Router, Request, Response } from 'express';
import { Pool } from 'pg';
import { RedisClientType } from 'redis';

const router = Router();

interface HealthCheckDependencies {
  db: Pool;
  redis: RedisClientType;
}

export function createHealthRoutes(deps: HealthCheckDependencies): Router {
  const { db, redis } = deps;

  /**
   * Basic health check
   * GET /health
   */
  router.get('/', (req: Request, res: Response) => {
    res.json({
      status: 'healthy',
      service: 'support-service',
      timestamp: new Date().toISOString(),
    });
  });

  /**
   * Liveness probe
   * GET /health/live
   */
  router.get('/live', (req: Request, res: Response) => {
    res.json({
      status: 'live',
      timestamp: new Date().toISOString(),
    });
  });

  /**
   * Readiness probe with dependency checks
   * GET /health/ready
   */
  router.get('/ready', async (req: Request, res: Response) => {
    const checks: Record<string, { status: string; latency?: number; error?: string }> = {};
    let isReady = true;

    // Check PostgreSQL
    const dbStart = Date.now();
    try {
      await db.query('SELECT 1');
      checks.database = {
        status: 'healthy',
        latency: Date.now() - dbStart,
      };
    } catch (error) {
      isReady = false;
      checks.database = {
        status: 'unhealthy',
        latency: Date.now() - dbStart,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check Redis
    const redisStart = Date.now();
    try {
      await redis.ping();
      checks.redis = {
        status: 'healthy',
        latency: Date.now() - redisStart,
      };
    } catch (error) {
      isReady = false;
      checks.redis = {
        status: 'unhealthy',
        latency: Date.now() - redisStart,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    const statusCode = isReady ? 200 : 503;

    res.status(statusCode).json({
      status: isReady ? 'ready' : 'not_ready',
      service: 'support-service',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  return router;
}
