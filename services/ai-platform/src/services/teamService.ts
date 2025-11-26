/**
 * Multi-Agent Team Service
 *
 * Manages multi-agent collaboration with various coordination patterns.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import { EventEmitter } from 'events';
import {
  AgentTeam,
  TeamExecution,
  CollaborationPattern,
  AgentConfig,
  AgentExecution,
  CreateAgentTeamInput,
  ExecuteTeamInput,
} from '../models/agent';
import { AgentService } from './agentService';

interface TeamMember {
  agentId: string;
  role: string;
  agent?: AgentConfig;
}

interface TeamMessage {
  id: string;
  fromAgent: string;
  toAgent?: string; // undefined = broadcast
  content: string;
  metadata?: Record<string, unknown>;
  timestamp: Date;
}

interface TeamState {
  executionId: string;
  currentPhase: string;
  messages: TeamMessage[];
  agentOutputs: Map<string, unknown>;
  sharedContext: Record<string, unknown>;
  votes?: Map<string, unknown>;
  consensus?: unknown;
}

export class TeamService extends EventEmitter {
  private db: Pool;
  private redis: RedisClientType;
  private agentService: AgentService;
  private activeExecutions: Map<string, TeamState> = new Map();

  constructor(db: Pool, redis: RedisClientType, agentService: AgentService) {
    super();
    this.db = db;
    this.redis = redis;
    this.agentService = agentService;
  }

  /**
   * Create a new agent team
   */
  async createTeam(input: CreateAgentTeamInput, userId: string): Promise<AgentTeam> {
    const team: AgentTeam = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      members: input.members,
      collaborationPattern: input.collaborationPattern || CollaborationPattern.SEQUENTIAL,
      coordinationConfig: {
        maxRounds: 10,
        timeout: 300000, // 5 minutes
        earlyTermination: true,
        votingThreshold: 0.5,
        ...input.coordinationConfig,
      },
      sharedMemory: input.sharedMemory || { enabled: false },
      status: 'active',
      createdAt: new Date(),
      updatedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO agent_teams (
        id, name, description, members, collaboration_pattern,
        coordination_config, shared_memory, status, created_at, updated_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)`,
      [
        team.id, team.name, team.description, JSON.stringify(team.members),
        team.collaborationPattern, JSON.stringify(team.coordinationConfig),
        JSON.stringify(team.sharedMemory), team.status, team.createdAt,
        team.updatedAt, team.createdBy,
      ]
    );

    return team;
  }

  /**
   * Get team by ID
   */
  async getTeam(teamId: string): Promise<AgentTeam | null> {
    const result = await this.db.query(
      `SELECT * FROM agent_teams WHERE id = $1`,
      [teamId]
    );

    if (result.rows.length === 0) return null;

    return this.mapTeamRow(result.rows[0]);
  }

  /**
   * List teams
   */
  async listTeams(filters?: {
    status?: AgentTeam['status'];
    pattern?: CollaborationPattern;
  }): Promise<AgentTeam[]> {
    let query = `SELECT * FROM agent_teams WHERE 1=1`;
    const values: unknown[] = [];
    let paramIndex = 1;

    if (filters?.status) {
      query += ` AND status = $${paramIndex++}`;
      values.push(filters.status);
    }
    if (filters?.pattern) {
      query += ` AND collaboration_pattern = $${paramIndex++}`;
      values.push(filters.pattern);
    }

    query += ` ORDER BY created_at DESC`;

    const result = await this.db.query(query, values);
    return result.rows.map(this.mapTeamRow);
  }

  /**
   * Execute team collaboration
   */
  async executeTeam(
    teamId: string,
    input: ExecuteTeamInput,
    userId: string
  ): Promise<TeamExecution> {
    const team = await this.getTeam(teamId);
    if (!team) throw new Error('Team not found');
    if (team.status !== 'active') {
      throw new Error('Team is not active');
    }

    // Load all team member agents
    const members: TeamMember[] = [];
    for (const member of team.members) {
      const agent = await this.agentService.getAgent(member.agentId);
      if (!agent) {
        throw new Error(`Agent ${member.agentId} not found`);
      }
      members.push({ ...member, agent });
    }

    const execution: TeamExecution = {
      id: uuidv4(),
      teamId,
      status: 'running',
      input: input.input,
      agentExecutions: [],
      messages: [],
      startedAt: new Date(),
      createdBy: userId,
    };

    await this.db.query(
      `INSERT INTO team_executions (
        id, team_id, status, input, agent_executions, messages,
        started_at, created_by
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        execution.id, teamId, execution.status, JSON.stringify(execution.input),
        JSON.stringify([]), JSON.stringify([]), execution.startedAt, userId,
      ]
    );

    // Initialize team state
    const state: TeamState = {
      executionId: execution.id,
      currentPhase: 'initialization',
      messages: [],
      agentOutputs: new Map(),
      sharedContext: input.context || {},
    };
    this.activeExecutions.set(execution.id, state);

    // Execute based on collaboration pattern
    try {
      let result: unknown;

      switch (team.collaborationPattern) {
        case CollaborationPattern.SEQUENTIAL:
          result = await this.executeSequential(team, members, execution, state, input);
          break;
        case CollaborationPattern.PARALLEL:
          result = await this.executeParallel(team, members, execution, state, input);
          break;
        case CollaborationPattern.HIERARCHICAL:
          result = await this.executeHierarchical(team, members, execution, state, input);
          break;
        case CollaborationPattern.DEBATE:
          result = await this.executeDebate(team, members, execution, state, input);
          break;
        case CollaborationPattern.CONSENSUS:
          result = await this.executeConsensus(team, members, execution, state, input);
          break;
        case CollaborationPattern.SUPERVISOR:
          result = await this.executeSupervisor(team, members, execution, state, input);
          break;
        default:
          throw new Error(`Unknown collaboration pattern: ${team.collaborationPattern}`);
      }

      execution.status = 'completed';
      execution.output = result;
      execution.completedAt = new Date();
    } catch (error) {
      execution.status = 'failed';
      execution.error = error instanceof Error ? error.message : String(error);
      execution.completedAt = new Date();
    }

    // Update execution record
    await this.db.query(
      `UPDATE team_executions SET
        status = $1, output = $2, error = $3, messages = $4,
        agent_executions = $5, completed_at = $6
      WHERE id = $7`,
      [
        execution.status, JSON.stringify(execution.output), execution.error,
        JSON.stringify(state.messages), JSON.stringify(execution.agentExecutions),
        execution.completedAt, execution.id,
      ]
    );

    this.activeExecutions.delete(execution.id);

    return execution;
  }

  /**
   * Sequential pattern: Agents execute one after another
   */
  private async executeSequential(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    let currentInput = input.input;

    for (const member of members) {
      state.currentPhase = `executing_${member.role}`;
      this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

      const agentExecution = await this.agentService.executeAgent(
        member.agentId,
        {
          input: currentInput,
          context: {
            ...state.sharedContext,
            previousOutputs: Object.fromEntries(state.agentOutputs),
            teamRole: member.role,
          },
        },
        execution.createdBy
      );

      execution.agentExecutions.push(agentExecution.id);
      state.agentOutputs.set(member.agentId, agentExecution.output);

      // Pass output to next agent
      currentInput = agentExecution.output;

      // Add message for communication record
      state.messages.push({
        id: uuidv4(),
        fromAgent: member.agentId,
        content: JSON.stringify(agentExecution.output),
        timestamp: new Date(),
      });
    }

    return currentInput;
  }

  /**
   * Parallel pattern: All agents execute simultaneously
   */
  private async executeParallel(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    state.currentPhase = 'parallel_execution';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    const executions = await Promise.all(
      members.map(async (member) => {
        const agentExecution = await this.agentService.executeAgent(
          member.agentId,
          {
            input: input.input,
            context: {
              ...state.sharedContext,
              teamRole: member.role,
            },
          },
          execution.createdBy
        );

        execution.agentExecutions.push(agentExecution.id);
        state.agentOutputs.set(member.agentId, agentExecution.output);

        state.messages.push({
          id: uuidv4(),
          fromAgent: member.agentId,
          content: JSON.stringify(agentExecution.output),
          timestamp: new Date(),
        });

        return {
          agentId: member.agentId,
          role: member.role,
          output: agentExecution.output,
        };
      })
    );

    // Aggregate results
    state.currentPhase = 'aggregation';
    return {
      results: executions,
      aggregatedAt: new Date(),
    };
  }

  /**
   * Hierarchical pattern: Supervisor delegates to workers
   */
  private async executeHierarchical(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    const supervisor = members.find(m => m.role === 'supervisor');
    const workers = members.filter(m => m.role !== 'supervisor');

    if (!supervisor) {
      throw new Error('Hierarchical pattern requires a supervisor agent');
    }

    state.currentPhase = 'planning';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    // Step 1: Supervisor creates plan
    const planExecution = await this.agentService.executeAgent(
      supervisor.agentId,
      {
        input: input.input,
        context: {
          ...state.sharedContext,
          mode: 'planning',
          availableWorkers: workers.map(w => ({
            agentId: w.agentId,
            role: w.role,
            capabilities: w.agent?.capabilities,
          })),
        },
      },
      execution.createdBy
    );

    execution.agentExecutions.push(planExecution.id);
    const plan = planExecution.output as { tasks: Array<{ workerId: string; task: string }> };

    state.messages.push({
      id: uuidv4(),
      fromAgent: supervisor.agentId,
      content: JSON.stringify({ type: 'plan', plan }),
      timestamp: new Date(),
    });

    // Step 2: Delegate tasks to workers
    state.currentPhase = 'delegation';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    const workerResults: Record<string, unknown> = {};

    if (plan?.tasks) {
      for (const task of plan.tasks) {
        const worker = workers.find(w => w.agentId === task.workerId);
        if (!worker) continue;

        const workerExecution = await this.agentService.executeAgent(
          worker.agentId,
          {
            input: task.task,
            context: {
              ...state.sharedContext,
              delegatedBy: supervisor.agentId,
              originalInput: input.input,
            },
          },
          execution.createdBy
        );

        execution.agentExecutions.push(workerExecution.id);
        workerResults[worker.agentId] = workerExecution.output;
        state.agentOutputs.set(worker.agentId, workerExecution.output);

        state.messages.push({
          id: uuidv4(),
          fromAgent: worker.agentId,
          toAgent: supervisor.agentId,
          content: JSON.stringify(workerExecution.output),
          timestamp: new Date(),
        });
      }
    }

    // Step 3: Supervisor synthesizes results
    state.currentPhase = 'synthesis';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    const synthesisExecution = await this.agentService.executeAgent(
      supervisor.agentId,
      {
        input: input.input,
        context: {
          ...state.sharedContext,
          mode: 'synthesis',
          workerResults,
        },
      },
      execution.createdBy
    );

    execution.agentExecutions.push(synthesisExecution.id);

    return synthesisExecution.output;
  }

  /**
   * Debate pattern: Agents argue positions and refine through rounds
   */
  private async executeDebate(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    const maxRounds = team.coordinationConfig?.maxRounds || 5;
    const positions: Map<string, unknown[]> = new Map();

    // Initialize positions for each agent
    members.forEach(m => positions.set(m.agentId, []));

    // Initial positions
    state.currentPhase = 'initial_positions';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    for (const member of members) {
      const positionExecution = await this.agentService.executeAgent(
        member.agentId,
        {
          input: input.input,
          context: {
            ...state.sharedContext,
            mode: 'initial_position',
            teamRole: member.role,
          },
        },
        execution.createdBy
      );

      execution.agentExecutions.push(positionExecution.id);
      positions.get(member.agentId)?.push(positionExecution.output);

      state.messages.push({
        id: uuidv4(),
        fromAgent: member.agentId,
        content: JSON.stringify({ round: 0, position: positionExecution.output }),
        metadata: { round: 0, type: 'position' },
        timestamp: new Date(),
      });
    }

    // Debate rounds
    for (let round = 1; round <= maxRounds; round++) {
      state.currentPhase = `debate_round_${round}`;
      this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

      const roundPositions: Map<string, unknown> = new Map();

      for (const member of members) {
        const otherPositions: Record<string, unknown[]> = {};
        positions.forEach((pos, agentId) => {
          if (agentId !== member.agentId) {
            otherPositions[agentId] = pos;
          }
        });

        const debateExecution = await this.agentService.executeAgent(
          member.agentId,
          {
            input: input.input,
            context: {
              ...state.sharedContext,
              mode: 'debate',
              round,
              myPreviousPositions: positions.get(member.agentId),
              otherPositions,
            },
          },
          execution.createdBy
        );

        execution.agentExecutions.push(debateExecution.id);
        roundPositions.set(member.agentId, debateExecution.output);

        state.messages.push({
          id: uuidv4(),
          fromAgent: member.agentId,
          content: JSON.stringify({ round, position: debateExecution.output }),
          metadata: { round, type: 'debate' },
          timestamp: new Date(),
        });
      }

      // Update positions
      roundPositions.forEach((pos, agentId) => {
        positions.get(agentId)?.push(pos);
      });

      // Check for convergence
      if (team.coordinationConfig?.earlyTermination) {
        const converged = this.checkDebateConvergence(roundPositions);
        if (converged) {
          break;
        }
      }
    }

    // Final synthesis
    state.currentPhase = 'final_synthesis';

    // If there's a designated synthesizer, use them
    const synthesizer = members.find(m => m.role === 'synthesizer') || members[0];

    const finalExecution = await this.agentService.executeAgent(
      synthesizer.agentId,
      {
        input: input.input,
        context: {
          ...state.sharedContext,
          mode: 'synthesize_debate',
          allPositions: Object.fromEntries(positions),
        },
      },
      execution.createdBy
    );

    execution.agentExecutions.push(finalExecution.id);

    return {
      finalPosition: finalExecution.output,
      debateHistory: Object.fromEntries(positions),
      rounds: Math.min(maxRounds, positions.get(members[0].agentId)?.length || 0),
    };
  }

  /**
   * Consensus pattern: Agents vote and iterate until agreement
   */
  private async executeConsensus(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    const maxRounds = team.coordinationConfig?.maxRounds || 5;
    const threshold = team.coordinationConfig?.votingThreshold || 0.66;

    state.votes = new Map();

    // Initial proposals
    state.currentPhase = 'proposals';
    this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

    const proposals: Map<string, unknown> = new Map();

    for (const member of members) {
      const proposalExecution = await this.agentService.executeAgent(
        member.agentId,
        {
          input: input.input,
          context: {
            ...state.sharedContext,
            mode: 'propose',
            teamRole: member.role,
          },
        },
        execution.createdBy
      );

      execution.agentExecutions.push(proposalExecution.id);
      proposals.set(member.agentId, proposalExecution.output);

      state.messages.push({
        id: uuidv4(),
        fromAgent: member.agentId,
        content: JSON.stringify({ type: 'proposal', proposal: proposalExecution.output }),
        timestamp: new Date(),
      });
    }

    // Voting rounds
    for (let round = 1; round <= maxRounds; round++) {
      state.currentPhase = `voting_round_${round}`;
      this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

      const votes: Map<string, string> = new Map(); // agentId -> proposalId they vote for

      for (const member of members) {
        const voteExecution = await this.agentService.executeAgent(
          member.agentId,
          {
            input: input.input,
            context: {
              ...state.sharedContext,
              mode: 'vote',
              proposals: Object.fromEntries(proposals),
              round,
            },
          },
          execution.createdBy
        );

        execution.agentExecutions.push(voteExecution.id);
        const vote = voteExecution.output as { selectedProposal: string; reasoning: string };
        votes.set(member.agentId, vote.selectedProposal);

        state.messages.push({
          id: uuidv4(),
          fromAgent: member.agentId,
          content: JSON.stringify({ type: 'vote', round, vote }),
          timestamp: new Date(),
        });
      }

      // Count votes
      const voteCounts = new Map<string, number>();
      votes.forEach((proposalId) => {
        voteCounts.set(proposalId, (voteCounts.get(proposalId) || 0) + 1);
      });

      // Check for consensus
      const totalVotes = members.length;
      for (const [proposalId, count] of voteCounts) {
        if (count / totalVotes >= threshold) {
          state.consensus = proposals.get(proposalId);
          return {
            consensus: state.consensus,
            proposedBy: proposalId,
            votesReceived: count,
            totalVotes,
            rounds: round,
          };
        }
      }

      // If no consensus, refine proposals based on votes
      state.currentPhase = `refinement_round_${round}`;

      for (const member of members) {
        const refinementExecution = await this.agentService.executeAgent(
          member.agentId,
          {
            input: input.input,
            context: {
              ...state.sharedContext,
              mode: 'refine',
              myProposal: proposals.get(member.agentId),
              allProposals: Object.fromEntries(proposals),
              voteResults: Object.fromEntries(voteCounts),
              round,
            },
          },
          execution.createdBy
        );

        execution.agentExecutions.push(refinementExecution.id);
        proposals.set(member.agentId, refinementExecution.output);
      }
    }

    // No consensus reached - return best effort
    const voteCounts = new Map<string, number>();
    state.votes?.forEach((proposalId: unknown) => {
      const id = String(proposalId);
      voteCounts.set(id, (voteCounts.get(id) || 0) + 1);
    });

    let bestProposal = members[0].agentId;
    let maxVotes = 0;
    voteCounts.forEach((count, proposalId) => {
      if (count > maxVotes) {
        maxVotes = count;
        bestProposal = proposalId;
      }
    });

    return {
      consensus: null,
      bestEffort: proposals.get(bestProposal),
      proposedBy: bestProposal,
      votesReceived: maxVotes,
      totalVotes: members.length,
      rounds: maxRounds,
      message: 'Consensus not reached within maximum rounds',
    };
  }

  /**
   * Supervisor pattern: One agent supervises and coordinates others dynamically
   */
  private async executeSupervisor(
    team: AgentTeam,
    members: TeamMember[],
    execution: TeamExecution,
    state: TeamState,
    input: ExecuteTeamInput
  ): Promise<unknown> {
    const supervisor = members.find(m => m.role === 'supervisor');
    const workers = members.filter(m => m.role !== 'supervisor');

    if (!supervisor) {
      throw new Error('Supervisor pattern requires a supervisor agent');
    }

    const maxIterations = team.coordinationConfig?.maxRounds || 10;
    let iteration = 0;
    let complete = false;
    let result: unknown;

    while (!complete && iteration < maxIterations) {
      iteration++;
      state.currentPhase = `supervisor_iteration_${iteration}`;
      this.emit('phase_change', { executionId: execution.id, phase: state.currentPhase });

      // Supervisor decides next action
      const supervisionExecution = await this.agentService.executeAgent(
        supervisor.agentId,
        {
          input: input.input,
          context: {
            ...state.sharedContext,
            mode: 'supervise',
            iteration,
            availableWorkers: workers.map(w => ({
              agentId: w.agentId,
              role: w.role,
              capabilities: w.agent?.capabilities,
            })),
            previousResults: Object.fromEntries(state.agentOutputs),
            messageHistory: state.messages,
          },
        },
        execution.createdBy
      );

      execution.agentExecutions.push(supervisionExecution.id);

      const decision = supervisionExecution.output as {
        action: 'delegate' | 'complete' | 'refine';
        target?: string;
        task?: string;
        result?: unknown;
      };

      state.messages.push({
        id: uuidv4(),
        fromAgent: supervisor.agentId,
        content: JSON.stringify({ iteration, decision }),
        metadata: { iteration, type: 'supervision' },
        timestamp: new Date(),
      });

      switch (decision.action) {
        case 'delegate':
          if (decision.target && decision.task) {
            const worker = workers.find(w => w.agentId === decision.target);
            if (worker) {
              const workerExecution = await this.agentService.executeAgent(
                worker.agentId,
                {
                  input: decision.task,
                  context: {
                    ...state.sharedContext,
                    delegatedBy: supervisor.agentId,
                    iteration,
                    originalInput: input.input,
                  },
                },
                execution.createdBy
              );

              execution.agentExecutions.push(workerExecution.id);
              state.agentOutputs.set(worker.agentId, workerExecution.output);

              state.messages.push({
                id: uuidv4(),
                fromAgent: worker.agentId,
                toAgent: supervisor.agentId,
                content: JSON.stringify(workerExecution.output),
                metadata: { iteration, type: 'worker_result' },
                timestamp: new Date(),
              });
            }
          }
          break;

        case 'complete':
          complete = true;
          result = decision.result;
          break;

        case 'refine':
          // Supervisor wants to refine the approach
          state.sharedContext.refinement = decision;
          break;
      }
    }

    if (!complete) {
      // Force completion after max iterations
      const finalExecution = await this.agentService.executeAgent(
        supervisor.agentId,
        {
          input: input.input,
          context: {
            ...state.sharedContext,
            mode: 'force_complete',
            allResults: Object.fromEntries(state.agentOutputs),
          },
        },
        execution.createdBy
      );

      execution.agentExecutions.push(finalExecution.id);
      result = finalExecution.output;
    }

    return {
      result,
      iterations: iteration,
      completed: complete,
    };
  }

  /**
   * Check if debate has converged
   */
  private checkDebateConvergence(positions: Map<string, unknown>): boolean {
    const positionStrings = Array.from(positions.values()).map(p => JSON.stringify(p));
    const uniquePositions = new Set(positionStrings);

    // Consider converged if all positions are similar (in production, use semantic similarity)
    return uniquePositions.size === 1;
  }

  /**
   * Get team execution by ID
   */
  async getExecution(executionId: string): Promise<TeamExecution | null> {
    const result = await this.db.query(
      `SELECT * FROM team_executions WHERE id = $1`,
      [executionId]
    );

    if (result.rows.length === 0) return null;

    return this.mapExecutionRow(result.rows[0]);
  }

  /**
   * List team executions
   */
  async listExecutions(teamId: string, limit = 20): Promise<TeamExecution[]> {
    const result = await this.db.query(
      `SELECT * FROM team_executions WHERE team_id = $1 ORDER BY started_at DESC LIMIT $2`,
      [teamId, limit]
    );

    return result.rows.map(this.mapExecutionRow);
  }

  /**
   * Cancel team execution
   */
  async cancelExecution(executionId: string): Promise<void> {
    const state = this.activeExecutions.get(executionId);
    if (!state) {
      throw new Error('Execution not found or already completed');
    }

    await this.db.query(
      `UPDATE team_executions SET status = 'cancelled', completed_at = NOW() WHERE id = $1`,
      [executionId]
    );

    this.activeExecutions.delete(executionId);
    this.emit('execution_cancelled', { executionId });
  }

  /**
   * Update team configuration
   */
  async updateTeam(
    teamId: string,
    updates: Partial<Pick<AgentTeam, 'name' | 'description' | 'members' | 'coordinationConfig'>>
  ): Promise<AgentTeam> {
    const team = await this.getTeam(teamId);
    if (!team) throw new Error('Team not found');

    const updatedTeam = { ...team, ...updates, updatedAt: new Date() };

    await this.db.query(
      `UPDATE agent_teams SET
        name = $1, description = $2, members = $3,
        coordination_config = $4, updated_at = $5
      WHERE id = $6`,
      [
        updatedTeam.name, updatedTeam.description, JSON.stringify(updatedTeam.members),
        JSON.stringify(updatedTeam.coordinationConfig), updatedTeam.updatedAt, teamId,
      ]
    );

    return updatedTeam;
  }

  /**
   * Archive team
   */
  async archiveTeam(teamId: string): Promise<void> {
    await this.db.query(
      `UPDATE agent_teams SET status = 'archived', updated_at = NOW() WHERE id = $1`,
      [teamId]
    );
  }

  // ===========================================
  // Helpers
  // ===========================================

  private mapTeamRow(row: Record<string, unknown>): AgentTeam {
    return {
      id: row.id as string,
      name: row.name as string,
      description: row.description as string | undefined,
      members: row.members as AgentTeam['members'],
      collaborationPattern: row.collaboration_pattern as CollaborationPattern,
      coordinationConfig: row.coordination_config as AgentTeam['coordinationConfig'],
      sharedMemory: row.shared_memory as AgentTeam['sharedMemory'],
      status: row.status as AgentTeam['status'],
      createdAt: row.created_at as Date,
      updatedAt: row.updated_at as Date,
      createdBy: row.created_by as string,
    };
  }

  private mapExecutionRow(row: Record<string, unknown>): TeamExecution {
    return {
      id: row.id as string,
      teamId: row.team_id as string,
      status: row.status as TeamExecution['status'],
      input: row.input as unknown,
      output: row.output as unknown,
      error: row.error as string | undefined,
      agentExecutions: row.agent_executions as string[],
      messages: row.messages as TeamExecution['messages'],
      startedAt: row.started_at as Date,
      completedAt: row.completed_at as Date | undefined,
      createdBy: row.created_by as string,
    };
  }
}
