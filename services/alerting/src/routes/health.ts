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
      service: 'alerting-service',
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
      service: 'alerting-service',
      timestamp: new Date().toISOString(),
      checks,
    });
  });

  /**
   * Detailed health check with metrics
   * GET /health/detailed
   */
  router.get('/detailed', async (req: Request, res: Response) => {
    const checks: Record<string, any> = {};

    // Database health and stats
    try {
      const poolStats = {
        totalCount: db.totalCount,
        idleCount: db.idleCount,
        waitingCount: db.waitingCount,
      };

      const dbResult = await db.query(`
        SELECT
          (SELECT COUNT(*) FROM alerts WHERE status = 'triggered') as active_alerts,
          (SELECT COUNT(*) FROM alert_rules WHERE enabled = true) as active_rules,
          (SELECT COUNT(*) FROM escalation_policies) as escalation_policies,
          (SELECT COUNT(*) FROM on_call_schedules) as on_call_schedules
      `);

      checks.database = {
        status: 'healthy',
        pool: poolStats,
        stats: {
          activeAlerts: parseInt(dbResult.rows[0].active_alerts, 10),
          activeRules: parseInt(dbResult.rows[0].active_rules, 10),
          escalationPolicies: parseInt(dbResult.rows[0].escalation_policies, 10),
          onCallSchedules: parseInt(dbResult.rows[0].on_call_schedules, 10),
        },
      };
    } catch (error) {
      checks.database = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Redis health
    try {
      const info = await redis.info('memory');
      const memoryMatch = info.match(/used_memory_human:(\S+)/);

      checks.redis = {
        status: 'healthy',
        memoryUsed: memoryMatch ? memoryMatch[1] : 'unknown',
      };
    } catch (error) {
      checks.redis = {
        status: 'unhealthy',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }

    // Process stats
    const processStats = {
      uptime: process.uptime(),
      memoryUsage: process.memoryUsage(),
      cpuUsage: process.cpuUsage(),
      nodeVersion: process.version,
    };

    res.json({
      status: 'healthy',
      service: 'alerting-service',
      version: process.env.npm_package_version || '1.0.0',
      timestamp: new Date().toISOString(),
      checks,
      process: processStats,
    });
  });

  return router;
}
