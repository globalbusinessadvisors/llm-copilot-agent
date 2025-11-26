/**
 * On-Call Service
 *
 * Manages on-call schedules, rotations, and overrides.
 */

import { Pool } from 'pg';
import { RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';
import {
  OnCallSchedule,
  OnCallUser,
  AlertChannel,
} from '../models/alert';

export interface CreateScheduleInput {
  name: string;
  description?: string;
  timezone?: string;
  rotations: Array<{
    name: string;
    type: 'daily' | 'weekly' | 'custom';
    startTime: string;
    handoffTime: string;
    users: string[];
    restrictions?: Array<{
      type: 'time_of_day' | 'day_of_week';
      startTime?: string;
      endTime?: string;
      daysOfWeek?: number[];
    }>;
  }>;
}

export interface CreateOverrideInput {
  scheduleId: string;
  userId: string;
  startAt: Date;
  endAt: Date;
  reason?: string;
}

export interface CreateUserInput {
  name: string;
  email: string;
  phone?: string;
  slackUserId?: string;
  notificationPreferences?: {
    email?: boolean;
    slack?: boolean;
    sms?: boolean;
    phone?: boolean;
  };
}

export interface CurrentOnCall {
  scheduleId: string;
  scheduleName: string;
  user: OnCallUser;
  startedAt: Date;
  endsAt: Date;
  isOverride: boolean;
}

export class OnCallService {
  private db: Pool;
  private redis: RedisClientType;

  constructor(db: Pool, redis: RedisClientType) {
    this.db = db;
    this.redis = redis;
  }

  // ===========================================
  // Schedule Management
  // ===========================================

  /**
   * Create a new on-call schedule
   */
  async createSchedule(input: CreateScheduleInput): Promise<OnCallSchedule> {
    const schedule: OnCallSchedule = {
      id: uuidv4(),
      name: input.name,
      description: input.description,
      timezone: input.timezone || 'UTC',
      rotations: input.rotations.map(r => ({
        ...r,
        id: uuidv4(),
      })),
      overrides: [],
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO on_call_schedules (
        id, name, description, timezone, rotations, overrides, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        schedule.id,
        schedule.name,
        schedule.description,
        schedule.timezone,
        JSON.stringify(schedule.rotations),
        JSON.stringify(schedule.overrides),
        schedule.createdAt,
        schedule.updatedAt,
      ]
    );

    return schedule;
  }

  /**
   * Get schedule by ID
   */
  async getSchedule(scheduleId: string): Promise<OnCallSchedule | null> {
    const result = await this.db.query(
      `SELECT * FROM on_call_schedules WHERE id = $1`,
      [scheduleId]
    );

    if (result.rows.length === 0) return null;

    return this.mapScheduleRow(result.rows[0]);
  }

  /**
   * Get all schedules
   */
  async getSchedules(): Promise<OnCallSchedule[]> {
    const result = await this.db.query(
      `SELECT * FROM on_call_schedules ORDER BY name`
    );
    return result.rows.map(this.mapScheduleRow);
  }

  /**
   * Update schedule
   */
  async updateSchedule(
    scheduleId: string,
    updates: Partial<CreateScheduleInput>
  ): Promise<OnCallSchedule | null> {
    const schedule = await this.getSchedule(scheduleId);
    if (!schedule) return null;

    const updatedSchedule = {
      ...schedule,
      name: updates.name ?? schedule.name,
      description: updates.description ?? schedule.description,
      timezone: updates.timezone ?? schedule.timezone,
      rotations: updates.rotations
        ? updates.rotations.map(r => ({ ...r, id: uuidv4() }))
        : schedule.rotations,
      updatedAt: new Date(),
    };

    await this.db.query(
      `UPDATE on_call_schedules SET
        name = $1, description = $2, timezone = $3, rotations = $4, updated_at = NOW()
      WHERE id = $5`,
      [
        updatedSchedule.name,
        updatedSchedule.description,
        updatedSchedule.timezone,
        JSON.stringify(updatedSchedule.rotations),
        scheduleId,
      ]
    );

    // Clear cache
    await this.redis.del(`oncall:current:${scheduleId}`);

    return this.getSchedule(scheduleId);
  }

  /**
   * Delete schedule
   */
  async deleteSchedule(scheduleId: string): Promise<boolean> {
    const result = await this.db.query(
      `DELETE FROM on_call_schedules WHERE id = $1 RETURNING id`,
      [scheduleId]
    );

    if (result.rows.length > 0) {
      await this.redis.del(`oncall:current:${scheduleId}`);
      return true;
    }

    return false;
  }

  // ===========================================
  // Override Management
  // ===========================================

  /**
   * Create an on-call override
   */
  async createOverride(input: CreateOverrideInput): Promise<{ id: string }> {
    const overrideId = uuidv4();

    await this.db.query(
      `INSERT INTO on_call_overrides (
        id, schedule_id, user_id, start_at, end_at, reason, created_at
      ) VALUES ($1, $2, $3, $4, $5, $6, NOW())`,
      [
        overrideId,
        input.scheduleId,
        input.userId,
        input.startAt,
        input.endAt,
        input.reason || null,
      ]
    );

    // Clear cache
    await this.redis.del(`oncall:current:${input.scheduleId}`);

    return { id: overrideId };
  }

  /**
   * Get active overrides for a schedule
   */
  async getActiveOverrides(scheduleId: string): Promise<Array<{
    id: string;
    userId: string;
    user: OnCallUser;
    startAt: Date;
    endAt: Date;
    reason?: string;
  }>> {
    const result = await this.db.query(
      `SELECT o.*, u.name, u.email, u.phone, u.slack_user_id, u.notification_preferences
       FROM on_call_overrides o
       JOIN on_call_users u ON o.user_id = u.id
       WHERE o.schedule_id = $1 AND o.end_at > NOW()
       ORDER BY o.start_at ASC`,
      [scheduleId]
    );

    return result.rows.map(row => ({
      id: row.id,
      userId: row.user_id,
      user: {
        id: row.user_id,
        name: row.name,
        email: row.email,
        phone: row.phone,
        slackUserId: row.slack_user_id,
        notificationPreferences: row.notification_preferences || {},
        createdAt: row.created_at,
        updatedAt: row.updated_at,
      },
      startAt: row.start_at,
      endAt: row.end_at,
      reason: row.reason,
    }));
  }

  /**
   * Delete an override
   */
  async deleteOverride(overrideId: string): Promise<boolean> {
    const result = await this.db.query(
      `DELETE FROM on_call_overrides WHERE id = $1 RETURNING schedule_id`,
      [overrideId]
    );

    if (result.rows.length > 0) {
      await this.redis.del(`oncall:current:${result.rows[0].schedule_id}`);
      return true;
    }

    return false;
  }

  // ===========================================
  // User Management
  // ===========================================

  /**
   * Create an on-call user
   */
  async createUser(input: CreateUserInput): Promise<OnCallUser> {
    const user: OnCallUser = {
      id: uuidv4(),
      name: input.name,
      email: input.email,
      phone: input.phone,
      slackUserId: input.slackUserId,
      notificationPreferences: input.notificationPreferences || {
        email: true,
        slack: true,
        sms: false,
        phone: false,
      },
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    await this.db.query(
      `INSERT INTO on_call_users (
        id, name, email, phone, slack_user_id, notification_preferences, created_at, updated_at
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        user.id,
        user.name,
        user.email,
        user.phone,
        user.slackUserId,
        JSON.stringify(user.notificationPreferences),
        user.createdAt,
        user.updatedAt,
      ]
    );

    return user;
  }

  /**
   * Get user by ID
   */
  async getUser(userId: string): Promise<OnCallUser | null> {
    const result = await this.db.query(
      `SELECT * FROM on_call_users WHERE id = $1`,
      [userId]
    );

    if (result.rows.length === 0) return null;

    return this.mapUserRow(result.rows[0]);
  }

  /**
   * Get all users
   */
  async getUsers(): Promise<OnCallUser[]> {
    const result = await this.db.query(
      `SELECT * FROM on_call_users ORDER BY name`
    );
    return result.rows.map(this.mapUserRow);
  }

  /**
   * Update user
   */
  async updateUser(
    userId: string,
    updates: Partial<CreateUserInput>
  ): Promise<OnCallUser | null> {
    const user = await this.getUser(userId);
    if (!user) return null;

    await this.db.query(
      `UPDATE on_call_users SET
        name = COALESCE($1, name),
        email = COALESCE($2, email),
        phone = COALESCE($3, phone),
        slack_user_id = COALESCE($4, slack_user_id),
        notification_preferences = COALESCE($5, notification_preferences),
        updated_at = NOW()
      WHERE id = $6`,
      [
        updates.name,
        updates.email,
        updates.phone,
        updates.slackUserId,
        updates.notificationPreferences ? JSON.stringify(updates.notificationPreferences) : null,
        userId,
      ]
    );

    return this.getUser(userId);
  }

  /**
   * Delete user
   */
  async deleteUser(userId: string): Promise<boolean> {
    const result = await this.db.query(
      `DELETE FROM on_call_users WHERE id = $1 RETURNING id`,
      [userId]
    );
    return result.rows.length > 0;
  }

  // ===========================================
  // Current On-Call Resolution
  // ===========================================

  /**
   * Get the current on-call user for a schedule
   */
  async getCurrentOnCall(scheduleId: string): Promise<CurrentOnCall | null> {
    // Check cache first
    const cached = await this.redis.get(`oncall:current:${scheduleId}`);
    if (cached) {
      return JSON.parse(cached);
    }

    // Check for active override first
    const overrideResult = await this.db.query(
      `SELECT o.*, u.name, u.email, u.phone, u.slack_user_id, u.notification_preferences,
              s.name as schedule_name
       FROM on_call_overrides o
       JOIN on_call_users u ON o.user_id = u.id
       JOIN on_call_schedules s ON o.schedule_id = s.id
       WHERE o.schedule_id = $1 AND o.start_at <= NOW() AND o.end_at > NOW()
       ORDER BY o.created_at DESC
       LIMIT 1`,
      [scheduleId]
    );

    if (overrideResult.rows.length > 0) {
      const row = overrideResult.rows[0];
      const currentOnCall: CurrentOnCall = {
        scheduleId,
        scheduleName: row.schedule_name,
        user: {
          id: row.user_id,
          name: row.name,
          email: row.email,
          phone: row.phone,
          slackUserId: row.slack_user_id,
          notificationPreferences: row.notification_preferences || {},
          createdAt: row.created_at,
          updatedAt: row.updated_at,
        },
        startedAt: row.start_at,
        endsAt: row.end_at,
        isOverride: true,
      };

      // Cache for 5 minutes
      await this.redis.set(
        `oncall:current:${scheduleId}`,
        JSON.stringify(currentOnCall),
        { EX: 300 }
      );

      return currentOnCall;
    }

    // Get schedule and calculate current rotation
    const schedule = await this.getSchedule(scheduleId);
    if (!schedule || schedule.rotations.length === 0) {
      return null;
    }

    // Use first rotation for simplicity
    const rotation = schedule.rotations[0];
    if (!rotation.users || rotation.users.length === 0) {
      return null;
    }

    // Calculate which user is on call based on rotation type
    const currentUser = await this.calculateCurrentOnCallUser(rotation, schedule.timezone);
    if (!currentUser) return null;

    const user = await this.getUser(currentUser.userId);
    if (!user) return null;

    const currentOnCall: CurrentOnCall = {
      scheduleId,
      scheduleName: schedule.name,
      user,
      startedAt: currentUser.startedAt,
      endsAt: currentUser.endsAt,
      isOverride: false,
    };

    // Cache for 5 minutes
    await this.redis.set(
      `oncall:current:${scheduleId}`,
      JSON.stringify(currentOnCall),
      { EX: 300 }
    );

    return currentOnCall;
  }

  /**
   * Calculate the current on-call user based on rotation settings
   */
  private async calculateCurrentOnCallUser(
    rotation: OnCallSchedule['rotations'][0],
    timezone: string
  ): Promise<{ userId: string; startedAt: Date; endsAt: Date } | null> {
    if (!rotation.users || rotation.users.length === 0) {
      return null;
    }

    const now = new Date();
    const users = rotation.users;

    // Parse handoff time
    const [handoffHour, handoffMinute] = rotation.handoffTime.split(':').map(Number);

    switch (rotation.type) {
      case 'daily': {
        // Daily rotation - changes at handoffTime each day
        const todayHandoff = new Date(now);
        todayHandoff.setHours(handoffHour, handoffMinute, 0, 0);

        let rotationStart: Date;
        let rotationEnd: Date;

        if (now >= todayHandoff) {
          rotationStart = todayHandoff;
          rotationEnd = new Date(todayHandoff);
          rotationEnd.setDate(rotationEnd.getDate() + 1);
        } else {
          rotationEnd = todayHandoff;
          rotationStart = new Date(todayHandoff);
          rotationStart.setDate(rotationStart.getDate() - 1);
        }

        // Calculate which user based on day
        const epoch = new Date('2024-01-01');
        const daysSinceEpoch = Math.floor((rotationStart.getTime() - epoch.getTime()) / (1000 * 60 * 60 * 24));
        const userIndex = daysSinceEpoch % users.length;

        return {
          userId: users[userIndex],
          startedAt: rotationStart,
          endsAt: rotationEnd,
        };
      }

      case 'weekly': {
        // Weekly rotation - changes at handoffTime on a specific day
        const currentDayOfWeek = now.getDay();
        const daysToSubtract = currentDayOfWeek === 0 ? 6 : currentDayOfWeek - 1; // Monday = 0

        const weekStart = new Date(now);
        weekStart.setDate(weekStart.getDate() - daysToSubtract);
        weekStart.setHours(handoffHour, handoffMinute, 0, 0);

        const weekEnd = new Date(weekStart);
        weekEnd.setDate(weekEnd.getDate() + 7);

        // Adjust if we're before this week's handoff
        if (now < weekStart) {
          weekStart.setDate(weekStart.getDate() - 7);
          weekEnd.setDate(weekEnd.getDate() - 7);
        }

        // Calculate which user based on week
        const epoch = new Date('2024-01-01');
        const weeksSinceEpoch = Math.floor((weekStart.getTime() - epoch.getTime()) / (1000 * 60 * 60 * 24 * 7));
        const userIndex = weeksSinceEpoch % users.length;

        return {
          userId: users[userIndex],
          startedAt: weekStart,
          endsAt: weekEnd,
        };
      }

      case 'custom':
      default: {
        // Custom rotation - simple round-robin based on hour
        const hoursSinceEpoch = Math.floor(now.getTime() / (1000 * 60 * 60));
        const userIndex = hoursSinceEpoch % users.length;

        const rotationStart = new Date(now);
        rotationStart.setMinutes(0, 0, 0);

        const rotationEnd = new Date(rotationStart);
        rotationEnd.setHours(rotationEnd.getHours() + 1);

        return {
          userId: users[userIndex],
          startedAt: rotationStart,
          endsAt: rotationEnd,
        };
      }
    }
  }

  /**
   * Get all current on-calls across all schedules
   */
  async getAllCurrentOnCalls(): Promise<CurrentOnCall[]> {
    const schedules = await this.getSchedules();
    const currentOnCalls: CurrentOnCall[] = [];

    for (const schedule of schedules) {
      const current = await this.getCurrentOnCall(schedule.id);
      if (current) {
        currentOnCalls.push(current);
      }
    }

    return currentOnCalls;
  }

  /**
   * Get on-call shifts for a user in a time range
   */
  async getUserShifts(
    userId: string,
    startDate: Date,
    endDate: Date
  ): Promise<Array<{
    scheduleId: string;
    scheduleName: string;
    startAt: Date;
    endAt: Date;
    isOverride: boolean;
  }>> {
    // Get overrides for this user
    const overridesResult = await this.db.query(
      `SELECT o.*, s.name as schedule_name
       FROM on_call_overrides o
       JOIN on_call_schedules s ON o.schedule_id = s.id
       WHERE o.user_id = $1 AND o.start_at < $3 AND o.end_at > $2
       ORDER BY o.start_at ASC`,
      [userId, startDate, endDate]
    );

    const shifts = overridesResult.rows.map(row => ({
      scheduleId: row.schedule_id,
      scheduleName: row.schedule_name,
      startAt: row.start_at,
      endAt: row.end_at,
      isOverride: true,
    }));

    // TODO: Calculate regular rotation shifts for the user
    // This would involve iterating through schedules where the user is in a rotation

    return shifts;
  }

  // ===========================================
  // Private Helpers
  // ===========================================

  private mapScheduleRow(row: any): OnCallSchedule {
    return {
      id: row.id,
      name: row.name,
      description: row.description,
      timezone: row.timezone,
      rotations: row.rotations || [],
      overrides: row.overrides || [],
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private mapUserRow(row: any): OnCallUser {
    return {
      id: row.id,
      name: row.name,
      email: row.email,
      phone: row.phone,
      slackUserId: row.slack_user_id,
      notificationPreferences: row.notification_preferences || {},
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }
}
