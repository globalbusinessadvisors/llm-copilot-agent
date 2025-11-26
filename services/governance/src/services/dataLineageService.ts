/**
 * Data Lineage Service
 *
 * Manages data lineage tracking, providing visibility into data flow and transformations.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  LineageNode,
  LineageEdge,
  DataLineageGraph,
  LineageNodeType,
  LineageEdgeType,
  CreateLineageNodeInput,
  CreateLineageEdgeInput,
} from '../models/governance';

interface LineageImpactAnalysis {
  affectedNodes: LineageNode[];
  affectedEdges: LineageEdge[];
  impactLevel: 'low' | 'medium' | 'high' | 'critical';
  affectedSystems: string[];
}

export class DataLineageService {
  private db: Pool;
  private redis: RedisClientType;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Node Management
  // ===========================================

  /**
   * Create a lineage node
   */
  async createNode(input: CreateLineageNodeInput): Promise<LineageNode> {
    const node: LineageNode = {
      id: uuidv4(),
      type: input.type,
      name: input.name,
      description: input.description,
      source: input.source,
      schema: input.schema,
      metadata: input.metadata,
      tags: input.tags,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO lineage_nodes (
        id, type, name, description, source, schema, metadata, tags,
        created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        node.id, node.type, node.name, node.description, JSON.stringify(node.source),
        JSON.stringify(node.schema), JSON.stringify(node.metadata), node.tags,
        node.createdAt, node.updatedAt,
      ]
    );

    // Invalidate cache
    await this.invalidateNodeCache(node.id);

    return node;
  }

  /**
   * Get node by ID
   */
  async getNode(nodeId: string): Promise<LineageNode | null> {
    // Check cache first
    const cached = await this.redis.get(`lineage:node:${nodeId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    const result = await this.db.query(
      `SELECT * FROM lineage_nodes WHERE id = $1`,
      [nodeId]
    );

    if (result.rows.length === 0) return null;

    const node = this.mapNodeRow(result.rows[0]);

    // Cache for 5 minutes
    await this.redis.setEx(`lineage:node:${nodeId}`, 300, JSON.stringify(node));

    return node;
  }

  /**
   * List nodes
   */
  async listNodes(filters?: {
    type?: LineageNodeType;
    system?: string;
    tags?: string[];
  }): Promise<LineageNode[]> {
    let query = `SELECT * FROM lineage_nodes WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }
    if (filters?.system) {
      query += ` AND source->>'system' = $${paramIndex++}`;
      values.push(filters.system);
    }
    if (filters?.tags && filters.tags.length > 0) {
      query += ` AND tags && $${paramIndex++}`;
      values.push(filters.tags);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapNodeRow);
  }

  /**
   * Update node
   */
  async updateNode(
    nodeId: string,
    updates: Partial<CreateLineageNodeInput>
  ): Promise<LineageNode> {
    const node = await this.getNode(nodeId);
    if (!node) throw new Error('Node not found');

    const updatedNode = {
      ...node,
      ...updates,
      updatedAt: new Date(),
    };

    await this.db.query(
      `UPDATE lineage_nodes SET
        name = $1, description = $2, source = $3, schema = $4,
        metadata = $5, tags = $6, updated_at = $7
      WHERE id = $8`,
      [
        updatedNode.name, updatedNode.description, JSON.stringify(updatedNode.source),
        JSON.stringify(updatedNode.schema), JSON.stringify(updatedNode.metadata),
        updatedNode.tags, updatedNode.updatedAt, nodeId,
      ]
    );

    await this.invalidateNodeCache(nodeId);

    return updatedNode;
  }

  /**
   * Delete node
   */
  async deleteNode(nodeId: string): Promise<void> {
    // First delete associated edges
    await this.db.query(
      `DELETE FROM lineage_edges WHERE source_node_id = $1 OR target_node_id = $1`,
      [nodeId]
    );

    await this.db.query(`DELETE FROM lineage_nodes WHERE id = $1`, [nodeId]);
    await this.invalidateNodeCache(nodeId);
  }

  // ===========================================
  // Edge Management
  // ===========================================

  /**
   * Create a lineage edge
   */
  async createEdge(input: CreateLineageEdgeInput): Promise<LineageEdge> {
    // Verify both nodes exist
    const [sourceNode, targetNode] = await Promise.all([
      this.getNode(input.sourceNodeId),
      this.getNode(input.targetNodeId),
    ]);

    if (!sourceNode) throw new Error('Source node not found');
    if (!targetNode) throw new Error('Target node not found');

    // Check for circular dependency
    const wouldCreateCycle = await this.checkCycle(input.sourceNodeId, input.targetNodeId);
    if (wouldCreateCycle) {
      throw new Error('Creating this edge would create a circular dependency');
    }

    const edge: LineageEdge = {
      id: uuidv4(),
      type: input.type,
      sourceNodeId: input.sourceNodeId,
      targetNodeId: input.targetNodeId,
      transformation: input.transformation,
      metadata: input.metadata,
      createdAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO lineage_edges (
        id, type, source_node_id, target_node_id, transformation, metadata, created_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)`,
      [
        edge.id, edge.type, edge.sourceNodeId, edge.targetNodeId,
        JSON.stringify(edge.transformation), JSON.stringify(edge.metadata),
        edge.createdAt,
      ]
    );

    // Invalidate graph cache for affected nodes
    await this.invalidateGraphCache(input.sourceNodeId);
    await this.invalidateGraphCache(input.targetNodeId);

    return edge;
  }

  /**
   * Get edge by ID
   */
  async getEdge(edgeId: string): Promise<LineageEdge | null> {
    const result = await this.db.query(
      `SELECT * FROM lineage_edges WHERE id = $1`,
      [edgeId]
    );

    if (result.rows.length === 0) return null;

    return this.mapEdgeRow(result.rows[0]);
  }

  /**
   * List edges for a node
   */
  async listEdges(filters?: {
    sourceNodeId?: string;
    targetNodeId?: string;
    type?: LineageEdgeType;
  }): Promise<LineageEdge[]> {
    let query = `SELECT * FROM lineage_edges WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.sourceNodeId) {
      query += ` AND source_node_id = $${paramIndex++}`;
      values.push(filters.sourceNodeId);
    }
    if (filters?.targetNodeId) {
      query += ` AND target_node_id = $${paramIndex++}`;
      values.push(filters.targetNodeId);
    }
    if (filters?.type) {
      query += ` AND type = $${paramIndex++}`;
      values.push(filters.type);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapEdgeRow);
  }

  /**
   * Delete edge
   */
  async deleteEdge(edgeId: string): Promise<void> {
    const edge = await this.getEdge(edgeId);
    if (!edge) throw new Error('Edge not found');

    await this.db.query(`DELETE FROM lineage_edges WHERE id = $1`, [edgeId]);

    await this.invalidateGraphCache(edge.sourceNodeId);
    await this.invalidateGraphCache(edge.targetNodeId);
  }

  /**
   * Check if creating an edge would cause a cycle
   */
  private async checkCycle(sourceId: string, targetId: string): Promise<boolean> {
    // Use recursive CTE to detect cycles
    const result = await this.db.query(
      `WITH RECURSIVE lineage_path AS (
        SELECT source_node_id, target_node_id, ARRAY[source_node_id] as path
        FROM lineage_edges
        WHERE source_node_id = $1

        UNION ALL

        SELECT e.source_node_id, e.target_node_id, lp.path || e.source_node_id
        FROM lineage_edges e
        JOIN lineage_path lp ON e.source_node_id = lp.target_node_id
        WHERE NOT e.source_node_id = ANY(lp.path)
      )
      SELECT 1 FROM lineage_path WHERE target_node_id = $2 LIMIT 1`,
      [targetId, sourceId]
    );

    return result.rows.length > 0;
  }

  // ===========================================
  // Graph Operations
  // ===========================================

  /**
   * Get lineage graph for a node
   */
  async getLineageGraph(
    nodeId: string,
    options?: {
      direction?: 'upstream' | 'downstream' | 'both';
      depth?: number;
    }
  ): Promise<DataLineageGraph> {
    const direction = options?.direction || 'both';
    const depth = options?.depth || 10;

    const nodes: Map<string, LineageNode> = new Map();
    const edges: LineageEdge[] = [];

    // Get root node
    const rootNode = await this.getNode(nodeId);
    if (!rootNode) throw new Error('Node not found');
    nodes.set(rootNode.id, rootNode);

    // Get upstream nodes (sources)
    if (direction === 'upstream' || direction === 'both') {
      await this.traverseUpstream(nodeId, nodes, edges, depth);
    }

    // Get downstream nodes (targets)
    if (direction === 'downstream' || direction === 'both') {
      await this.traverseDownstream(nodeId, nodes, edges, depth);
    }

    return {
      id: uuidv4(),
      name: `Lineage for ${rootNode.name}`,
      description: `Data lineage graph centered on ${rootNode.name}`,
      nodes: Array.from(nodes.values()),
      edges,
      rootNodeId: nodeId,
      createdAt: new Date(),
      updatedAt: new Date(),
    };
  }

  /**
   * Traverse upstream (sources)
   */
  private async traverseUpstream(
    nodeId: string,
    nodes: Map<string, LineageNode>,
    edges: LineageEdge[],
    depth: number,
    currentDepth = 0
  ): Promise<void> {
    if (currentDepth >= depth) return;

    const upstreamEdges = await this.listEdges({ targetNodeId: nodeId });

    for (const edge of upstreamEdges) {
      if (!edges.find(e => e.id === edge.id)) {
        edges.push(edge);
      }

      if (!nodes.has(edge.sourceNodeId)) {
        const sourceNode = await this.getNode(edge.sourceNodeId);
        if (sourceNode) {
          nodes.set(sourceNode.id, sourceNode);
          await this.traverseUpstream(edge.sourceNodeId, nodes, edges, depth, currentDepth + 1);
        }
      }
    }
  }

  /**
   * Traverse downstream (targets)
   */
  private async traverseDownstream(
    nodeId: string,
    nodes: Map<string, LineageNode>,
    edges: LineageEdge[],
    depth: number,
    currentDepth = 0
  ): Promise<void> {
    if (currentDepth >= depth) return;

    const downstreamEdges = await this.listEdges({ sourceNodeId: nodeId });

    for (const edge of downstreamEdges) {
      if (!edges.find(e => e.id === edge.id)) {
        edges.push(edge);
      }

      if (!nodes.has(edge.targetNodeId)) {
        const targetNode = await this.getNode(edge.targetNodeId);
        if (targetNode) {
          nodes.set(targetNode.id, targetNode);
          await this.traverseDownstream(edge.targetNodeId, nodes, edges, depth, currentDepth + 1);
        }
      }
    }
  }

  /**
   * Perform impact analysis for a node
   */
  async analyzeImpact(nodeId: string): Promise<LineageImpactAnalysis> {
    const graph = await this.getLineageGraph(nodeId, { direction: 'downstream', depth: 20 });

    // Get all downstream nodes (excluding the source node)
    const affectedNodes = graph.nodes.filter(n => n.id !== nodeId);
    const affectedEdges = graph.edges;

    // Get unique systems
    const systems = new Set<string>();
    affectedNodes.forEach(node => {
      systems.add(node.source.system);
    });

    // Determine impact level
    let impactLevel: LineageImpactAnalysis['impactLevel'];
    if (affectedNodes.length === 0) {
      impactLevel = 'low';
    } else if (affectedNodes.length <= 5) {
      impactLevel = 'medium';
    } else if (affectedNodes.length <= 20) {
      impactLevel = 'high';
    } else {
      impactLevel = 'critical';
    }

    // Check for critical systems
    const criticalSystems = ['production', 'customer_data', 'billing'];
    if (affectedNodes.some(n => criticalSystems.includes(n.source.system))) {
      if (impactLevel !== 'critical') {
        impactLevel = impactLevel === 'low' ? 'medium' : impactLevel === 'medium' ? 'high' : 'critical';
      }
    }

    return {
      affectedNodes,
      affectedEdges,
      impactLevel,
      affectedSystems: Array.from(systems),
    };
  }

  /**
   * Find path between two nodes
   */
  async findPath(sourceId: string, targetId: string): Promise<{
    exists: boolean;
    path: LineageNode[];
    edges: LineageEdge[];
  } | null> {
    const result = await this.db.query(
      `WITH RECURSIVE lineage_path AS (
        SELECT
          source_node_id,
          target_node_id,
          id as edge_id,
          ARRAY[source_node_id] as node_path,
          ARRAY[id] as edge_path
        FROM lineage_edges
        WHERE source_node_id = $1

        UNION ALL

        SELECT
          e.source_node_id,
          e.target_node_id,
          e.id,
          lp.node_path || e.source_node_id,
          lp.edge_path || e.id
        FROM lineage_edges e
        JOIN lineage_path lp ON e.source_node_id = lp.target_node_id
        WHERE NOT e.source_node_id = ANY(lp.node_path)
          AND array_length(lp.node_path, 1) < 20
      )
      SELECT node_path || target_node_id as full_path, edge_path
      FROM lineage_path
      WHERE target_node_id = $2
      LIMIT 1`,
      [sourceId, targetId]
    );

    if (result.rows.length === 0) {
      return { exists: false, path: [], edges: [] };
    }

    const nodeIds: string[] = result.rows[0].full_path;
    const edgeIds: string[] = result.rows[0].edge_path;

    // Fetch nodes and edges
    const nodes = await Promise.all(nodeIds.map(id => this.getNode(id)));
    const edges = await Promise.all(edgeIds.map(id => this.getEdge(id)));

    return {
      exists: true,
      path: nodes.filter((n): n is LineageNode => n !== null),
      edges: edges.filter((e): e is LineageEdge => e !== null),
    };
  }

  // ===========================================
  // Search and Discovery
  // ===========================================

  /**
   * Search nodes by keyword
   */
  async searchNodes(query: string, limit = 20): Promise<LineageNode[]> {
    const result = await this.db.query(
      `SELECT * FROM lineage_nodes
       WHERE name ILIKE $1
          OR description ILIKE $1
          OR source->>'system' ILIKE $1
          OR EXISTS (SELECT 1 FROM unnest(tags) t WHERE t ILIKE $1)
       LIMIT $2`,
      [`%${query}%`, limit]
    );

    return result.rows.map(this.mapNodeRow);
  }

  /**
   * Find related nodes (connected directly or through transformations)
   */
  async findRelatedNodes(nodeId: string, limit = 10): Promise<LineageNode[]> {
    const result = await this.db.query(
      `SELECT DISTINCT n.*
       FROM lineage_nodes n
       JOIN lineage_edges e ON n.id = e.source_node_id OR n.id = e.target_node_id
       WHERE (e.source_node_id = $1 OR e.target_node_id = $1)
         AND n.id != $1
       LIMIT $2`,
      [nodeId, limit]
    );

    return result.rows.map(this.mapNodeRow);
  }

  // ===========================================
  // Statistics
  // ===========================================

  /**
   * Get lineage statistics
   */
  async getStatistics(): Promise<{
    totalNodes: number;
    totalEdges: number;
    nodesByType: Record<LineageNodeType, number>;
    edgesByType: Record<LineageEdgeType, number>;
    systemStats: Array<{ system: string; nodeCount: number }>;
    orphanedNodes: number;
  }> {
    // Total counts
    const [nodesResult, edgesResult] = await Promise.all([
      this.db.query(`SELECT COUNT(*) FROM lineage_nodes`),
      this.db.query(`SELECT COUNT(*) FROM lineage_edges`),
    ]);

    // By node type
    const nodeTypeResult = await this.db.query(
      `SELECT type, COUNT(*) as count FROM lineage_nodes GROUP BY type`
    );
    const nodesByType: Record<string, number> = {};
    nodeTypeResult.rows.forEach(row => {
      nodesByType[row.type] = parseInt(row.count, 10);
    });

    // By edge type
    const edgeTypeResult = await this.db.query(
      `SELECT type, COUNT(*) as count FROM lineage_edges GROUP BY type`
    );
    const edgesByType: Record<string, number> = {};
    edgeTypeResult.rows.forEach(row => {
      edgesByType[row.type] = parseInt(row.count, 10);
    });

    // By system
    const systemResult = await this.db.query(
      `SELECT source->>'system' as system, COUNT(*) as count
       FROM lineage_nodes
       GROUP BY source->>'system'
       ORDER BY count DESC`
    );
    const systemStats = systemResult.rows.map(row => ({
      system: row.system,
      nodeCount: parseInt(row.count, 10),
    }));

    // Orphaned nodes (no edges)
    const orphanedResult = await this.db.query(
      `SELECT COUNT(*) FROM lineage_nodes n
       WHERE NOT EXISTS (
         SELECT 1 FROM lineage_edges e
         WHERE e.source_node_id = n.id OR e.target_node_id = n.id
       )`
    );

    return {
      totalNodes: parseInt(nodesResult.rows[0]?.count || '0', 10),
      totalEdges: parseInt(edgesResult.rows[0]?.count || '0', 10),
      nodesByType: nodesByType as Record<LineageNodeType, number>,
      edgesByType: edgesByType as Record<LineageEdgeType, number>,
      systemStats,
      orphanedNodes: parseInt(orphanedResult.rows[0]?.count || '0', 10),
    };
  }

  // ===========================================
  // Cache Management
  // ===========================================

  private async invalidateNodeCache(nodeId: string): Promise<void> {
    await this.redis.del(`lineage:node:${nodeId}`);
  }

  private async invalidateGraphCache(nodeId: string): Promise<void> {
    // In production, would also invalidate graph cache for affected nodes
    await this.redis.del(`lineage:graph:${nodeId}`);
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapNodeRow(row: Record<string, unknown>): LineageNode {
    return {
      id: row.id as string,
      type: row.type as LineageNodeType,
      name: row.name as string,
      description: row.description as string | undefined,
      source: row.source as LineageNode['source'],
      schema: row.schema as LineageNode['schema'],
      metadata: row.metadata as Record<string, unknown>,
      tags: row.tags as string[],
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
    };
  }

  private mapEdgeRow(row: Record<string, unknown>): LineageEdge {
    return {
      id: row.id as string,
      type: row.type as LineageEdgeType,
      sourceNodeId: row.source_node_id as string,
      targetNodeId: row.target_node_id as string,
      transformation: row.transformation as LineageEdge['transformation'],
      metadata: row.metadata as Record<string, unknown>,
      createdAt: row.created_at as Date,
    };
  }
}
