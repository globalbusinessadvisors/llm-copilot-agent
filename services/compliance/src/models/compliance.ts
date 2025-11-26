/**
 * Compliance Models
 *
 * Type definitions for compliance framework, SOC 2, HIPAA, and data residency.
 */

import { z } from 'zod';

// ===========================================
// Compliance Framework Types
// ===========================================

export enum ComplianceFramework {
  SOC2_TYPE1 = 'soc2_type1',
  SOC2_TYPE2 = 'soc2_type2',
  HIPAA = 'hipaa',
  GDPR = 'gdpr',
  CCPA = 'ccpa',
  ISO27001 = 'iso27001',
  PCI_DSS = 'pci_dss',
  FEDRAMP = 'fedramp',
}

export enum ControlCategory {
  SECURITY = 'security',
  AVAILABILITY = 'availability',
  PROCESSING_INTEGRITY = 'processing_integrity',
  CONFIDENTIALITY = 'confidentiality',
  PRIVACY = 'privacy',
  ACCESS_CONTROL = 'access_control',
  RISK_MANAGEMENT = 'risk_management',
  INCIDENT_RESPONSE = 'incident_response',
  BUSINESS_CONTINUITY = 'business_continuity',
  VENDOR_MANAGEMENT = 'vendor_management',
}

export enum ControlStatus {
  NOT_IMPLEMENTED = 'not_implemented',
  PARTIALLY_IMPLEMENTED = 'partially_implemented',
  IMPLEMENTED = 'implemented',
  EFFECTIVE = 'effective',
  NEEDS_IMPROVEMENT = 'needs_improvement',
}

export enum EvidenceType {
  POLICY = 'policy',
  PROCEDURE = 'procedure',
  SCREENSHOT = 'screenshot',
  LOG = 'log',
  REPORT = 'report',
  CONFIGURATION = 'configuration',
  ATTESTATION = 'attestation',
  TEST_RESULT = 'test_result',
  TRAINING_RECORD = 'training_record',
  ACCESS_REVIEW = 'access_review',
}

export enum AuditStatus {
  SCHEDULED = 'scheduled',
  IN_PROGRESS = 'in_progress',
  PENDING_REVIEW = 'pending_review',
  COMPLETED = 'completed',
  FAILED = 'failed',
}

export enum FindingSeverity {
  CRITICAL = 'critical',
  HIGH = 'high',
  MEDIUM = 'medium',
  LOW = 'low',
  INFORMATIONAL = 'informational',
}

export enum FindingStatus {
  OPEN = 'open',
  IN_PROGRESS = 'in_progress',
  REMEDIATED = 'remediated',
  ACCEPTED = 'accepted',
  CLOSED = 'closed',
}

// ===========================================
// Control Schemas
// ===========================================

