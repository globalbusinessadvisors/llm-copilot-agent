/**
 * Compliance Service
 *
 * Manages compliance controls, audits, and reporting for various frameworks.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  Control,
  Audit,
  Finding,
  ComplianceReport,
  ComplianceFramework,
  ControlCategory,
  ControlStatus,
  AuditStatus,
  FindingSeverity,
  FindingStatus,
  EvidenceType,
  CreateControlInput,
  CreateAuditInput,
  CreateFindingInput,
  GenerateReportInput,
} from '../models/compliance';

export class ComplianceService {
  private db: Pool;
  private redis: RedisClientType;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Control Management
  // ===========================================

  /**
   * Create a new control
   */
  async createControl(input: CreateControlInput, userId: string): Promise<Control> {
    const control: Control = {
      id: uuidv4(),
      framework: input.framework,
      controlId: input.controlId,
      name: input.name,
      description: input.description,
      category: input.category,
      status: ControlStatus.NOT_IMPLEMENTED,
      owner: input.owner,
      implementation: input.implementation || {
        automationLevel: 'manual',
      },
      testing: {
        frequency: 'quarterly',
        ...input.testing,
      },
      relatedControls: input.relatedControls,
      metadata: input.metadata,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO compliance_controls (
        id, framework, control_id, name, description, category, status,
        owner, implementation, testing, related_controls, metadata,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        control.id, control.framework, control.controlId, control.name,
        control.description, control.category, control.status, control.owner,
        JSON.stringify(control.implementation), JSON.stringify(control.testing),
        control.relatedControls, JSON.stringify(control.metadata),
        control.createdAt, control.updatedAt, userId,
      ]
    );

    // Invalidate cache
    await this.redis.del(`compliance:controls:${control.framework}`);

    return control;
  }

  /**
   * Get control by ID
   */
  async getControl(controlId: string): Promise<Control | null> {
    const result = await this.db.query(
      `SELECT * FROM compliance_controls WHERE id = $1`,
      [controlId]
    );

    if (result.rows.length === 0) return null;

    return this.mapControlRow(result.rows[0]);
  }

  /**
   * List controls by framework
   */
  async listControls(filters?: {
    framework?: ComplianceFramework;
    category?: ControlCategory;
    status?: ControlStatus;
    owner?: string;
  }): Promise<Control[]> {
    let query = `SELECT * FROM compliance_controls WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.framework) {
      query += ` AND framework = $${paramIndex++}`;
      values.push(filters.framework);
    }
    if (filters?.category) {
      query += ` AND category = $${paramIndex++}`;
      values.push(filters.category);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.owner) {
      query += ` AND owner = $${paramIndex++}`;
      values.push(filters.owner);
    }

    query += ` ORDER BY control_id ASC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapControlRow);
  }

  /**
   * Update control status
   */
  async updateControlStatus(
    controlId: string,
    status: ControlStatus,
    evidence?: Control['evidence']
  ): Promise<Control> {
    const control = await this.getControl(controlId);
    if (!control) throw new Error('Control not found');

    const updates: Partial<Control> = {
      status,
      updatedAt: new Date(),
    };

    if (evidence) {
      updates.evidence = [...(control.evidence || []), ...evidence];
    }

    await this.db.query(
      `UPDATE compliance_controls SET
        status = $1, evidence = COALESCE($2, evidence), updated_at = $3
      WHERE id = $4`,
      [status, evidence ? JSON.stringify(updates.evidence) : null, updates.updatedAt, controlId]
    );

    return { ...control, ...updates };
  }

  /**
   * Record control test
   */
  async recordControlTest(
    controlId: string,
    testResult: {
      passed: boolean;
      notes?: string;
      evidence?: Control['evidence'];
    }
  ): Promise<Control> {
    const control = await this.getControl(controlId);
    if (!control) throw new Error('Control not found');

    const now = new Date();
    const nextTest = this.calculateNextTestDate(now, control.testing.frequency);

    // Update status based on test result
    let newStatus = control.status;
    if (testResult.passed) {
      newStatus = ControlStatus.EFFECTIVE;
    } else if (control.status === ControlStatus.EFFECTIVE) {
      newStatus = ControlStatus.NEEDS_IMPROVEMENT;
    }

    await this.db.query(
      `UPDATE compliance_controls SET
        status = $1,
        testing = jsonb_set(
          jsonb_set(testing, '{lastTested}', to_jsonb($2::text)),
          '{nextTest}', to_jsonb($3::text)
        ),
        evidence = COALESCE($4, evidence),
        updated_at = $5
      WHERE id = $6`,
      [
        newStatus,
        now.toISOString(),
        nextTest.toISOString(),
        testResult.evidence ? JSON.stringify([...(control.evidence || []), ...testResult.evidence]) : null,
        now,
        controlId,
      ]
    );

    // Record test in audit log
    await this.recordAuditEvent('control_test', controlId, {
      passed: testResult.passed,
      notes: testResult.notes,
      previousStatus: control.status,
      newStatus,
    });

    return {
      ...control,
      status: newStatus,
      testing: {
        ...control.testing,
        lastTested: now,
        nextTest,
      },
    };
  }

  /**
   * Calculate next test date based on frequency
   */
  private calculateNextTestDate(from: Date, frequency: string): Date {
    const next = new Date(from);

    switch (frequency) {
      case 'continuous':
        next.setDate(next.getDate() + 1);
        break;
      case 'daily':
        next.setDate(next.getDate() + 1);
        break;
      case 'weekly':
        next.setDate(next.getDate() + 7);
        break;
      case 'monthly':
        next.setMonth(next.getMonth() + 1);
        break;
      case 'quarterly':
        next.setMonth(next.getMonth() + 3);
        break;
      case 'annually':
        next.setFullYear(next.getFullYear() + 1);
        break;
      default:
        next.setMonth(next.getMonth() + 3);
    }

    return next;
  }

  // ===========================================
  // Audit Management
  // ===========================================

  /**
   * Create audit
   */
  async createAudit(input: CreateAuditInput, userId: string): Promise<Audit> {
    const audit: Audit = {
      id: uuidv4(),
      name: input.name,
      framework: input.framework,
      type: input.type,
      status: AuditStatus.SCHEDULED,
      scope: input.scope,
      auditor: input.auditor,
      schedule: input.schedule,
      findings: [],
      metadata: input.metadata,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO compliance_audits (
        id, name, framework, type, status, scope, auditor, schedule,
        findings, metadata, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
      [
        audit.id, audit.name, audit.framework, audit.type, audit.status,
        JSON.stringify(audit.scope), JSON.stringify(audit.auditor),
        JSON.stringify(audit.schedule), JSON.stringify(audit.findings),
        JSON.stringify(audit.metadata), audit.createdAt, audit.updatedAt, userId,
      ]
    );

    return audit;
  }

  /**
   * Get audit by ID
   */
  async getAudit(auditId: string): Promise<Audit | null> {
    const result = await this.db.query(
      `SELECT * FROM compliance_audits WHERE id = $1`,
      [auditId]
    );

    if (result.rows.length === 0) return null;

    return this.mapAuditRow(result.rows[0]);
  }

  /**
   * List audits
   */
  async listAudits(filters?: {
    framework?: ComplianceFramework;
    status?: AuditStatus;
    type?: Audit['type'];
  }): Promise<Audit[]> {
    let query = `SELECT * FROM compliance_audits WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.framework) {
      query += ` AND framework = $${paramIndex++}`;
      values.push(filters.framework);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapAuditRow);
  }

  /**
   * Update audit status
   */
  async updateAuditStatus(auditId: string, status: AuditStatus): Promise<Audit> {
    const audit = await this.getAudit(auditId);
    if (!audit) throw new Error('Audit not found');

    const updates: Partial<Audit> = {
      status,
      updatedAt: new Date(),
    };

    // Update schedule dates based on status
    if (status === AuditStatus.IN_PROGRESS && !audit.schedule.actualStart) {
      updates.schedule = { ...audit.schedule, actualStart: new Date() };
    } else if (status === AuditStatus.COMPLETED && !audit.schedule.actualEnd) {
      updates.schedule = { ...audit.schedule, actualEnd: new Date() };
    }

    await this.db.query(
      `UPDATE compliance_audits SET
        status = $1, schedule = COALESCE($2, schedule), updated_at = $3
      WHERE id = $4`,
      [
        status,
        updates.schedule ? JSON.stringify(updates.schedule) : null,
        updates.updatedAt,
        auditId,
      ]
    );

    return { ...audit, ...updates };
  }

  // ===========================================
  // Finding Management
  // ===========================================

  /**
   * Create finding
   */
  async createFinding(input: CreateFindingInput, userId: string): Promise<Finding> {
    // Calculate risk score
    const likelihoodScores: Record<string, number> = {
      rare: 1, unlikely: 2, possible: 3, likely: 4, almost_certain: 5,
    };
    const impactScores: Record<string, number> = {
      negligible: 1, minor: 2, moderate: 3, major: 4, catastrophic: 5,
    };
    const riskScore = likelihoodScores[input.risk.likelihood] * impactScores[input.risk.impact];

    const finding: Finding = {
      id: uuidv4(),
      auditId: input.auditId,
      controlId: input.controlId,
      title: input.title,
      description: input.description,
      severity: input.severity,
      status: FindingStatus.OPEN,
      risk: {
        ...input.risk,
        score: riskScore,
      },
      remediation: {
        ...input.remediation,
      },
      evidence: input.evidence,
      comments: [],
      metadata: input.metadata,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO compliance_findings (
        id, audit_id, control_id, title, description, severity, status,
        risk, remediation, evidence, comments, metadata, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)`,
      [
        finding.id, finding.auditId, finding.controlId, finding.title,
        finding.description, finding.severity, finding.status,
        JSON.stringify(finding.risk), JSON.stringify(finding.remediation),
        JSON.stringify(finding.evidence), JSON.stringify(finding.comments),
        JSON.stringify(finding.metadata), finding.createdAt, finding.updatedAt, userId,
      ]
    );

    // Add finding to audit
    await this.db.query(
      `UPDATE compliance_audits SET
        findings = array_append(findings, $1), updated_at = NOW()
      WHERE id = $2`,
      [finding.id, input.auditId]
    );

    return finding;
  }

  /**
   * Get finding by ID
   */
  async getFinding(findingId: string): Promise<Finding | null> {
    const result = await this.db.query(
      `SELECT * FROM compliance_findings WHERE id = $1`,
      [findingId]
    );

    if (result.rows.length === 0) return null;

    return this.mapFindingRow(result.rows[0]);
  }

  /**
   * List findings
   */
  async listFindings(filters?: {
    auditId?: string;
    controlId?: string;
    severity?: FindingSeverity;
    status?: FindingStatus;
  }): Promise<Finding[]> {
    let query = `SELECT * FROM compliance_findings WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.auditId) {
      query += ` AND audit_id = $${paramIndex++}`;
      values.push(filters.auditId);
    }
    if (filters?.controlId) {
      query += ` AND control_id = $${paramIndex++}`;
      values.push(filters.controlId);
    }
    if (filters?.severity) {
      query += ` AND severity = $${paramIndex++}`;
      values.push(filters.severity);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }

    query += ` ORDER BY risk->>'score' DESC, created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapFindingRow);
  }

  /**
   * Update finding status
   */
  async updateFindingStatus(
    findingId: string,
    status: FindingStatus,
    userId: string,
    comment?: string
  ): Promise<Finding> {
    const finding = await this.getFinding(findingId);
    if (!finding) throw new Error('Finding not found');

    const updates: Partial<Finding> = {
      status,
      updatedAt: new Date(),
    };

    // Add completion/verification info
    if (status === FindingStatus.REMEDIATED) {
      updates.remediation = {
        ...finding.remediation,
        completedDate: new Date(),
      };
    } else if (status === FindingStatus.CLOSED) {
      updates.remediation = {
        ...finding.remediation,
        verifiedBy: userId,
        verifiedDate: new Date(),
      };
    }

    // Add comment if provided
    if (comment) {
      updates.comments = [
        ...(finding.comments || []),
        {
          author: userId,
          content: comment,
          createdAt: new Date(),
        },
      ];
    }

    await this.db.query(
      `UPDATE compliance_findings SET
        status = $1, remediation = $2, comments = $3, updated_at = $4
      WHERE id = $5`,
      [
        status,
        JSON.stringify(updates.remediation || finding.remediation),
        JSON.stringify(updates.comments || finding.comments),
        updates.updatedAt,
        findingId,
      ]
    );

    return { ...finding, ...updates };
  }

  // ===========================================
  // Compliance Reporting
  // ===========================================

  /**
   * Generate compliance report
   */
  async generateReport(input: GenerateReportInput, userId: string): Promise<ComplianceReport> {
    // Get controls for the framework
    const controls = await this.listControls({ framework: input.framework });

    // Get findings for the period
    const findings = await this.db.query(
      `SELECT * FROM compliance_findings
       WHERE created_at >= $1 AND created_at <= $2`,
      [input.period.start, input.period.end]
    );

    const allFindings = findings.rows.map(this.mapFindingRow);

    // Group controls by category
    const categoryControls = new Map<ControlCategory, Control[]>();
    controls.forEach(control => {
      const existing = categoryControls.get(control.category) || [];
      categoryControls.set(control.category, [...existing, control]);
    });

    // Build report sections
    const sections: ComplianceReport['sections'] = [];
    for (const [category, categoryControlList] of categoryControls) {
      const controlsWithFindings = categoryControlList.map(control => {
        const controlFindings = allFindings.filter(f => f.controlId === control.id);
        return {
          controlId: control.controlId,
          name: control.name,
          status: control.status,
          findings: controlFindings.map(f => ({
            id: f.id,
            severity: f.severity,
            status: f.status,
          })),
        };
      });

      // Calculate category score
      const implementedCount = categoryControlList.filter(
        c => c.status === ControlStatus.IMPLEMENTED || c.status === ControlStatus.EFFECTIVE
      ).length;
      const score = categoryControlList.length > 0
        ? Math.round((implementedCount / categoryControlList.length) * 100)
        : 0;

      sections.push({
        category,
        controls: controlsWithFindings,
        score,
      });
    }

    // Calculate summary
    const totalControls = controls.length;
    const implementedControls = controls.filter(
      c => c.status !== ControlStatus.NOT_IMPLEMENTED
    ).length;
    const effectiveControls = controls.filter(
      c => c.status === ControlStatus.EFFECTIVE
    ).length;
    const openFindings = allFindings.filter(
      f => f.status === FindingStatus.OPEN || f.status === FindingStatus.IN_PROGRESS
    ).length;
    const overallScore = totalControls > 0
      ? Math.round((effectiveControls / totalControls) * 100)
      : 0;

    // Generate recommendations if requested
    let recommendations: ComplianceReport['recommendations'];
    if (input.includeRecommendations) {
      recommendations = this.generateRecommendations(controls, allFindings);
    }

    const report: ComplianceReport = {
      id: uuidv4(),
      name: `${input.framework} ${input.reportType} Report`,
      framework: input.framework,
      reportType: input.reportType,
      period: input.period,
      summary: {
        totalControls,
        implementedControls,
        effectiveControls,
        openFindings,
        overallScore,
      },
      sections,
      recommendations,
      generatedAt: new Date(),
      generatedBy: userId,
      format: input.format,
    };

    // Store report
    await this.db.query(
      `INSERT INTO compliance_reports (
        id, name, framework, report_type, period, summary, sections,
        recommendations, generated_at, generated_by, format
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)`,
      [
        report.id, report.name, report.framework, report.reportType,
        JSON.stringify(report.period), JSON.stringify(report.summary),
        JSON.stringify(report.sections), JSON.stringify(report.recommendations),
        report.generatedAt, userId, report.format,
      ]
    );

    return report;
  }

  /**
   * Generate recommendations based on controls and findings
   */
  private generateRecommendations(
    controls: Control[],
    findings: Finding[]
  ): ComplianceReport['recommendations'] {
    const recommendations: NonNullable<ComplianceReport['recommendations']> = [];

    // Check for not implemented controls
    const notImplemented = controls.filter(c => c.status === ControlStatus.NOT_IMPLEMENTED);
    if (notImplemented.length > 0) {
      recommendations.push({
        priority: 'high',
        description: `Implement ${notImplemented.length} controls that are currently not implemented`,
        impact: 'Significant improvement in compliance posture',
      });
    }

    // Check for critical findings
    const criticalFindings = findings.filter(
      f => f.severity === FindingSeverity.CRITICAL && f.status === FindingStatus.OPEN
    );
    if (criticalFindings.length > 0) {
      recommendations.push({
        priority: 'critical',
        description: `Address ${criticalFindings.length} critical findings immediately`,
        impact: 'Prevent potential security incidents and compliance violations',
      });
    }

    // Check for overdue remediations
    const now = new Date();
    const overdueFindings = findings.filter(
      f => f.status !== FindingStatus.CLOSED &&
           f.remediation.dueDate < now
    );
    if (overdueFindings.length > 0) {
      recommendations.push({
        priority: 'high',
        description: `Remediate ${overdueFindings.length} findings that are past their due date`,
        impact: 'Reduce risk exposure and demonstrate commitment to compliance',
      });
    }

    // Check for controls needing improvement
    const needsImprovement = controls.filter(
      c => c.status === ControlStatus.NEEDS_IMPROVEMENT
    );
    if (needsImprovement.length > 0) {
      recommendations.push({
        priority: 'medium',
        description: `Review and enhance ${needsImprovement.length} controls that need improvement`,
        impact: 'Strengthen overall control effectiveness',
      });
    }

    // Check for overdue control tests
    const overdueTests = controls.filter(
      c => c.testing.nextTest && c.testing.nextTest < now
    );
    if (overdueTests.length > 0) {
      recommendations.push({
        priority: 'medium',
        description: `Complete testing for ${overdueTests.length} controls with overdue tests`,
        impact: 'Maintain evidence of control effectiveness',
      });
    }

    return recommendations;
  }

  /**
   * Get compliance dashboard metrics
   */
  async getDashboardMetrics(framework?: ComplianceFramework): Promise<{
    controlsOverview: {
      total: number;
      byStatus: Record<ControlStatus, number>;
      byCategory: Record<ControlCategory, number>;
    };
    findingsOverview: {
      total: number;
      open: number;
      bySeverity: Record<FindingSeverity, number>;
      overdueCount: number;
    };
    auditOverview: {
      upcoming: number;
      inProgress: number;
      completed: number;
    };
    complianceScore: number;
    trendData: Array<{ date: string; score: number }>;
  }> {
    const controlFilter = framework ? { framework } : undefined;
    const controls = await this.listControls(controlFilter);

    // Controls overview
    const byStatus: Record<string, number> = {};
    const byCategory: Record<string, number> = {};
    controls.forEach(c => {
      byStatus[c.status] = (byStatus[c.status] || 0) + 1;
      byCategory[c.category] = (byCategory[c.category] || 0) + 1;
    });

    // Findings overview
    const findings = await this.listFindings();
    const now = new Date();
    const bySeverity: Record<string, number> = {};
    let openCount = 0;
    let overdueCount = 0;
    findings.forEach(f => {
      bySeverity[f.severity] = (bySeverity[f.severity] || 0) + 1;
      if (f.status === FindingStatus.OPEN || f.status === FindingStatus.IN_PROGRESS) {
        openCount++;
        if (f.remediation.dueDate < now) {
          overdueCount++;
        }
      }
    });

    // Audit overview
    const audits = await this.listAudits();
    const auditOverview = {
      upcoming: audits.filter(a => a.status === AuditStatus.SCHEDULED).length,
      inProgress: audits.filter(a => a.status === AuditStatus.IN_PROGRESS).length,
      completed: audits.filter(a => a.status === AuditStatus.COMPLETED).length,
    };

    // Calculate compliance score
    const effectiveControls = controls.filter(c => c.status === ControlStatus.EFFECTIVE).length;
    const complianceScore = controls.length > 0
      ? Math.round((effectiveControls / controls.length) * 100)
      : 0;

    // Generate trend data (mock for now - would need historical data)
    const trendData = this.generateTrendData();

    return {
      controlsOverview: {
        total: controls.length,
        byStatus: byStatus as Record<ControlStatus, number>,
        byCategory: byCategory as Record<ControlCategory, number>,
      },
      findingsOverview: {
        total: findings.length,
        open: openCount,
        bySeverity: bySeverity as Record<FindingSeverity, number>,
        overdueCount,
      },
      auditOverview,
      complianceScore,
      trendData,
    };
  }

  /**
   * Generate trend data (placeholder - would need historical storage)
   */
  private generateTrendData(): Array<{ date: string; score: number }> {
    const data: Array<{ date: string; score: number }> = [];
    const now = new Date();

    for (let i = 11; i >= 0; i--) {
      const date = new Date(now);
      date.setMonth(date.getMonth() - i);
      data.push({
        date: date.toISOString().slice(0, 7), // YYYY-MM format
        score: Math.floor(Math.random() * 20) + 70, // Mock score 70-90
      });
    }

    return data;
  }

  // ===========================================
  // Audit Event Logging
  // ===========================================

  /**
   * Record audit event
   */
  async recordAuditEvent(
    action: string,
    resourceId: string,
    details: Record<string, unknown>,
    userId?: string
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO compliance_audit_log (id, action, resource_id, details, user_id, created_at)
       VALUES ($1, $2, $3, $4, $5, NOW())`,
      [uuidv4(), action, resourceId, JSON.stringify(details), userId || 'system']
    );
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapControlRow(row: Record<string, unknown>): Control {
    return {
      id: row.id as string,
      framework: row.framework as ComplianceFramework,
      controlId: row.control_id as string,
      name: row.name as string,
      description: row.description as string,
      category: row.category as ControlCategory,
      status: row.status as ControlStatus,
      owner: row.owner as string,
      implementation: row.implementation as Control['implementation'],
      testing: row.testing as Control['testing'],
      evidence: row.evidence as Control['evidence'],
      relatedControls: row.related_controls as string[],
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
    };
  }

  private mapAuditRow(row: Record<string, unknown>): Audit {
    return {
      id: row.id as string,
      name: row.name as string,
      framework: row.framework as ComplianceFramework,
      type: row.type as Audit['type'],
      status: row.status as AuditStatus,
      scope: row.scope as Audit['scope'],
      auditor: row.auditor as Audit['auditor'],
      schedule: row.schedule as Audit['schedule'],
      findings: row.findings as string[],
      report: row.report as Audit['report'],
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapFindingRow(row: Record<string, unknown>): Finding {
    return {
      id: row.id as string,
      auditId: row.audit_id as string,
      controlId: row.control_id as string | undefined,
      title: row.title as string,
      description: row.description as string,
      severity: row.severity as FindingSeverity,
      status: row.status as FindingStatus,
      risk: row.risk as Finding['risk'],
      remediation: row.remediation as Finding['remediation'],
      evidence: row.evidence as Finding['evidence'],
      comments: row.comments as Finding['comments'],
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
    };
  }
}
