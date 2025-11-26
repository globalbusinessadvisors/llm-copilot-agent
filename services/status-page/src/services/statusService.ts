/**
 * Status Service
 *
 * Handles service status, incidents, and uptime tracking.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import axios from 'axios';
import {
  Service,
  ServiceStatus,
  Incident,
  IncidentStatus,
  IncidentSeverity,
  IncidentUpdate,
  Maintenance,
  MaintenanceStatus,
  StatusSummary,
  CreateServiceInput,
  CreateIncidentInput,
  CreateMaintenanceInput,
  UptimeEntry,
} from '../models/status';

export class StatusService {
  private db: Pool;
  private redis: RedisClientType;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Service Management
  // ===========================================

  /**
   * Create a new service to monitor
   */
  async createService(input: CreateServiceInput): Promise<Service> {
    const slug = input.slug || input.name.toLowerCase().replace(/\s+/g, '-');

    const service: Service = {
      id: uuidv4(),
      name: input.name,
      slug,
      description: input.description,
      status: ServiceStatus.OPERATIONAL,
      group: input.group,
      order: input.order || 0,
      isPublic: input.isPublic ?? true,
      healthCheckUrl: input.healthCheckUrl,
      healthCheckInterval: input.healthCheckInterval || 60,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO services (
        id, name, slug, description, status, service_group, display_order,
        is_public, health_check_url, health_check_interval, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        service.id, service.name, service.slug, service.description,
        service.status, service.group, service.order, service.isPublic,
        service.healthCheckUrl, service.healthCheckInterval,
        service.createdAt, service.updatedAt,
      ]
    );

    return service;
  }

  /**
   * Get all public services
   */
  async getServices(): Promise<Service[]> {
    const result = await this.db.query(
      `SELECT * FROM services WHERE is_public = true ORDER BY display_order, name`
    );
    return result.rows.map(this.mapServiceRow);
  }

  /**
   * Update service status
   */
  async updateServiceStatus(serviceId: string, status: ServiceStatus): Promise<void> {
    await this.db.query(
      `UPDATE services SET status = $1, updated_at = NOW() WHERE id = $2`,
      [status, serviceId]
    );

    // Cache the status update
    await this.redis.set(`service:status:${serviceId}`, status, { EX: 300 });
  }

  /**
   * Perform health check on a service
   */
  async healthCheck(service: Service): Promise<{ success: boolean; latency: number }> {
    if (!service.healthCheckUrl) {
      return { success: true, latency: 0 };
    }

    const startTime = Date.now();
    try {
      const response = await axios.get(service.healthCheckUrl, {
        timeout: 10000,
        validateStatus: (status) => status < 500,
      });

      const latency = Date.now() - startTime;
      const success = response.status < 400;

      // Record the check
      await this.recordHealthCheck(service.id, success, latency);

      // Update service status based on result
      if (!success && service.status === ServiceStatus.OPERATIONAL) {
        await this.updateServiceStatus(service.id, ServiceStatus.DEGRADED);
      } else if (success && service.status !== ServiceStatus.OPERATIONAL && service.status !== ServiceStatus.MAINTENANCE) {
        await this.updateServiceStatus(service.id, ServiceStatus.OPERATIONAL);
      }

      return { success, latency };
    } catch (error) {
      const latency = Date.now() - startTime;
      await this.recordHealthCheck(service.id, false, latency);

      if (service.status === ServiceStatus.OPERATIONAL) {
        await this.updateServiceStatus(service.id, ServiceStatus.DEGRADED);
      }

      return { success: false, latency };
    }
  }

  private async recordHealthCheck(serviceId: string, success: boolean, latency: number): Promise<void> {
    await this.db.query(
      `INSERT INTO health_checks (id, service_id, success, latency, created_at)
       VALUES ($1, $2, $3, $4, NOW())`,
      [uuidv4(), serviceId, success, latency]
    );

    // Update service last check
    await this.db.query(
      `UPDATE services SET last_check_at = NOW(), last_check_latency = $1 WHERE id = $2`,
      [latency, serviceId]
    );
  }

  // ===========================================
  // Incident Management
  // ===========================================

  /**
   * Create a new incident
   */
  async createIncident(input: CreateIncidentInput): Promise<Incident> {
    const incident: Incident = {
      id: uuidv4(),
      title: input.title,
      description: input.description,
      status: IncidentStatus.INVESTIGATING,
      severity: input.severity,
      affectedServices: input.affectedServices,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO incidents (
        id, title, description, status, severity, affected_services, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        incident.id, incident.title, incident.description, incident.status,
        incident.severity, JSON.stringify(incident.affectedServices),
        incident.createdAt, incident.updatedAt,
      ]
    );

    // Update affected services status
    for (const serviceId of input.affectedServices) {
      const newStatus = input.severity === IncidentSeverity.CRITICAL
        ? ServiceStatus.MAJOR_OUTAGE
        : input.severity === IncidentSeverity.MAJOR
        ? ServiceStatus.PARTIAL_OUTAGE
        : ServiceStatus.DEGRADED;
      await this.updateServiceStatus(serviceId, newStatus);
    }

    // Create initial update
    await this.addIncidentUpdate(incident.id, {
      status: IncidentStatus.INVESTIGATING,
      message: 'We are investigating this issue.',
      createdBy: 'System',
    });

    // Clear cached status
    await this.redis.del('status:summary');

    return incident;
  }

  /**
   * Update incident status
   */
  async updateIncident(
    incidentId: string,
    status: IncidentStatus,
    message: string,
    createdBy: string
  ): Promise<Incident | null> {
    const updates: string[] = ['status = $1', 'updated_at = NOW()'];
    const values: unknown[] = [status];

    if (status === IncidentStatus.RESOLVED) {
      updates.push('resolved_at = NOW()');
    }

    await this.db.query(
      `UPDATE incidents SET ${updates.join(', ')} WHERE id = $${updates.length + 1}`,
      [...values, incidentId]
    );

    // Add update entry
    await this.addIncidentUpdate(incidentId, { status, message, createdBy });

    // If resolved, restore service status
    if (status === IncidentStatus.RESOLVED) {
      const incident = await this.getIncident(incidentId);
      if (incident) {
        for (const serviceId of incident.affectedServices) {
          await this.updateServiceStatus(serviceId, ServiceStatus.OPERATIONAL);
        }
      }
    }

    // Clear cached status
    await this.redis.del('status:summary');

    return this.getIncident(incidentId);
  }

  /**
   * Get incident by ID
   */
  async getIncident(incidentId: string): Promise<Incident | null> {
    const result = await this.db.query(
      `SELECT * FROM incidents WHERE id = $1`,
      [incidentId]
    );

    if (result.rows.length === 0) return null;

    return this.mapIncidentRow(result.rows[0]);
  }

  /**
   * Get active incidents
   */
  async getActiveIncidents(): Promise<Incident[]> {
    const result = await this.db.query(
      `SELECT * FROM incidents WHERE status != 'resolved' ORDER BY severity DESC, created_at DESC`
    );
    return result.rows.map(this.mapIncidentRow);
  }

  /**
   * Get incident history
   */
  async getIncidentHistory(limit: number = 20): Promise<Incident[]> {
    const result = await this.db.query(
      `SELECT * FROM incidents ORDER BY created_at DESC LIMIT $1`,
      [limit]
    );
    return result.rows.map(this.mapIncidentRow);
  }

  /**
   * Add incident update
   */
  async addIncidentUpdate(
    incidentId: string,
    update: { status: IncidentStatus; message: string; createdBy: string }
  ): Promise<IncidentUpdate> {
    const incidentUpdate: IncidentUpdate = {
      id: uuidv4(),
      incidentId,
      status: update.status,
      message: update.message,
      createdAt: new Date(),
      createdBy: update.createdBy,
    };

    await this.db.query(
      `INSERT INTO incident_updates (id, incident_id, status, message, created_at, created_by)
       VALUES ($1, $2, $3, $4, $5, $6)`,
      [
        incidentUpdate.id, incidentUpdate.incidentId, incidentUpdate.status,
        incidentUpdate.message, incidentUpdate.createdAt, incidentUpdate.createdBy,
      ]
    );

    return incidentUpdate;
  }

  /**
   * Get incident updates
   */
  async getIncidentUpdates(incidentId: string): Promise<IncidentUpdate[]> {
    const result = await this.db.query(
      `SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at DESC`,
      [incidentId]
    );

    return result.rows.map((row): IncidentUpdate => ({
      id: row.id,
      incidentId: row.incident_id,
      status: row.status,
      message: row.message,
      createdAt: row.created_at,
      createdBy: row.created_by,
    }));
  }

  // ===========================================
  // Maintenance Windows
  // ===========================================

  /**
   * Schedule maintenance
   */
  async scheduleMaintenance(input: CreateMaintenanceInput): Promise<Maintenance> {
    const maintenance: Maintenance = {
      id: uuidv4(),
      title: input.title,
      description: input.description,
      status: MaintenanceStatus.SCHEDULED,
      affectedServices: input.affectedServices,
      scheduledStartAt: input.scheduledStartAt,
      scheduledEndAt: input.scheduledEndAt,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO maintenance_windows (
        id, title, description, status, affected_services,
        scheduled_start_at, scheduled_end_at, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
      [
        maintenance.id, maintenance.title, maintenance.description,
        maintenance.status, JSON.stringify(maintenance.affectedServices),
        maintenance.scheduledStartAt, maintenance.scheduledEndAt,
        maintenance.createdAt, maintenance.updatedAt,
      ]
    );

    await this.redis.del('status:summary');

    return maintenance;
  }

  /**
   * Get scheduled maintenance
   */
  async getScheduledMaintenance(): Promise<Maintenance[]> {
    const result = await this.db.query(
      `SELECT * FROM maintenance_windows
       WHERE status IN ('scheduled', 'in_progress')
       AND scheduled_end_at > NOW()
       ORDER BY scheduled_start_at ASC`
    );
    return result.rows.map(this.mapMaintenanceRow);
  }

  // ===========================================
  // Status Summary
  // ===========================================

  /**
   * Get public status summary
   */
  async getStatusSummary(): Promise<StatusSummary> {
    // Try cache first
    const cached = await this.redis.get('status:summary');
    if (cached) {
      return JSON.parse(cached);
    }

    const services = await this.getServices();
    const activeIncidents = await this.getActiveIncidents();
    const scheduledMaintenance = await this.getScheduledMaintenance();

    // Calculate overall status
    let overallStatus = ServiceStatus.OPERATIONAL;
    for (const service of services) {
      if (service.status === ServiceStatus.MAJOR_OUTAGE) {
        overallStatus = ServiceStatus.MAJOR_OUTAGE;
        break;
      } else if (service.status === ServiceStatus.PARTIAL_OUTAGE && overallStatus !== ServiceStatus.MAJOR_OUTAGE) {
        overallStatus = ServiceStatus.PARTIAL_OUTAGE;
      } else if (service.status === ServiceStatus.DEGRADED && overallStatus === ServiceStatus.OPERATIONAL) {
        overallStatus = ServiceStatus.DEGRADED;
      }
    }

    // Calculate uptime percentages
    const uptime = await this.calculateUptime();

    const summary: StatusSummary = {
      overallStatus,
      services: services.map(s => ({
        id: s.id,
        name: s.name,
        slug: s.slug,
        status: s.status,
        group: s.group,
      })),
      activeIncidents,
      scheduledMaintenance,
      uptime,
    };

    // Cache for 1 minute
    await this.redis.set('status:summary', JSON.stringify(summary), { EX: 60 });

    return summary;
  }

  private async calculateUptime(): Promise<{ day: number; week: number; month: number }> {
    const result = await this.db.query(`
      SELECT
        AVG(CASE WHEN created_at > NOW() - INTERVAL '1 day' AND success THEN 100.0 ELSE 0.0 END) as day_uptime,
        AVG(CASE WHEN created_at > NOW() - INTERVAL '7 days' AND success THEN 100.0 ELSE 0.0 END) as week_uptime,
        AVG(CASE WHEN created_at > NOW() - INTERVAL '30 days' AND success THEN 100.0 ELSE 0.0 END) as month_uptime
      FROM health_checks
    `);

    const row = result.rows[0];
    return {
      day: parseFloat(row.day_uptime) || 100,
      week: parseFloat(row.week_uptime) || 100,
      month: parseFloat(row.month_uptime) || 100,
    };
  }

  // ===========================================
  // Private Helpers
  // ===========================================

  private mapServiceRow(row: any): Service {
    return {
      id: row.id,
      name: row.name,
      slug: row.slug,
      description: row.description,
      status: row.status,
      group: row.service_group,
      order: row.display_order,
      isPublic: row.is_public,
      healthCheckUrl: row.health_check_url,
      healthCheckInterval: row.health_check_interval,
      lastCheckAt: row.last_check_at,
      lastCheckLatency: row.last_check_latency,
      uptimePercent: row.uptime_percent,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private mapIncidentRow(row: any): Incident {
    return {
      id: row.id,
      title: row.title,
      description: row.description,
      status: row.status,
      severity: row.severity,
      affectedServices: row.affected_services,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      resolvedAt: row.resolved_at,
      postmortemUrl: row.postmortem_url,
    };
  }

  private mapMaintenanceRow(row: any): Maintenance {
    return {
      id: row.id,
      title: row.title,
      description: row.description,
      status: row.status,
      affectedServices: row.affected_services,
      scheduledStartAt: row.scheduled_start_at,
      scheduledEndAt: row.scheduled_end_at,
      actualStartAt: row.actual_start_at,
      actualEndAt: row.actual_end_at,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