export const ControlSchema = z.object({
  id: z.string().uuid(),
  framework: z.nativeEnum(ComplianceFramework),
  controlId: z.string(), // e.g., "CC1.1" for SOC 2
  name: z.string(),
  description: z.string(),
  category: z.nativeEnum(ControlCategory),
  status: z.nativeEnum(ControlStatus),
  owner: z.string(),
  implementation: z.object({
    description: z.string().optional(),
    automationLevel: z.enum(['manual', 'semi_automated', 'fully_automated']),
    tools: z.array(z.string()).optional(),
    procedures: z.array(z.string()).optional(),
  }),
  testing: z.object({
    frequency: z.enum(['continuous', 'daily', 'weekly', 'monthly', 'quarterly', 'annually']),
    lastTested: z.date().optional(),
    nextTest: z.date().optional(),
    testProcedure: z.string().optional(),
  }),
  evidence: z.array(z.object({
    type: z.nativeEnum(EvidenceType),
    name: z.string(),
    description: z.string().optional(),
    url: z.string().optional(),
    collectedAt: z.date(),
    validUntil: z.date().optional(),
  })).optional(),
  relatedControls: z.array(z.string()).optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export type Control = z.infer<typeof ControlSchema>;

// ===========================================
// Audit Schemas
// ===========================================

export const AuditSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  framework: z.nativeEnum(ComplianceFramework),
  type: z.enum(['internal', 'external', 'self_assessment']),
  status: z.nativeEnum(AuditStatus),
  scope: z.object({
    controls: z.array(z.string()),
    systems: z.array(z.string()).optional(),
    departments: z.array(z.string()).optional(),
    dateRange: z.object({
      start: z.date(),
      end: z.date(),
    }),
  }),
  auditor: z.object({
    name: z.string(),
    organization: z.string().optional(),
    email: z.string().email().optional(),
  }),
  schedule: z.object({
    plannedStart: z.date(),
    plannedEnd: z.date(),
    actualStart: z.date().optional(),
    actualEnd: z.date().optional(),
  }),
  findings: z.array(z.string()).optional(), // Finding IDs
  report: z.object({
    summary: z.string().optional(),
    opinion: z.enum(['unqualified', 'qualified', 'adverse', 'disclaimer']).optional(),
    reportUrl: z.string().optional(),
    issuedAt: z.date().optional(),
  }).optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export type Audit = z.infer<typeof AuditSchema>;

// ===========================================
// Finding Schemas
// ===========================================

export const FindingSchema = z.object({
  id: z.string().uuid(),
  auditId: z.string().uuid(),
  controlId: z.string().optional(),
  title: z.string(),
  description: z.string(),
  severity: z.nativeEnum(FindingSeverity),
  status: z.nativeEnum(FindingStatus),
  risk: z.object({
    likelihood: z.enum(['rare', 'unlikely', 'possible', 'likely', 'almost_certain']),
    impact: z.enum(['negligible', 'minor', 'moderate', 'major', 'catastrophic']),
    score: z.number().min(0).max(25).optional(),
  }),
  remediation: z.object({
    plan: z.string().optional(),
    owner: z.string(),
    dueDate: z.date(),
    completedDate: z.date().optional(),
    verifiedBy: z.string().optional(),
    verifiedDate: z.date().optional(),
  }),
  evidence: z.array(z.object({
    type: z.nativeEnum(EvidenceType),
    name: z.string(),
    url: z.string().optional(),
    collectedAt: z.date(),
  })).optional(),
  comments: z.array(z.object({
    author: z.string(),
    content: z.string(),
    createdAt: z.date(),
  })).optional(),
  metadata: z.record(z.unknown()).optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

export type Finding = z.infer<typeof FindingSchema>;

// ===========================================
// Data Residency Types
// ===========================================

export enum DataClassification {
  PUBLIC = 'public',
  INTERNAL = 'internal',
  CONFIDENTIAL = 'confidential',
  RESTRICTED = 'restricted',
  PHI = 'phi', // Protected Health Information
  PII = 'pii', // Personally Identifiable Information
  PCI = 'pci', // Payment Card Industry
}

export enum DataRegion {
  US_EAST = 'us-east',
  US_WEST = 'us-west',
  EU_WEST = 'eu-west',
  EU_CENTRAL = 'eu-central',
  APAC_SOUTH = 'apac-south',
  APAC_EAST = 'apac-east',
  GLOBAL = 'global',
}

export const DataResidencyPolicySchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  description: z.string().optional(),
  classification: z.nativeEnum(DataClassification),
  allowedRegions: z.array(z.nativeEnum(DataRegion)),
  restrictedRegions: z.array(z.nativeEnum(DataRegion)).optional(),
  requirements: z.object({
    encryption: z.object({
      atRest: z.boolean(),
      inTransit: z.boolean(),
      algorithm: z.string().optional(),
    }),
    retention: z.object({
      minDays: z.number().optional(),
      maxDays: z.number().optional(),
      deletionMethod: z.enum(['soft', 'hard', 'crypto_shred']).optional(),
    }),
    access: z.object({
      requireMFA: z.boolean().optional(),
      allowedRoles: z.array(z.string()).optional(),
      auditAccess: z.boolean().optional(),
    }),
    transfer: z.object({
      allowCrossBorder: z.boolean(),
      requireDPA: z.boolean().optional(), // Data Processing Agreement
      allowedMechanisms: z.array(z.string()).optional(),
    }),
  }),
  applicableTo: z.object({
    dataTypes: z.array(z.string()).optional(),
    systems: z.array(z.string()).optional(),
    departments: z.array(z.string()).optional(),
  }).optional(),
  status: z.enum(['draft', 'active', 'archived']),
  effectiveDate: z.date().optional(),
  expirationDate: z.date().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export type DataResidencyPolicy = z.infer<typeof DataResidencyPolicySchema>;

// ===========================================
// HIPAA Specific Types
// ===========================================

export enum HIPAASafeguard {
  ADMINISTRATIVE = 'administrative',
  PHYSICAL = 'physical',
  TECHNICAL = 'technical',
}

export const PHIAccessLogSchema = z.object({
  id: z.string().uuid(),
  userId: z.string(),
  patientId: z.string().optional(), // Pseudonymized
  accessType: z.enum(['view', 'create', 'update', 'delete', 'export', 'print']),
  resourceType: z.string(),
  resourceId: z.string(),
  purpose: z.string().optional(),
  accessGranted: z.boolean(),
  ipAddress: z.string().optional(),
  userAgent: z.string().optional(),
  timestamp: z.date(),
  metadata: z.record(z.unknown()).optional(),
});

export type PHIAccessLog = z.infer<typeof PHIAccessLogSchema>;

export const BusinessAssociateAgreementSchema = z.object({
  id: z.string().uuid(),
  vendorId: z.string(),
  vendorName: z.string(),
  agreementType: z.enum(['baa', 'dpa', 'both']),
  status: z.enum(['pending', 'active', 'expired', 'terminated']),
  effectiveDate: z.date(),
  expirationDate: z.date().optional(),
  autoRenew: z.boolean(),
  terms: z.object({
    permittedUses: z.array(z.string()),
    subcontractorAllowed: z.boolean(),
    breachNotificationHours: z.number(),
    terminationConditions: z.array(z.string()).optional(),
  }),
  contacts: z.array(z.object({
    name: z.string(),
    email: z.string().email(),
    role: z.string(),
    isPrimary: z.boolean(),
  })),
  documents: z.array(z.object({
    name: z.string(),
    url: z.string(),
    uploadedAt: z.date(),
  })).optional(),
  lastReviewed: z.date().optional(),
  nextReview: z.date().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
  createdBy: z.string(),
});

export type BusinessAssociateAgreement = z.infer<typeof BusinessAssociateAgreementSchema>;

// ===========================================
// Compliance Report Types
// ===========================================

export const ComplianceReportSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  framework: z.nativeEnum(ComplianceFramework),
  reportType: z.enum(['status', 'gap_analysis', 'risk_assessment', 'audit_readiness']),
  period: z.object({
    start: z.date(),
    end: z.date(),
  }),
  summary: z.object({
    totalControls: z.number(),
    implementedControls: z.number(),
    effectiveControls: z.number(),
    openFindings: z.number(),
    overallScore: z.number().min(0).max(100).optional(),
  }),
  sections: z.array(z.object({
    category: z.nativeEnum(ControlCategory),
    controls: z.array(z.object({
      controlId: z.string(),
      name: z.string(),
      status: z.nativeEnum(ControlStatus),
      findings: z.array(z.object({
        id: z.string(),
        severity: z.nativeEnum(FindingSeverity),
        status: z.nativeEnum(FindingStatus),
      })).optional(),
    })),
    score: z.number().min(0).max(100).optional(),
  })),
  recommendations: z.array(z.object({
    priority: z.enum(['critical', 'high', 'medium', 'low']),
    description: z.string(),
    impact: z.string().optional(),
  })).optional(),
  generatedAt: z.date(),
  generatedBy: z.string(),
  format: z.enum(['json', 'pdf', 'html']),
  url: z.string().optional(),
});

export type ComplianceReport = z.infer<typeof ComplianceReportSchema>;

// ===========================================
// Input Types
// ===========================================

export interface CreateControlInput {
  framework: ComplianceFramework;
  controlId: string;
  name: string;
  description: string;
  category: ControlCategory;
  owner: string;
  implementation?: Control['implementation'];
  testing?: Partial<Control['testing']>;
  relatedControls?: string[];
  metadata?: Record<string, unknown>;
}

export interface CreateAuditInput {
  name: string;
  framework: ComplianceFramework;
  type: Audit['type'];
  scope: Audit['scope'];
  auditor: Audit['auditor'];
  schedule: Audit['schedule'];
  metadata?: Record<string, unknown>;
}

export interface CreateFindingInput {
  auditId: string;
  controlId?: string;
  title: string;
  description: string;
  severity: FindingSeverity;
  risk: Finding['risk'];
  remediation: Omit<Finding['remediation'], 'completedDate' | 'verifiedBy' | 'verifiedDate'>;
  evidence?: Finding['evidence'];
  metadata?: Record<string, unknown>;
}

export interface CreateDataResidencyPolicyInput {
  name: string;
  description?: string;
  classification: DataClassification;
  allowedRegions: DataRegion[];
  restrictedRegions?: DataRegion[];
  requirements: DataResidencyPolicy['requirements'];
  applicableTo?: DataResidencyPolicy['applicableTo'];
  effectiveDate?: Date;
  expirationDate?: Date;
}

export interface CreateBAAInput {
  vendorId: string;
  vendorName: string;
  agreementType: BusinessAssociateAgreement['agreementType'];
  effectiveDate: Date;
  expirationDate?: Date;
  autoRenew: boolean;
  terms: BusinessAssociateAgreement['terms'];
  contacts: BusinessAssociateAgreement['contacts'];
}

export interface GenerateReportInput {
  framework: ComplianceFramework;
  reportType: ComplianceReport['reportType'];
  period: ComplianceReport['period'];
  format: ComplianceReport['format'];
  includeRecommendations?: boolean;
}
