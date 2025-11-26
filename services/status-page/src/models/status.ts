/**
 * Status Page Models
 *
 * Models for service status, incidents, and maintenance windows.
 */

import { z } from 'zod';

// ===========================================
// Enums
// ===========================================

export enum ServiceStatus {
  OPERATIONAL = 'operational',
  DEGRADED = 'degraded',
  PARTIAL_OUTAGE = 'partial_outage',
  MAJOR_OUTAGE = 'major_outage',
  MAINTENANCE = 'maintenance',
}

export enum IncidentStatus {
  INVESTIGATING = 'investigating',
  IDENTIFIED = 'identified',
  MONITORING = 'monitoring',
  RESOLVED = 'resolved',
}

export enum IncidentSeverity {
  MINOR = 'minor',
  MAJOR = 'major',
  CRITICAL = 'critical',
}

export enum MaintenanceStatus {
  SCHEDULED = 'scheduled',
  IN_PROGRESS = 'in_progress',
  COMPLETED = 'completed',
  CANCELLED = 'cancelled',
}

// ===========================================
// Schemas
// ===========================================

export const ServiceSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  slug: z.string(),
  description: z.string().optional(),
  status: z.nativeEnum(ServiceStatus),
  group: z.string().optional(),
  order: z.number().default(0),
  isPublic: z.boolean().default(true),
  healthCheckUrl: z.string().url().optional(),
  healthCheckInterval: z.number().default(60),
  lastCheckAt: z.date().optional(),
  lastCheckLatency: z.number().optional(),
  uptimePercent: z.number().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const IncidentSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  description: z.string(),
  status: z.nativeEnum(IncidentStatus),
  severity: z.nativeEnum(IncidentSeverity),
  affectedServices: z.array(z.string().uuid()),
  createdAt: z.date(),
  updatedAt: z.date(),
  resolvedAt: z.date().optional(),
  postmortemUrl: z.string().url().optional(),
});

export const IncidentUpdateSchema = z.object({
  id: z.string().uuid(),
  incidentId: z.string().uuid(),
  status: z.nativeEnum(IncidentStatus),
  message: z.string(),
  createdAt: z.date(),
  createdBy: z.string(),
});

export const MaintenanceSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  description: z.string(),
  status: z.nativeEnum(MaintenanceStatus),
  affectedServices: z.array(z.string().uuid()),
  scheduledStartAt: z.date(),
  scheduledEndAt: z.date(),
  actualStartAt: z.date().optional(),
  actualEndAt: z.date().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export const UptimeEntrySchema = z.object({
  serviceId: z.string().uuid(),
  date: z.date(),
  uptimePercent: z.number(),
  totalChecks: z.number(),
  successfulChecks: z.number(),
  avgLatency: z.number(),
  incidents: z.number(),
});

// ===========================================
// Types
// ===========================================

export type Service = z.infer<typeof ServiceSchema>;
export type Incident = z.infer<typeof IncidentSchema>;
export type IncidentUpdate = z.infer<typeof IncidentUpdateSchema>;
export type Maintenance = z.infer<typeof MaintenanceSchema>;
export type UptimeEntry = z.infer<typeof UptimeEntrySchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateServiceInput {
  name: string;
  slug?: string;
  description?: string;
  group?: string;
  order?: number;
  isPublic?: boolean;
  healthCheckUrl?: string;
  healthCheckInterval?: number;
}

export interface CreateIncidentInput {
  title: string;
  description: string;
  severity: IncidentSeverity;
  affectedServices: string[];
}

export interface CreateMaintenanceInput {
  title: string;
  description: string;
  affectedServices: string[];
  scheduledStartAt: Date;
  scheduledEndAt: Date;
}

// ===========================================
// Status Summary Types
// ===========================================

export interface StatusSummary {
  overallStatus: ServiceStatus;
  services: Array<{
    id: string;
    name: string;
    slug: string;
    status: ServiceStatus;
    group?: string;
  }>;
  activeIncidents: Incident[];
  scheduledMaintenance: Maintenance[];
  uptime: {
    day: number;
    week: number;
    month: number;
  };
}

export interface ServiceMetrics {
  serviceId: string;
  uptimeHistory: Array<{
    date: string;
    uptimePercent: number;
  }>;
  incidentHistory: Array<{
    date: string;
    count: number;
  }>;
  latencyHistory: Array<{
    date: string;
    avgLatency: number;
    p95Latency: number;
    p99Latency: number;
  }>;
}

// ===========================================
// Status Display Helpers
// ===========================================

export const STATUS_DISPLAY: Record<ServiceStatus, { label: string; color: string; bgColor: string }> = {
  [ServiceStatus.OPERATIONAL]: { label: 'Operational', color: 'text-green-600', bgColor: 'bg-green-500' },
  [ServiceStatus.DEGRADED]: { label: 'Degraded', color: 'text-yellow-600', bgColor: 'bg-yellow-500' },
  [ServiceStatus.PARTIAL_OUTAGE]: { label: 'Partial Outage', color: 'text-orange-600', bgColor: 'bg-orange-500' },
  [ServiceStatus.MAJOR_OUTAGE]: { label: 'Major Outage', color: 'text-red-600', bgColor: 'bg-red-500' },
  [ServiceStatus.MAINTENANCE]: { label: 'Maintenance', color: 'text-blue-600', bgColor: 'bg-blue-500' },
};

export const INCIDENT_STATUS_DISPLAY: Record<IncidentStatus, { label: string; color: string }> = {
  [IncidentStatus.INVESTIGATING]: { label: 'Investigating', color: 'text-red-600' },
  [IncidentStatus.IDENTIFIED]: { label: 'Identified', color: 'text-orange-600' },
  [IncidentStatus.MONITORING]: { label: 'Monitoring', color: 'text-yellow-600' },
  [IncidentStatus.RESOLVED]: { label: 'Resolved', color: 'text-green-600' },
};

export const SEVERITY_DISPLAY: Record<IncidentSeverity, { label: string; color: string; bgColor: string }> = {
  [IncidentSeverity.MINOR]: { label: 'Minor', color: 'text-yellow-600', bgColor: 'bg-yellow-100' },
  [IncidentSeverity.MAJOR]: { label: 'Major', color: 'text-orange-600', bgColor: 'bg-orange-100' },
  [IncidentSeverity.CRITICAL]: { label: 'Critical', color: 'text-red-600', bgColor: 'bg-red-100' },
};
