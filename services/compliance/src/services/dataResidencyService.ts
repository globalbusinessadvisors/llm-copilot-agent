/**
 * Data Residency Service
 *
 * Manages data residency policies, regional data storage, and cross-border data transfer compliance.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  DataResidencyPolicy,
  DataClassification,
  DataRegion,
  CreateDataResidencyPolicyInput,
} from '../models/compliance';

interface DataAsset {
  id: string;
  name: string;
  type: string;
  classification: DataClassification;
  currentRegion: DataRegion;
  policies: string[];
  encryptionStatus: {
    atRest: boolean;
    inTransit: boolean;
    algorithm?: string;
  };
  retentionInfo: {
    createdAt: Date;
    expiresAt?: Date;
    deletionScheduled?: Date;
  };
  metadata: Record<string, unknown>;
  createdAt: Date;
  updatedAt: Date;
}

interface DataTransferRequest {
  id: string;
  assetId: string;
  sourceRegion: DataRegion;
  targetRegion: DataRegion;
  purpose: string;
  requestedBy: string;
  approvedBy?: string;
  status: 'pending' | 'approved' | 'denied' | 'completed' | 'failed';
  transferMechanism?: string;
  dpaReference?: string;
  startedAt?: Date;
  completedAt?: Date;
  createdAt: Date;
}

export class DataResidencyService {
  private db: Pool;
  private redis: RedisClientType;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Policy Management
  // ===========================================

  /**
   * Create data residency policy
   */
  async createPolicy(input: CreateDataResidencyPolicyInput, userId: string): Promise<DataResidencyPolicy> {
    const policy: DataResidencyPolicy = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      classification: input.classification,
      allowedRegions: input.allowedRegions,
      restrictedRegions: input.restrictedRegions,
      requirements: input.requirements,
      applicableTo: input.applicableTo,
      status: 'draft',
      effectiveDate: input.effectiveDate,
      expirationDate: input.expirationDate,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO data_residency_policies (
        id, name, description, classification, allowed_regions, restricted_regions,
        requirements, applicable_to, status, effective_date, expiration_date,
        created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)`,
      [
        policy.id, policy.name, policy.description, policy.classification,
        policy.allowedRegions, policy.restrictedRegions,
        JSON.stringify(policy.requirements), JSON.stringify(policy.applicableTo),
        policy.status, policy.effectiveDate, policy.expirationDate,
        policy.createdAt, policy.updatedAt, userId,
      ]
    );

    // Invalidate cache
    await this.redis.del('data-residency:policies:all');

    return policy;
  }

  /**
   * Get policy by ID
   */
  async getPolicy(policyId: string): Promise<DataResidencyPolicy | null> {
    const result = await this.db.query(
      `SELECT * FROM data_residency_policies WHERE id = $1`,
      [policyId]
    );

    if (result.rows.length === 0) return null;

    return this.mapPolicyRow(result.rows[0]);
  }

  /**
   * List policies
   */
  async listPolicies(filters?: {
    classification?: DataClassification;
    status?: DataResidencyPolicy['status'];
    region?: DataRegion;
  }): Promise<DataResidencyPolicy[]> {
    let query = `SELECT * FROM data_residency_policies WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.classification) {
      query += ` AND classification = $${paramIndex++}`;
      values.push(filters.classification);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.region) {
      query += ` AND $${paramIndex++} = ANY(allowed_regions)`;
      values.push(filters.region);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapPolicyRow);
  }

  /**
   * Activate policy
   */
  async activatePolicy(policyId: string): Promise<DataResidencyPolicy> {
    const policy = await this.getPolicy(policyId);
    if (!policy) throw new Error('Policy not found');

    await this.db.query(
      `UPDATE data_residency_policies SET status = 'active', updated_at = NOW() WHERE id = $1`,
      [policyId]
    );

    await this.redis.del('data-residency:policies:all');

    return { ...policy, status: 'active', updatedAt: new Date() };
  }

  /**
   * Get policies for data classification
   */
  async getPoliciesForClassification(classification: DataClassification): Promise<DataResidencyPolicy[]> {
    const cacheKey = `data-residency:policies:${classification}`;
    const cached = await this.redis.get(cacheKey);

    if (cached) {
      return JSON.parse(cached);
    }

    const policies = await this.listPolicies({
      classification,
      status: 'active',
    });

    await this.redis.setEx(cacheKey, 3600, JSON.stringify(policies));

    return policies;
  }

  // ===========================================
  // Data Asset Management
  // ===========================================

  /**
   * Register data asset
   */
  async registerDataAsset(input: {
    name: string;
    type: string;
    classification: DataClassification;
    region: DataRegion;
    encryptionStatus: DataAsset['encryptionStatus'];
    retentionDays?: number;
    metadata?: Record<string, unknown>;
  }, userId: string): Promise<DataAsset> {
    // Get applicable policies
    const policies = await this.getPoliciesForClassification(input.classification);
    const policyIds = policies.map(p => p.id);

    // Validate region against policies
    const regionAllowed = policies.every(p =>
      p.allowedRegions.includes(input.region) &&
      (!p.restrictedRegions || !p.restrictedRegions.includes(input.region))
    );

    if (!regionAllowed && policies.length > 0) {
      throw new Error(`Region ${input.region} is not allowed for ${input.classification} data`);
    }

    // Calculate expiration date
    let expiresAt: Date | undefined;
    if (input.retentionDays) {
      expiresAt = new Date();
      expiresAt.setDate(expiresAt.getDate() + input.retentionDays);
    }

    const asset: DataAsset = {
      id: uuidv4(),
      name: input.name,
      type: input.type,
      classification: input.classification,
      currentRegion: input.region,
      policies: policyIds,
      encryptionStatus: input.encryptionStatus,
      retentionInfo: {
        createdAt: new Date(),
        expiresAt,
      },
      metadata: input.metadata || {},
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO data_assets (
        id, name, type, classification, current_region, policies,
        encryption_status, retention_info, metadata, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        asset.id, asset.name, asset.type, asset.classification, asset.currentRegion,
        asset.policies, JSON.stringify(asset.encryptionStatus),
        JSON.stringify(asset.retentionInfo), JSON.stringify(asset.metadata),
        asset.createdAt, asset.updatedAt, userId,
      ]
    );

    return asset;
  }

  /**
   * Get data asset by ID
   */
  async getDataAsset(assetId: string): Promise<DataAsset | null> {
    const result = await this.db.query(
      `SELECT * FROM data_assets WHERE id = $1`,
      [assetId]
    );

    if (result.rows.length === 0) return null;

    return this.mapDataAssetRow(result.rows[0]);
  }

  /**
   * List data assets
   */
  async listDataAssets(filters?: {
    classification?: DataClassification;
    region?: DataRegion;
    type?: string;
  }): Promise<DataAsset[]> {
    let query = `SELECT * FROM data_assets WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.classification) {
      query += ` AND classification = $${paramIndex++}`;
      values.push(filters.classification);
    }
    if (filters?.region) {
      query += ` AND current_region = $${paramIndex++}`;
      values.push(filters.region);
    }
    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapDataAssetRow);
  }

  /**
   * Check compliance for data asset
   */
  async checkAssetCompliance(assetId: string): Promise<{
    compliant: boolean;
    violations: Array<{
      policyId: string;
      policyName: string;
      requirement: string;
      currentState: string;
    }>;
  }> {
    const asset = await this.getDataAsset(assetId);
    if (!asset) throw new Error('Asset not found');

    const violations: Array<{
      policyId: string;
      policyName: string;
      requirement: string;
      currentState: string;
    }> = [];

    for (const policyId of asset.policies) {
      const policy = await this.getPolicy(policyId);
      if (!policy || policy.status !== 'active') continue;

      // Check region
      if (!policy.allowedRegions.includes(asset.currentRegion)) {
        violations.push({
          policyId: policy.id,
          policyName: policy.name,
          requirement: `Data must be stored in: ${policy.allowedRegions.join(', ')}`,
          currentState: `Currently stored in: ${asset.currentRegion}`,
        });
      }

      // Check encryption at rest
      if (policy.requirements.encryption.atRest && !asset.encryptionStatus.atRest) {
        violations.push({
          policyId: policy.id,
          policyName: policy.name,
          requirement: 'Encryption at rest is required',
          currentState: 'Not encrypted at rest',
        });
      }

      // Check encryption in transit
      if (policy.requirements.encryption.inTransit && !asset.encryptionStatus.inTransit) {
        violations.push({
          policyId: policy.id,
          policyName: policy.name,
          requirement: 'Encryption in transit is required',
          currentState: 'Not encrypted in transit',
        });
      }

      // Check retention
      if (policy.requirements.retention.maxDays && asset.retentionInfo.expiresAt) {
        const maxDate = new Date(asset.retentionInfo.createdAt);
        maxDate.setDate(maxDate.getDate() + policy.requirements.retention.maxDays);

        if (asset.retentionInfo.expiresAt > maxDate) {
          violations.push({
            policyId: policy.id,
            policyName: policy.name,
            requirement: `Maximum retention: ${policy.requirements.retention.maxDays} days`,
            currentState: `Current retention exceeds maximum`,
          });
        }
      }
    }

    return {
      compliant: violations.length === 0,
      violations,
    };
  }

  // ===========================================
  // Cross-Border Data Transfer
  // ===========================================

  /**
   * Request data transfer
   */
  async requestDataTransfer(input: {
    assetId: string;
    targetRegion: DataRegion;
    purpose: string;
    transferMechanism?: string;
    dpaReference?: string;
  }, userId: string): Promise<DataTransferRequest> {
    const asset = await this.getDataAsset(input.assetId);
    if (!asset) throw new Error('Asset not found');

    // Check if transfer is allowed
    const transferValidation = await this.validateTransfer(asset, input.targetRegion);

    const request: DataTransferRequest = {
      id: uuidv4(),
      assetId: input.assetId,
      sourceRegion: asset.currentRegion,
      targetRegion: input.targetRegion,
      purpose: input.purpose,
      requestedBy: userId,
      status: transferValidation.allowed ? 'approved' : 'pending',
      transferMechanism: input.transferMechanism,
      dpaReference: input.dpaReference,
      createdAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO data_transfer_requests (
        id, asset_id, source_region, target_region, purpose, requested_by,
        status, transfer_mechanism, dpa_reference, created_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        request.id, request.assetId, request.sourceRegion, request.targetRegion,
        request.purpose, request.requestedBy, request.status,
        request.transferMechanism, request.dpaReference, request.createdAt,
      ]
    );

    // If not auto-approved, create notification for review
    if (request.status === 'pending') {
      await this.createTransferReviewNotification(request, transferValidation.reasons);
    }

    return request;
  }

  /**
   * Validate transfer against policies
   */
  private async validateTransfer(
    asset: DataAsset,
    targetRegion: DataRegion
  ): Promise<{
    allowed: boolean;
    reasons: string[];
  }> {
    const reasons: string[] = [];

    for (const policyId of asset.policies) {
      const policy = await this.getPolicy(policyId);
      if (!policy || policy.status !== 'active') continue;

      // Check if target region is allowed
      if (!policy.allowedRegions.includes(targetRegion)) {
        reasons.push(`Policy ${policy.name}: Target region ${targetRegion} not in allowed regions`);
      }

      // Check if target region is restricted
      if (policy.restrictedRegions?.includes(targetRegion)) {
        reasons.push(`Policy ${policy.name}: Target region ${targetRegion} is restricted`);
      }

      // Check cross-border transfer requirements
      if (!policy.requirements.transfer.allowCrossBorder &&
          this.getRegionCountry(asset.currentRegion) !== this.getRegionCountry(targetRegion)) {
        reasons.push(`Policy ${policy.name}: Cross-border transfers not allowed`);
      }

      // Check if DPA is required
      if (policy.requirements.transfer.requireDPA &&
          this.getRegionCountry(asset.currentRegion) !== this.getRegionCountry(targetRegion)) {
        reasons.push(`Policy ${policy.name}: DPA required for cross-border transfer`);
      }
    }

    return {
      allowed: reasons.length === 0,
      reasons,
    };
  }

  /**
   * Get country code from region
   */
  private getRegionCountry(region: DataRegion): string {
    const regionCountryMap: Record<DataRegion, string> = {
      [DataRegion.US_EAST]: 'US',
      [DataRegion.US_WEST]: 'US',
      [DataRegion.EU_WEST]: 'EU',
      [DataRegion.EU_CENTRAL]: 'EU',
      [DataRegion.APAC_SOUTH]: 'APAC',
      [DataRegion.APAC_EAST]: 'APAC',
      [DataRegion.GLOBAL]: 'GLOBAL',
    };
    return regionCountryMap[region];
  }

  /**
   * Approve transfer request
   */
  async approveTransferRequest(
    requestId: string,
    approverId: string,
    notes?: string
  ): Promise<DataTransferRequest> {
    const request = await this.getTransferRequest(requestId);
    if (!request) throw new Error('Transfer request not found');

    if (request.status !== 'pending') {
      throw new Error('Can only approve pending requests');
    }

    await this.db.query(
      `UPDATE data_transfer_requests SET
        status = 'approved', approved_by = $1, updated_at = NOW()
      WHERE id = $2`,
      [approverId, requestId]
    );

    // Log approval
    await this.logDataResidencyEvent({
      eventType: 'transfer_approved',
      assetId: request.assetId,
      requestId,
      userId: approverId,
      details: { notes },
    });

    return { ...request, status: 'approved', approvedBy: approverId };
  }

  /**
   * Execute approved transfer
   */
  async executeTransfer(requestId: string): Promise<DataTransferRequest> {
    const request = await this.getTransferRequest(requestId);
    if (!request) throw new Error('Transfer request not found');

    if (request.status !== 'approved') {
      throw new Error('Transfer must be approved before execution');
    }

    const startedAt = new Date();

    await this.db.query(
      `UPDATE data_transfer_requests SET started_at = $1 WHERE id = $2`,
      [startedAt, requestId]
    );

    try {
      // In production, this would trigger actual data transfer
      // For now, we just update the asset's region
      await this.db.query(
        `UPDATE data_assets SET current_region = $1, updated_at = NOW() WHERE id = $2`,
        [request.targetRegion, request.assetId]
      );

      const completedAt = new Date();

      await this.db.query(
        `UPDATE data_transfer_requests SET
          status = 'completed', completed_at = $1
        WHERE id = $2`,
        [completedAt, requestId]
      );

      // Log transfer completion
      await this.logDataResidencyEvent({
        eventType: 'transfer_completed',
        assetId: request.assetId,
        requestId,
        details: {
          sourceRegion: request.sourceRegion,
          targetRegion: request.targetRegion,
          duration: completedAt.getTime() - startedAt.getTime(),
        },
      });

      return { ...request, status: 'completed', startedAt, completedAt };
    } catch (error) {
      await this.db.query(
        `UPDATE data_transfer_requests SET status = 'failed' WHERE id = $1`,
        [requestId]
      );

      throw error;
    }
  }

  /**
   * Get transfer request by ID
   */
  async getTransferRequest(requestId: string): Promise<DataTransferRequest | null> {
    const result = await this.db.query(
      `SELECT * FROM data_transfer_requests WHERE id = $1`,
      [requestId]
    );

    if (result.rows.length === 0) return null;

    return this.mapTransferRequestRow(result.rows[0]);
  }

  /**
   * List transfer requests
   */
  async listTransferRequests(filters?: {
    assetId?: string;
    status?: DataTransferRequest['status'];
    sourceRegion?: DataRegion;
    targetRegion?: DataRegion;
  }): Promise<DataTransferRequest[]> {
    let query = `SELECT * FROM data_transfer_requests WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.assetId) {
      query += ` AND asset_id = $${paramIndex++}`;
      values.push(filters.assetId);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.sourceRegion) {
      query += ` AND source_region = $${paramIndex++}`;
      values.push(filters.sourceRegion);
    }
    if (filters?.targetRegion) {
      query += ` AND target_region = $${paramIndex++}`;
      values.push(filters.targetRegion);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapTransferRequestRow);
  }

  // ===========================================
  // Reporting
  // ===========================================

  /**
   * Generate data residency report
   */
  async generateReport(): Promise<{
    summary: {
      totalAssets: number;
      byClassification: Record<DataClassification, number>;
      byRegion: Record<DataRegion, number>;
      complianceRate: number;
    };
    transfers: {
      total: number;
      pending: number;
      completed: number;
      denied: number;
    };
    violations: Array<{
      assetId: string;
      assetName: string;
      violations: string[];
    }>;
  }> {
    // Get all assets
    const assets = await this.listDataAssets();

    // Calculate by classification
    const byClassification: Record<string, number> = {};
    const byRegion: Record<string, number> = {};

    assets.forEach(asset => {
      byClassification[asset.classification] = (byClassification[asset.classification] || 0) + 1;
      byRegion[asset.currentRegion] = (byRegion[asset.currentRegion] || 0) + 1;
    });

    // Check compliance for all assets
    const violations: Array<{
      assetId: string;
      assetName: string;
      violations: string[];
    }> = [];

    let compliantCount = 0;

    for (const asset of assets) {
      const compliance = await this.checkAssetCompliance(asset.id);
      if (compliance.compliant) {
        compliantCount++;
      } else {
        violations.push({
          assetId: asset.id,
          assetName: asset.name,
          violations: compliance.violations.map(v => `${v.policyName}: ${v.requirement}`),
        });
      }
    }

    // Get transfer stats
    const transferResult = await this.db.query(
      `SELECT status, COUNT(*) as count FROM data_transfer_requests GROUP BY status`
    );

    const transferStats: Record<string, number> = {};
    transferResult.rows.forEach(row => {
      transferStats[row.status] = parseInt(row.count, 10);
    });

    return {
      summary: {
        totalAssets: assets.length,
        byClassification: byClassification as Record<DataClassification, number>,
        byRegion: byRegion as Record<DataRegion, number>,
        complianceRate: assets.length > 0
          ? Math.round((compliantCount / assets.length) * 100)
          : 100,
      },
      transfers: {
        total: Object.values(transferStats).reduce((a, b) => a + b, 0),
        pending: transferStats.pending || 0,
        completed: transferStats.completed || 0,
        denied: transferStats.denied || 0,
      },
      violations,
    };
  }

  // ===========================================
  // Helpers
  // ===========================================

  /**
   * Create notification for transfer review
   */
  private async createTransferReviewNotification(
    request: DataTransferRequest,
    reasons: string[]
  ): Promise<void> {
    await this.db.query(
      `INSERT INTO notifications (id, type, title, message, data, created_at)
       VALUES ($1, $2, $3, $4, $5, NOW())`,
      [
        uuidv4(),
        'data_transfer_review',
        'Data Transfer Request Requires Review',
        `Transfer request for asset ${request.assetId} from ${request.sourceRegion} to ${request.targetRegion}`,
        JSON.stringify({ requestId: request.id, reasons }),
      ]
    );
  }

  /**
   * Log data residency event
   */
  private async logDataResidencyEvent(event: {
    eventType: string;
    assetId?: string;
    requestId?: string;
    userId?: string;
    details?: Record<string, unknown>;
  }): Promise<void> {
    await this.db.query(
      `INSERT INTO data_residency_events (id, event_type, asset_id, request_id, user_id, details, created_at)
       VALUES ($1, $2, $3, $4, $5, $6, NOW())`,
      [
        uuidv4(),
        event.eventType,
        event.assetId,
        event.requestId,
        event.userId,
        JSON.stringify(event.details),
      ]
    );
  }

  private mapPolicyRow(row: Record<string, unknown>): DataResidencyPolicy {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      classification: row.classification as DataClassification,
      allowedRegions: row.allowed_regions as DataRegion[],
      restrictedRegions: row.restricted_regions as DataRegion[] | undefined,
      requirements: row.requirements as DataResidencyPolicy['requirements'],
      applicableTo: row.applicable_to as DataResidencyPolicy['applicableTo'],
      status: row.status as DataResidencyPolicy['status'],
      effectiveDate: row.effective_date as Date | undefined,
      expirationDate: row.expiration_date as Date | undefined,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapDataAssetRow(row: Record<string, unknown>): DataAsset {
    return {
      id: row.id as string,
      name: row.name as string,
      type: row.type as string,
      classification: row.classification as DataClassification,
      currentRegion: row.current_region as DataRegion,
      policies: row.policies as string[],
      encryptionStatus: row.encryption_status as DataAsset['encryptionStatus'],
      retentionInfo: row.retention_info as DataAsset['retentionInfo'],
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
    };
  }

  private mapTransferRequestRow(row: Record<string, unknown>): DataTransferRequest {
    return {
      id: row.id as string,
      assetId: row.asset_id as string,
      sourceRegion: row.source_region as DataRegion,
      targetRegion: row.target_region as DataRegion,
      purpose: row.purpose as string,
      requestedBy: row.requested_by as string,
      approvedBy: row.approved_by as string | undefined,
      status: row.status as DataTransferRequest['status'],
      transferMechanism: row.transfer_mechanism as string | undefined,
      dpaReference: row.dpa_reference as string | undefined,
      startedAt: row.started_at as Date | undefined,
      completedAt: row.completed_at as Date | undefined,
      createdAt: row.created_at as Date,
    };
  }
}
