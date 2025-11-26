/**
 * Health Check Routes
 *
 * Provides health and readiness endpoints for orchestration systems.
 */

import { Router, Request, Response } from 'express';
import { Pool } from 'pg';
import { RedisClientType } from 'redis';

export function createHealthRouter(db: Pool, redis: RedisClientType): Router {
  const router = Router();

  /**
   * Basic health check
   * GET /health
   */
  router.get('/health', (_req: Request, res: Response) => {
    res.json({
      status: 'ok',
      service: 'billing',
      timestamp: new Date().toISOString(),
    });
  });

  /**
   * Readiness check - verifies all dependencies
   * GET /ready
   */
  router.get('/ready', async (_req: Request, res: Response) => {
    const checks: Record<string, { status: string; latency?: number; error?: string }> = {};

    // Check PostgreSQL
    const dbStart = Date.now();
    try {
      await db.query('SELECT 1');
      checks.database = { status: 'ok', latency: Date.now() - dbStart };
    } catch (error) {
      checks.database = {
        status: 'error',
        latency: Date.now() - dbStart,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check Redis
    const redisStart = Date.now();
    try {
      await redis.ping();
      checks.redis = { status: 'ok', latency: Date.now() - redisStart };
    } catch (error) {
      checks.redis = {
        status: 'error',
        latency: Date.now() - redisStart,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Determine overall status
    const allHealthy = Object.values(checks).every((c) => c.status === 'ok');

    res.status(allHealthy ? 200 : 503).json({
      status: allHealthy ? 'ready' : 'not_ready',
      service: 'billing',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  /**
   * Liveness check - basic process health
   * GET /live
   */
  router.get('/live', (_req: Request, res: Response) => {
    res.json({
      status: 'alive',
      service: 'billing',
      uptime: process.uptime(),
      memory: process.memoryUsage(),
    });
  });

  /**
   * Version info
   * GET /version
   */
  router.get('/version', (_req: Request, res: Response) => {
    res.json({
      service: 'billing',
      version: process.env.npm_package_version || '1.0.0',
      nodeVersion: process.version,
      environment: process.env.NODE_ENV || 'development',
    });
  });

  return router;
}
