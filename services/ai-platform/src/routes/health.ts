/**
 * Health Check Routes
 *
 * API endpoints for service health monitoring.
 */

import { Router, Request, Response } from 'express';
import { Pool } from 'pg';
import { RedisClientType } from 'redis';

export function createHealthRoutes(db: Pool, redis: RedisClientType): Router {
  const router = Router();

  /**
   * Basic health check
   */
  router.get('/', async (_req: Request, res: Response) => {
    res.json({
      status: 'healthy',
      service: 'ai-platform',
      timestamp: new Date().toISOString(),
    });
  });

  /**
   * Detailed health check
   */
  router.get('/detailed', async (_req: Request, res: Response) => {
    const checks: Record<string, { status: string; latency?: number; error?: string }> = {};

    // Check database
    try {
      const startDb = Date.now();
      await db.query('SELECT 1');
      checks.database = {
        status: 'healthy',
        latency: Date.now() - startDb,
      };
    } catch (error) {
      checks.database = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check Redis
    try {
      const startRedis = Date.now();
      await redis.ping();
      checks.redis = {
        status: 'healthy',
        latency: Date.now() - startRedis,
      };
    } catch (error) {
      checks.redis = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Check OpenAI (if configured)
    if (process.env.OPENAI_API_KEY) {
      checks.openai = { status: 'configured' };
    } else {
      checks.openai = { status: 'not_configured' };
    }

    // Check Anthropic (if configured)
    if (process.env.ANTHROPIC_API_KEY) {
      checks.anthropic = { status: 'configured' };
    } else {
      checks.anthropic = { status: 'not_configured' };
    }

    const overallStatus = Object.values(checks).every(
      c => c.status === 'healthy' || c.status === 'configured' || c.status === 'not_configured'
    ) ? 'healthy' : 'degraded';

    res.status(overallStatus === 'healthy' ? 200 : 503).json({
      status: overallStatus,
      service: 'ai-platform',
      version: process.env.npm_package_version || '1.0.0',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  /**
   * Readiness check
   */
  router.get('/ready', async (_req: Request, res: Response) => {
    try {
      // Check database connection
      await db.query('SELECT 1');

      // Check Redis connection
      await redis.ping();

      res.json({
        ready: true,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      res.status(503).json({
        ready: false,
        error: error instanceof Error ? error.message : 'Unknown error',
        timestamp: new Date().toISOString(),
      });
    }
  });

  /**
   * Liveness check
   */
  router.get('/live', (_req: Request, res: Response) => {
    res.json({
      live: true,
      timestamp: new Date().toISOString(),
    });
  });

  return router;
}
