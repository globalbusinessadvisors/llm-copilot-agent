/**
 * Policy Service
 *
 * Manages usage policies and enforces policy rules across the platform.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  Policy,
  PolicyViolation,
  PolicyType,
  PolicyScope,
  PolicyEnforcement,
  CreatePolicyInput,
  EvaluatePolicyInput,
} from '../models/governance';

interface PolicyEvaluationResult {
  allowed: boolean;
  policy?: Policy;
  violation?: PolicyViolation;
  warnings: string[];
}

export class PolicyService {
  private db: Pool;
  private redis: RedisClientType;
  private policyCache: Map<string, Policy[]> = new Map();

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Policy Management
  // ===========================================

  /**
   * Create a policy
   */
  async createPolicy(input: CreatePolicyInput, userId: string): Promise<Policy> {
    const policy: Policy = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      type: input.type,
      scope: input.scope,
      enforcement: input.enforcement,
      rules: input.rules,
      targets: input.targets,
      exceptions: input.exceptions,
      version: 1,
      status: 'draft',
      effectiveDate: input.effectiveDate,
      expirationDate: input.expirationDate,
      metadata: input.metadata,
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO policies (
        id, name, description, type, scope, enforcement, rules, targets,
        exceptions, version, status, effective_date, expiration_date,
        metadata, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)`,
      [
        policy.id, policy.name, policy.description, policy.type, policy.scope,
        policy.enforcement, JSON.stringify(policy.rules), JSON.stringify(policy.targets),
        JSON.stringify(policy.exceptions), policy.version, policy.status,
        policy.effectiveDate, policy.expirationDate, JSON.stringify(policy.metadata),
        policy.createdAt, policy.updatedAt, userId,
      ]
    );

    return policy;
  }

  /**
   * Get policy by ID
   */
  async getPolicy(policyId: string): Promise<Policy | null> {
    const result = await this.db.query(
      `SELECT * FROM policies WHERE id = $1`,
      [policyId]
    );

    if (result.rows.length === 0) return null;

    return this.mapPolicyRow(result.rows[0]);
  }

  /**
   * List policies
   */
  async listPolicies(filters?: {
    type?: PolicyType;
    scope?: PolicyScope;
    status?: Policy['status'];
    enforcement?: PolicyEnforcement;
  }): Promise<Policy[]> {
    let query = `SELECT * FROM policies WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }
    if (filters?.scope) {
      query += ` AND scope = $${paramIndex++}`;
      values.push(filters.scope);
    }
    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.enforcement) {
      query += ` AND enforcement = $${paramIndex++}`;
      values.push(filters.enforcement);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapPolicyRow);
  }

  /**
   * Update policy
   */
  async updatePolicy(
    policyId: string,
    updates: Partial<CreatePolicyInput>
  ): Promise<Policy> {
    const policy = await this.getPolicy(policyId);
    if (!policy) throw new Error('Policy not found');

    // Increment version for significant changes
    const newVersion = policy.version + 1;

    const updatedPolicy = {
      ...policy,
      ...updates,
      version: newVersion,
      updatedAt: new Date(),
    };

    await this.db.query(
      `UPDATE policies SET
        name = $1, description = $2, type = $3, scope = $4, enforcement = $5,
        rules = $6, targets = $7, exceptions = $8, version = $9,
        effective_date = $10, expiration_date = $11, metadata = $12, updated_at = $13
      WHERE id = $14`,
      [
        updatedPolicy.name, updatedPolicy.description, updatedPolicy.type,
        updatedPolicy.scope, updatedPolicy.enforcement, JSON.stringify(updatedPolicy.rules),
        JSON.stringify(updatedPolicy.targets), JSON.stringify(updatedPolicy.exceptions),
        updatedPolicy.version, updatedPolicy.effectiveDate, updatedPolicy.expirationDate,
        JSON.stringify(updatedPolicy.metadata), updatedPolicy.updatedAt, policyId,
      ]
    );

    // Invalidate cache
    await this.invalidatePolicyCache();

    return updatedPolicy;
  }

  /**
   * Activate policy
   */
  async activatePolicy(policyId: string): Promise<Policy> {
    const policy = await this.getPolicy(policyId);
    if (!policy) throw new Error('Policy not found');

    await this.db.query(
      `UPDATE policies SET status = 'active', updated_at = NOW() WHERE id = $1`,
      [policyId]
    );

    await this.invalidatePolicyCache();

    return { ...policy, status: 'active', updatedAt: new Date() };
  }

  /**
   * Deprecate policy
   */
  async deprecatePolicy(policyId: string): Promise<Policy> {
    const policy = await this.getPolicy(policyId);
    if (!policy) throw new Error('Policy not found');

    await this.db.query(
      `UPDATE policies SET status = 'deprecated', updated_at = NOW() WHERE id = $1`,
      [policyId]
    );

    await this.invalidatePolicyCache();

    return { ...policy, status: 'deprecated', updatedAt: new Date() };
  }

  // ===========================================
  // Policy Evaluation
  // ===========================================

  /**
   * Evaluate policies for an action
   */
  async evaluatePolicy(input: EvaluatePolicyInput): Promise<PolicyEvaluationResult> {
    const policies = await this.getApplicablePolicies(input);
    const warnings: string[] = [];
    let blockedByPolicy: Policy | undefined;

    for (const policy of policies) {
      // Check if within effective dates
      const now = new Date();
      if (policy.effectiveDate && policy.effectiveDate > now) continue;
      if (policy.expirationDate && policy.expirationDate < now) continue;

      // Check exceptions
      if (this.isExempt(policy, input)) {
        continue;
      }

      // Evaluate rules
      const ruleResult = this.evaluateRules(policy, input);

      if (!ruleResult.passed) {
        if (policy.enforcement === PolicyEnforcement.STRICT) {
          blockedByPolicy = policy;
          break;
        } else if (policy.enforcement === PolicyEnforcement.PERMISSIVE) {
          warnings.push(`Policy ${policy.name}: ${ruleResult.reason}`);
        } else {
          // Audit only - log but don't block
          await this.logPolicyViolation(policy, input, ruleResult.reason, false);
        }
      }
    }

    if (blockedByPolicy) {
      const violation = await this.logPolicyViolation(
        blockedByPolicy,
        input,
        'Action blocked by policy',
        true
      );

      return {
        allowed: false,
        policy: blockedByPolicy,
        violation,
        warnings,
      };
    }

    return {
      allowed: true,
      warnings,
    };
  }

  /**
   * Get applicable policies for an action
   */
  private async getApplicablePolicies(input: EvaluatePolicyInput): Promise<Policy[]> {
    const cacheKey = `policies:${input.resource.type}`;

    // Check cache
    if (this.policyCache.has(cacheKey)) {
      return this.policyCache.get(cacheKey)!;
    }

    // Get active policies
    const policies = await this.listPolicies({ status: 'active' });

    // Filter to applicable policies
    const applicable = policies.filter(policy => {
      // Check scope
      if (policy.targets) {
        const targets = policy.targets;

        if (targets.users && !targets.users.includes(input.userId)) {
          return false;
        }
        // Add other target checks as needed
      }

      return true;
    });

    this.policyCache.set(cacheKey, applicable);

    return applicable;
  }

  /**
   * Evaluate policy rules
   */
  private evaluateRules(
    policy: Policy,
    input: EvaluatePolicyInput
  ): { passed: boolean; reason?: string } {
    for (const rule of policy.rules) {
      const conditionResult = this.evaluateCondition(rule.condition, input);

      if (conditionResult) {
        // Condition matched, check if action is allowed
        if (rule.action === 'deny') {
          return {
            passed: false,
            reason: `Rule ${rule.id}: ${rule.condition}`,
          };
        }
      }
    }

    return { passed: true };
  }

  /**
   * Evaluate a condition expression
   */
  private evaluateCondition(condition: string, input: EvaluatePolicyInput): boolean {
    try {
      // Simple condition evaluation
      // In production, use a proper expression evaluator

      // Check for resource type conditions
      if (condition.includes('resource.type')) {
        const match = condition.match(/resource\.type\s*==\s*['"](\w+)['"]/);
        if (match && match[1] !== input.resource.type) {
          return false;
        }
      }

      // Check for action conditions
      if (condition.includes('action')) {
        const match = condition.match(/action\s*==\s*['"](\w+)['"]/);
        if (match && match[1] !== input.action) {
          return false;
        }
      }

      // Check for user conditions
      if (condition.includes('user.id')) {
        const match = condition.match(/user\.id\s*==\s*['"](\w+)['"]/);
        if (match && match[1] !== input.userId) {
          return false;
        }
      }

      // Rate limiting conditions
      if (condition.includes('rate_limit')) {
        // Would check rate limits from Redis
        return false;
      }

      // Time-based conditions
      if (condition.includes('time')) {
        const match = condition.match(/time\.hour\s*(>=|<=|==)\s*(\d+)/);
        if (match) {
          const hour = new Date().getHours();
          const targetHour = parseInt(match[2], 10);
          switch (match[1]) {
            case '>=': return hour >= targetHour;
            case '<=': return hour <= targetHour;
            case '==': return hour === targetHour;
          }
        }
      }

      return true;
    } catch {
      return false;
    }
  }

  /**
   * Check if user/action is exempt from policy
   */
  private isExempt(policy: Policy, input: EvaluatePolicyInput): boolean {
    if (!policy.exceptions) return false;

    const now = new Date();

    for (const exception of policy.exceptions) {
      // Check if exception is expired
      if (exception.expiresAt && exception.expiresAt < now) {
        continue;
      }

      if (exception.type === 'user' && exception.value === input.userId) {
        return true;
      }

      if (exception.type === 'action' && exception.value === input.action) {
        return true;
      }

      if (exception.type === 'resource' && exception.value === input.resource.type) {
        return true;
      }
    }

    return false;
  }

  /**
   * Log policy violation
   */
  private async logPolicyViolation(
    policy: Policy,
    input: EvaluatePolicyInput,
    details: string,
    blocked: boolean
  ): Promise<PolicyViolation> {
    const violation: PolicyViolation = {
      id: uuidv4(),
      policyId: policy.id,
      policyName: policy.name,
      ruleId: policy.rules[0]?.id || 'unknown',
      userId: input.userId,
      action: input.action,
      resource: input.resource,
      violationType: blocked ? 'hard' : 'soft',
      blocked,
      details,
      context: input.context,
      timestamp: new Date(),
    };

    await this.db.query(
      `INSERT INTO policy_violations (
        id, policy_id, policy_name, rule_id, user_id, action, resource,
        violation_type, blocked, details, context, timestamp
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)`,
      [
        violation.id, violation.policyId, violation.policyName, violation.ruleId,
        violation.userId, violation.action, JSON.stringify(violation.resource),
        violation.violationType, violation.blocked, violation.details,
        JSON.stringify(violation.context), violation.timestamp,
      ]
    );

    return violation;
  }

  /**
   * Get policy violations
   */
  async getViolations(filters?: {
    policyId?: string;
    userId?: string;
    blocked?: boolean;
    startDate?: Date;
    endDate?: Date;
    limit?: number;
  }): Promise<PolicyViolation[]> {
    let query = `SELECT * FROM policy_violations WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.policyId) {
      query += ` AND policy_id = $${paramIndex++}`;
      values.push(filters.policyId);
    }
    if (filters?.userId) {
      query += ` AND user_id = $${paramIndex++}`;
      values.push(filters.userId);
    }
    if (filters?.blocked !== undefined) {
      query += ` AND blocked = $${paramIndex++}`;
      values.push(filters.blocked);
    }
    if (filters?.startDate) {
      query += ` AND timestamp >= $${paramIndex++}`;
      values.push(filters.startDate);
    }
    if (filters?.endDate) {
      query += ` AND timestamp <= $${paramIndex++}`;
      values.push(filters.endDate);
    }

    query += ` ORDER BY timestamp DESC LIMIT $${paramIndex}`;
    values.push(filters?.limit || 100);

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapViolationRow);
  }

  /**
   * Get policy statistics
   */
  async getStatistics(options?: {
    policyId?: string;
    startDate?: Date;
    endDate?: Date;
  }): Promise<{
    totalViolations: number;
    blockedCount: number;
    byPolicy: Array<{ policyId: string; policyName: string; count: number }>;
    byUser: Array<{ userId: string; count: number }>;
    timeline: Array<{ date: string; count: number }>;
  }> {
    let dateFilter = '';
    const values: unknown[] = [];
    let paramIndex = 1;

    if (options?.startDate) {
      dateFilter += ` AND timestamp >= $${paramIndex++}`;
      values.push(options.startDate);
    }
    if (options?.endDate) {
      dateFilter += ` AND timestamp <= $${paramIndex++}`;
      values.push(options.endDate);
    }

    // Total and blocked
    const totalResult = await this.db.query(
      `SELECT COUNT(*) as total, SUM(CASE WHEN blocked THEN 1 ELSE 0 END) as blocked
       FROM policy_violations WHERE 1=1 ${dateFilter}`,
      values
    );

    // By policy
    const byPolicyResult = await this.db.query(
      `SELECT policy_id, policy_name, COUNT(*) as count
       FROM policy_violations WHERE 1=1 ${dateFilter}
       GROUP BY policy_id, policy_name ORDER BY count DESC`,
      values
    );

    // By user
    const byUserResult = await this.db.query(
      `SELECT user_id, COUNT(*) as count
       FROM policy_violations WHERE 1=1 ${dateFilter}
       GROUP BY user_id ORDER BY count DESC LIMIT 20`,
      values
    );

    // Timeline
    const timelineResult = await this.db.query(
      `SELECT DATE_TRUNC('day', timestamp) as date, COUNT(*) as count
       FROM policy_violations WHERE 1=1 ${dateFilter}
       GROUP BY date ORDER BY date`,
      values
    );

    return {
      totalViolations: parseInt(totalResult.rows[0]?.total || '0', 10),
      blockedCount: parseInt(totalResult.rows[0]?.blocked || '0', 10),
      byPolicy: byPolicyResult.rows.map(row => ({
        policyId: row.policy_id,
        policyName: row.policy_name,
        count: parseInt(row.count, 10),
      })),
      byUser: byUserResult.rows.map(row => ({
        userId: row.user_id,
        count: parseInt(row.count, 10),
      })),
      timeline: timelineResult.rows.map(row => ({
        date: row.date.toISOString().split('T')[0],
        count: parseInt(row.count, 10),
      })),
    };
  }

  /**
   * Invalidate policy cache
   */
  private async invalidatePolicyCache(): Promise<void> {
    this.policyCache.clear();
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapPolicyRow(row: Record<string, unknown>): Policy {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      type: row.type as PolicyType,
      scope: row.scope as PolicyScope,
      enforcement: row.enforcement as PolicyEnforcement,
      rules: row.rules as Policy['rules'],
      targets: row.targets as Policy['targets'],
      exceptions: row.exceptions as Policy['exceptions'],
      version: row.version as number,
      status: row.status as Policy['status'],
      effectiveDate: row.effective_date as Date | undefined,
      expirationDate: row.expiration_date as Date | undefined,
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapViolationRow(row: Record<string, unknown>): PolicyViolation {
    return {
      id: row.id as string,
      policyId: row.policy_id as string,
      policyName: row.policy_name as string,
      ruleId: row.rule_id as string,
      userId: row.user_id as string,
      action: row.action as string,
      resource: row.resource as PolicyViolation['resource'],
      violationType: row.violation_type as PolicyViolation['violationType'],
      blocked: row.blocked as boolean,
      details: row.details as string,
      context: row.context as Record<string, unknown>,
      timestamp: row.timestamp as Date,
    };
  }
}
