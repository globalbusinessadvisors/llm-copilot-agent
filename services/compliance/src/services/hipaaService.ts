/**
 * HIPAA Compliance Service
 *
 * Manages HIPAA-specific compliance requirements including PHI access logging,
 * Business Associate Agreements, and safeguard monitoring.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import crypto from 'crypto';
import {
  PHIAccessLog,
  BusinessAssociateAgreement,
  HIPAASafeguard,
  CreateBAAInput,
} from '../models/compliance';

interface PHIAccessLogInput {
  userId: string;
  patientId?: string;
  accessType: PHIAccessLog['accessType'];
  resourceType: string;
  resourceId: string;
  purpose?: string;
  accessGranted: boolean;
  ipAddress?: string;
  userAgent?: string;
  metadata?: Record<string, unknown>;
}

interface HIPAAControlRequirement {
  id: string;
  safeguard: HIPAASafeguard;
  standard: string;
  specification: string;
  required: boolean;
  description: string;
}

export class HIPAAService {
  private db: Pool;
  private redis: RedisClientType;
  private encryptionKey: Buffer;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;

    // Initialize encryption key for PHI pseudonymization
    const key = process.env.HIPAA_ENCRYPTION_KEY || 'default-key-change-in-production';
    this.encryptionKey = crypto.scryptSync(key, 'salt', 32);
  }

  // ===========================================
  // PHI Access Logging
  // ===========================================

  /**
   * Log PHI access event
   */
  async logPHIAccess(input: PHIAccessLogInput): Promise<PHIAccessLog> {
    // Pseudonymize patient ID if present
    const pseudonymizedPatientId = input.patientId
      ? this.pseudonymize(input.patientId)
      : undefined;

    const log: PHIAccessLog = {
      id: uuidv4(),
      userId: input.userId,
      patientId: pseudonymizedPatientId,
      accessType: input.accessType,
      resourceType: input.resourceType,
      resourceId: input.resourceId,
      purpose: input.purpose,
      accessGranted: input.accessGranted,
      ipAddress: input.ipAddress,
      userAgent: input.userAgent,
      timestamp: new Date(),
      metadata: input.metadata,
    };

    await this.db.query(
      `INSERT INTO phi_access_logs (
        id, user_id, patient_id, access_type, resource_type, resource_id,
        purpose, access_granted, ip_address, user_agent, timestamp, metadata
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        log.id, log.userId, log.patientId, log.accessType, log.resourceType,
        log.resourceId, log.purpose, log.accessGranted, log.ipAddress,
        log.userAgent, log.timestamp, JSON.stringify(log.metadata),
      ]
    );

    // Alert on suspicious access patterns
    await this.checkAccessPatterns(input);

    return log;
  }

  /**
   * Get PHI access logs
   */
  async getPHIAccessLogs(filters: {
    userId?: string;
    patientId?: string;
    accessType?: PHIAccessLog['accessType'];
    startDate?: Date;
    endDate?: Date;
    limit?: number;
  }): Promise<PHIAccessLog[]> {
    let query = `SELECT * FROM phi_access_logs WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters.userId) {
      query += ` AND user_id = $${paramIndex++}`;
      values.push(filters.userId);
    }
    if (filters.patientId) {
      // Pseudonymize for lookup
      query += ` AND patient_id = $${paramIndex++}`;
      values.push(this.pseudonymize(filters.patientId));
    }
    if (filters.accessType) {
      query += ` AND access_type = $${paramIndex++}`;
      values.push(filters.accessType);
    }
    if (filters.startDate) {
      query += ` AND timestamp >= $${paramIndex++}`;
      values.push(filters.startDate);
    }
    if (filters.endDate) {
      query += ` AND timestamp <= $${paramIndex++}`;
      values.push(filters.endDate);
    }

    query += ` ORDER BY timestamp DESC LIMIT $${paramIndex}`;
    values.push(filters.limit || 100);

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapPHIAccessLogRow);
  }

  /**
   * Generate PHI access report for auditing
   */
  async generateAccessReport(options: {
    startDate: Date;
    endDate: Date;
    patientId?: string;
    userId?: string;
  }): Promise<{
    summary: {
      totalAccesses: number;
      uniqueUsers: number;
      uniquePatients: number;
      accessesByType: Record<string, number>;
      deniedAccesses: number;
    };
    details: PHIAccessLog[];
  }> {
    const logs = await this.getPHIAccessLogs({
      startDate: options.startDate,
      endDate: options.endDate,
      patientId: options.patientId,
      userId: options.userId,
      limit: 10000,
    });

    const uniqueUsers = new Set(logs.map(l => l.userId));
    const uniquePatients = new Set(logs.filter(l => l.patientId).map(l => l.patientId));
    const accessesByType: Record<string, number> = {};
    let deniedAccesses = 0;

    logs.forEach(log => {
      accessesByType[log.accessType] = (accessesByType[log.accessType] || 0) + 1;
      if (!log.accessGranted) deniedAccesses++;
    });

    return {
      summary: {
        totalAccesses: logs.length,
        uniqueUsers: uniqueUsers.size,
        uniquePatients: uniquePatients.size,
        accessesByType,
        deniedAccesses,
      },
      details: logs,
    };
  }

  /**
   * Check for suspicious access patterns
   */
  private async checkAccessPatterns(input: PHIAccessLogInput): Promise<void> {
    const cacheKey = `hipaa:access:${input.userId}:count`;
    const windowMinutes = 15;

    // Increment access count
    const count = await this.redis.incr(cacheKey);
    if (count === 1) {
      await this.redis.expire(cacheKey, windowMinutes * 60);
    }

    // Alert if excessive access
    const threshold = 100;
    if (count > threshold) {
      await this.createAlert({
        type: 'excessive_phi_access',
        severity: 'high',
        userId: input.userId,
        message: `User ${input.userId} has accessed PHI ${count} times in ${windowMinutes} minutes`,
        metadata: { accessCount: count, windowMinutes },
      });
    }

    // Check for after-hours access
    const hour = new Date().getUTCHours();
    if (hour < 6 || hour > 22) {
      await this.createAlert({
        type: 'after_hours_phi_access',
        severity: 'medium',
        userId: input.userId,
        message: `After-hours PHI access by user ${input.userId}`,
        metadata: { hour, accessType: input.accessType },
      });
    }
  }

  // ===========================================
  // Business Associate Agreements
  // ===========================================

  /**
   * Create Business Associate Agreement
   */
  async createBAA(input: CreateBAAInput, userId: string): Promise<BusinessAssociateAgreement> {
    const baa: BusinessAssociateAgreement = {
      id: uuidv4(),
      vendorId: input.vendorId,
      vendorName: input.vendorName,
      agreementType: input.agreementType,
      status: 'active',
      effectiveDate: input.effectiveDate,
      expirationDate: input.expirationDate,
      autoRenew: input.autoRenew,
      terms: input.terms,
      contacts: input.contacts,
      documents: [],
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO business_associate_agreements (
        id, vendor_id, vendor_name, agreement_type, status, effective_date,
        expiration_date, auto_renew, terms, contacts, documents,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)`,
      [
        baa.id, baa.vendorId, baa.vendorName, baa.agreementType, baa.status,
        baa.effectiveDate, baa.expirationDate, baa.autoRenew,
        JSON.stringify(baa.terms), JSON.stringify(baa.contacts),
        JSON.stringify(baa.documents), baa.createdAt, baa.updatedAt, userId,
      ]
    );

    // Schedule expiration reminder
    if (baa.expirationDate) {
      await this.scheduleExpirationReminder(baa);
    }

    return baa;
  }

  /**
   * Get BAA by ID
   */
  async getBAA(baaId: string): Promise<BusinessAssociateAgreement | null> {
    const result = await this.db.query(
      `SELECT * FROM business_associate_agreements WHERE id = $1`,
      [baaId]
    );

    if (result.rows.length === 0) return null;

    return this.mapBAARow(result.rows[0]);
  }

  /**
   * List BAAs
   */
  async listBAAs(filters?: {
    status?: BusinessAssociateAgreement['status'];
    vendorId?: string;
    expiringWithinDays?: number;
  }): Promise<BusinessAssociateAgreement[]> {
    let query = `SELECT * FROM business_associate_agreements WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.vendorId) {
      query += ` AND vendor_id = $${paramIndex++}`;
      values.push(filters.vendorId);
    }
    if (filters?.expiringWithinDays) {
      const expirationDate = new Date();
      expirationDate.setDate(expirationDate.getDate() + filters.expiringWithinDays);
      query += ` AND expiration_date IS NOT NULL AND expiration_date <= $${paramIndex++}`;
      values.push(expirationDate);
    }

    query += ` ORDER BY expiration_date ASC NULLS LAST`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapBAARow);
  }

  /**
   * Update BAA status
   */
  async updateBAAStatus(
    baaId: string,
    status: BusinessAssociateAgreement['status']
  ): Promise<BusinessAssociateAgreement> {
    const baa = await this.getBAA(baaId);
    if (!baa) throw new Error('BAA not found');

    await this.db.query(
      `UPDATE business_associate_agreements SET status = $1, updated_at = NOW() WHERE id = $2`,
      [status, baaId]
    );

    return { ...baa, status, updatedAt: new Date() };
  }

  /**
   * Add document to BAA
   */
  async addBAADocument(
    baaId: string,
    document: { name: string; url: string }
  ): Promise<BusinessAssociateAgreement> {
    const baa = await this.getBAA(baaId);
    if (!baa) throw new Error('BAA not found');

    const newDocument = {
      ...document,
      uploadedAt: new Date(),
    };

    await this.db.query(
      `UPDATE business_associate_agreements
       SET documents = documents || $1::jsonb, updated_at = NOW()
       WHERE id = $2`,
      [JSON.stringify([newDocument]), baaId]
    );

    return {
      ...baa,
      documents: [...(baa.documents || []), newDocument],
      updatedAt: new Date(),
    };
  }

  /**
   * Schedule expiration reminder
   */
  private async scheduleExpirationReminder(baa: BusinessAssociateAgreement): Promise<void> {
    if (!baa.expirationDate) return;

    // Schedule reminder 90 days before expiration
    const reminderDate = new Date(baa.expirationDate);
    reminderDate.setDate(reminderDate.getDate() - 90);

    await this.db.query(
      `INSERT INTO scheduled_tasks (id, task_type, scheduled_for, payload, status)
       VALUES ($1, $2, $3, $4, 'pending')`,
      [
        uuidv4(),
        'baa_expiration_reminder',
        reminderDate,
        JSON.stringify({ baaId: baa.id, vendorName: baa.vendorName }),
      ]
    );
  }

  // ===========================================
  // HIPAA Safeguards
  // ===========================================

  /**
   * Get HIPAA control requirements
   */
  getHIPAAControlRequirements(): HIPAAControlRequirement[] {
    return [
      // Administrative Safeguards
      {
        id: '164.308(a)(1)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Management Process',
        specification: 'Risk Analysis',
        required: true,
        description: 'Conduct an accurate and thorough assessment of potential risks and vulnerabilities',
      },
      {
        id: '164.308(a)(1)(ii)(B)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Management Process',
        specification: 'Risk Management',
        required: true,
        description: 'Implement security measures sufficient to reduce risks and vulnerabilities',
      },
      {
        id: '164.308(a)(1)(ii)(C)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Management Process',
        specification: 'Sanction Policy',
        required: true,
        description: 'Apply appropriate sanctions against workforce members who fail to comply',
      },
      {
        id: '164.308(a)(1)(ii)(D)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Management Process',
        specification: 'Information System Activity Review',
        required: true,
        description: 'Regularly review records of information system activity',
      },
      {
        id: '164.308(a)(3)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Workforce Security',
        specification: 'Authorization and/or Supervision',
        required: false,
        description: 'Implement procedures for authorization and supervision of workforce members',
      },
      {
        id: '164.308(a)(4)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Information Access Management',
        specification: 'Access Authorization',
        required: false,
        description: 'Implement policies for granting access to ePHI',
      },
      {
        id: '164.308(a)(5)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Awareness and Training',
        specification: 'Security Reminders',
        required: false,
        description: 'Periodic security updates',
      },
      {
        id: '164.308(a)(6)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Security Incident Procedures',
        specification: 'Response and Reporting',
        required: true,
        description: 'Identify and respond to suspected or known security incidents',
      },
      {
        id: '164.308(a)(7)',
        safeguard: HIPAASafeguard.ADMINISTRATIVE,
        standard: 'Contingency Plan',
        specification: 'Data Backup Plan',
        required: true,
        description: 'Establish procedures to create and maintain retrievable exact copies of ePHI',
      },
      // Physical Safeguards
      {
        id: '164.310(a)(1)',
        safeguard: HIPAASafeguard.PHYSICAL,
        standard: 'Facility Access Controls',
        specification: 'Contingency Operations',
        required: false,
        description: 'Establish procedures for facility access in support of restoration of lost data',
      },
      {
        id: '164.310(b)',
        safeguard: HIPAASafeguard.PHYSICAL,
        standard: 'Workstation Use',
        specification: 'Workstation Use',
        required: true,
        description: 'Implement policies for proper workstation use',
      },
      {
        id: '164.310(c)',
        safeguard: HIPAASafeguard.PHYSICAL,
        standard: 'Workstation Security',
        specification: 'Workstation Security',
        required: true,
        description: 'Implement physical safeguards for workstations',
      },
      {
        id: '164.310(d)(1)',
        safeguard: HIPAASafeguard.PHYSICAL,
        standard: 'Device and Media Controls',
        specification: 'Disposal',
        required: true,
        description: 'Implement policies for final disposal of ePHI and hardware',
      },
      // Technical Safeguards
      {
        id: '164.312(a)(1)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Access Control',
        specification: 'Unique User Identification',
        required: true,
        description: 'Assign a unique name and/or number for identifying and tracking user identity',
      },
      {
        id: '164.312(a)(2)(ii)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Access Control',
        specification: 'Emergency Access Procedure',
        required: true,
        description: 'Establish procedures for obtaining necessary ePHI during an emergency',
      },
      {
        id: '164.312(a)(2)(iii)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Access Control',
        specification: 'Automatic Logoff',
        required: false,
        description: 'Implement electronic procedures that terminate session after inactivity',
      },
      {
        id: '164.312(a)(2)(iv)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Access Control',
        specification: 'Encryption and Decryption',
        required: false,
        description: 'Implement a mechanism to encrypt and decrypt ePHI',
      },
      {
        id: '164.312(b)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Audit Controls',
        specification: 'Audit Controls',
        required: true,
        description: 'Implement hardware, software, and procedural mechanisms to record and examine activity',
      },
      {
        id: '164.312(c)(1)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Integrity',
        specification: 'Mechanism to Authenticate ePHI',
        required: false,
        description: 'Implement electronic mechanisms to corroborate that ePHI has not been altered',
      },
      {
        id: '164.312(d)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Person or Entity Authentication',
        specification: 'Person or Entity Authentication',
        required: true,
        description: 'Implement procedures to verify that a person or entity seeking access is the one claimed',
      },
      {
        id: '164.312(e)(1)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Transmission Security',
        specification: 'Integrity Controls',
        required: false,
        description: 'Implement security measures to ensure ePHI transmitted is not improperly modified',
      },
      {
        id: '164.312(e)(2)(ii)',
        safeguard: HIPAASafeguard.TECHNICAL,
        standard: 'Transmission Security',
        specification: 'Encryption',
        required: false,
        description: 'Implement a mechanism to encrypt ePHI whenever appropriate',
      },
    ];
  }

  /**
   * Assess HIPAA compliance status
   */
  async assessHIPAACompliance(): Promise<{
    overallScore: number;
    bySafeguard: Record<HIPAASafeguard, {
      implemented: number;
      total: number;
      score: number;
    }>;
    requiredControls: {
      implemented: number;
      total: number;
    };
    addressableControls: {
      implemented: number;
      total: number;
    };
    gaps: Array<{
      controlId: string;
      standard: string;
      specification: string;
      required: boolean;
    }>;
  }> {
    const requirements = this.getHIPAAControlRequirements();

    // Get implemented controls
    const result = await this.db.query(
      `SELECT control_id FROM compliance_controls WHERE framework = 'hipaa' AND status IN ('implemented', 'effective')`
    );
    const implementedIds = new Set(result.rows.map(r => r.control_id));

    // Calculate by safeguard
    const bySafeguard: Record<HIPAASafeguard, { implemented: number; total: number; score: number }> = {
      [HIPAASafeguard.ADMINISTRATIVE]: { implemented: 0, total: 0, score: 0 },
      [HIPAASafeguard.PHYSICAL]: { implemented: 0, total: 0, score: 0 },
      [HIPAASafeguard.TECHNICAL]: { implemented: 0, total: 0, score: 0 },
    };

    let requiredImplemented = 0;
    let requiredTotal = 0;
    let addressableImplemented = 0;
    let addressableTotal = 0;
    const gaps: Array<{
      controlId: string;
      standard: string;
      specification: string;
      required: boolean;
    }> = [];

    requirements.forEach(req => {
      bySafeguard[req.safeguard].total++;

      const isImplemented = implementedIds.has(req.id);
      if (isImplemented) {
        bySafeguard[req.safeguard].implemented++;
      } else {
        gaps.push({
          controlId: req.id,
          standard: req.standard,
          specification: req.specification,
          required: req.required,
        });
      }

      if (req.required) {
        requiredTotal++;
        if (isImplemented) requiredImplemented++;
      } else {
        addressableTotal++;
        if (isImplemented) addressableImplemented++;
      }
    });

    // Calculate scores
    Object.values(bySafeguard).forEach(safeguard => {
      safeguard.score = safeguard.total > 0
        ? Math.round((safeguard.implemented / safeguard.total) * 100)
        : 100;
    });

    const totalImplemented = requiredImplemented + addressableImplemented;
    const total = requiredTotal + addressableTotal;
    const overallScore = total > 0
      ? Math.round((totalImplemented / total) * 100)
      : 100;

    return {
      overallScore,
      bySafeguard,
      requiredControls: {
        implemented: requiredImplemented,
        total: requiredTotal,
      },
      addressableControls: {
        implemented: addressableImplemented,
        total: addressableTotal,
      },
      gaps: gaps.sort((a, b) => (a.required ? 0 : 1) - (b.required ? 0 : 1)),
    };
  }

  /**
   * Report a breach
   */
  async reportBreach(input: {
    discoveryDate: Date;
    affectedIndividuals: number;
    phiTypes: string[];
    description: string;
    containmentActions: string[];
    reportedBy: string;
  }): Promise<{
    id: string;
    notificationRequired: boolean;
    notificationDeadline: Date;
    hhsNotificationRequired: boolean;
  }> {
    const breachId = uuidv4();
    const notificationDeadline = new Date(input.discoveryDate);
    notificationDeadline.setDate(notificationDeadline.getDate() + 60);

    // HHS notification required for breaches affecting 500+ individuals
    const hhsNotificationRequired = input.affectedIndividuals >= 500;

    await this.db.query(
      `INSERT INTO hipaa_breaches (
        id, discovery_date, affected_individuals, phi_types, description,
        containment_actions, notification_deadline, hhs_notification_required,
        status, reported_by, created_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'investigating', $9, NOW())`,
      [
        breachId, input.discoveryDate, input.affectedIndividuals,
        input.phiTypes, input.description, input.containmentActions,
        notificationDeadline, hhsNotificationRequired, input.reportedBy,
      ]
    );

    // Create high-priority alert
    await this.createAlert({
      type: 'hipaa_breach_reported',
      severity: 'critical',
      userId: input.reportedBy,
      message: `HIPAA breach reported affecting ${input.affectedIndividuals} individuals`,
      metadata: { breachId, affectedIndividuals: input.affectedIndividuals },
    });

    return {
      id: breachId,
      notificationRequired: true,
      notificationDeadline,
      hhsNotificationRequired,
    };
  }

  // ===========================================
  // Helpers
  // ===========================================

  /**
   * Pseudonymize a value
   */
  private pseudonymize(value: string): string {
    const iv = crypto.randomBytes(16);
    const cipher = crypto.createCipheriv('aes-256-cbc', this.encryptionKey, iv);
    let encrypted = cipher.update(value, 'utf8', 'hex');
    encrypted += cipher.final('hex');
    return iv.toString('hex') + ':' + encrypted;
  }

  /**
   * Create security alert
   */
  private async createAlert(alert: {
    type: string;
    severity: string;
    userId: string;
    message: string;
    metadata?: Record<string, unknown>;
  }): Promise<void> {
    await this.db.query(
      `INSERT INTO security_alerts (id, type, severity, user_id, message, metadata, created_at)
       VALUES ($1, $2, $3, $4, $5, $6, NOW())`,
      [uuidv4(), alert.type, alert.severity, alert.userId, alert.message, JSON.stringify(alert.metadata)]
    );

    // For critical alerts, also publish to Redis for real-time notification
    if (alert.severity === 'critical' || alert.severity === 'high') {
      await this.redis.publish('security-alerts', JSON.stringify(alert));
    }
  }

  private mapPHIAccessLogRow(row: Record<string, unknown>): PHIAccessLog {
    return {
      id: row.id as string,
      userId: row.user_id as string,
      patientId: row.patient_id as string | undefined,
      accessType: row.access_type as PHIAccessLog['accessType'],
      resourceType: row.resource_type as string,
      resourceId: row.resource_id as string,
      purpose: row.purpose as string | undefined,
      accessGranted: row.access_granted as boolean,
      ipAddress: row.ip_address as string | undefined,
      userAgent: row.user_agent as string | undefined,
      timestamp: row.timestamp as Date,
      metadata: row.metadata as Record<string, unknown>,
    };
  }

  private mapBAARow(row: Record<string, unknown>): BusinessAssociateAgreement {
    return {
      id: row.id as string,
      vendorId: row.vendor_id as string,
      vendorName: row.vendor_name as string,
      agreementType: row.agreement_type as BusinessAssociateAgreement['agreementType'],
      status: row.status as BusinessAssociateAgreement['status'],
      effectiveDate: row.effective_date as Date,
      expirationDate: row.expiration_date as Date | undefined,
      autoRenew: row.auto_renew as boolean,
      terms: row.terms as BusinessAssociateAgreement['terms'],
      contacts: row.contacts as BusinessAssociateAgreement['contacts'],
      documents: row.documents as BusinessAssociateAgreement['documents'],
      lastReviewed: row.last_reviewed as Date | undefined,
      nextReview: row.next_review as Date | undefined,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }
}
