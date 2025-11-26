//! Workflow scheduling
//!
//! Provides cron-based and time-based workflow scheduling.

use crate::{engine::{WorkflowEngine, WorkflowDefinition}, Result, WorkflowError};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval_at, Instant};
use tracing::{debug, error, info, warn};

/// Schedule specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    /// Run once at a specific time
    Once {
        at: DateTime<Utc>,
    },
    /// Run at fixed intervals
    Interval {
        /// Interval in seconds
        interval_seconds: u64,
        /// Start immediately or wait for first interval
        start_immediately: bool,
    },
    /// Run at specific times daily
    Daily {
        /// Times to run (in UTC)
        times: Vec<NaiveTime>,
        /// Timezone offset in hours (for display)
        timezone_offset_hours: i32,
    },
    /// Run on specific days of the week
    Weekly {
        /// Days to run
        days: Vec<Weekday>,
        /// Time to run (in UTC)
        time: NaiveTime,
    },
    /// Run on specific days of the month
    Monthly {
        /// Days of month (1-31)
        days: Vec<u32>,
        /// Time to run (in UTC)
        time: NaiveTime,
    },
    /// Cron expression (simplified)
    Cron {
        /// Cron expression string
        expression: String,
    },
}

impl Schedule {
    /// Calculate next execution time
    pub fn next_execution(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();

        match self {
            Schedule::Once { at } => {
                if *at > now {
                    Some(*at)
                } else {
                    None
                }
            }
            Schedule::Interval {
                interval_seconds,
                start_immediately,
            } => {
                if *start_immediately {
                    Some(now)
                } else {
                    Some(now + Duration::seconds(*interval_seconds as i64))
                }
            }
            Schedule::Daily { times, .. } => {
                // Find next time today or tomorrow
                let today = now.date_naive();

                for &time in times {
                    let candidate = today.and_time(time).and_utc();
                    if candidate > now {
                        return Some(candidate);
                    }
                }

                // All times passed today, use first time tomorrow
                times
                    .first()
                    .map(|&time| (today + Duration::days(1)).and_time(time).and_utc())
            }
            Schedule::Weekly { days, time } => {
                let today = now.date_naive();
                let current_weekday = today.weekday();

                // Check remaining days this week
                for &day in days {
                    let days_until = days_until_weekday(current_weekday, day);
                    let candidate_date = today + Duration::days(days_until as i64);
                    let candidate = candidate_date.and_time(*time).and_utc();

                    if candidate > now {
                        return Some(candidate);
                    }
                }

                // Next week
                days.first().map(|&day| {
                    let days_until = days_until_weekday(current_weekday, day) + 7;
                    let candidate_date = today + Duration::days(days_until as i64);
                    candidate_date.and_time(*time).and_utc()
                })
            }
            Schedule::Monthly { days, time } => {
                let today = now.date_naive();
                let current_day = today.day();

                // Check remaining days this month
                for &day in days {
                    if day > current_day {
                        if let Some(date) = today.with_day(day) {
                            let candidate = date.and_time(*time).and_utc();
                            if candidate > now {
                                return Some(candidate);
                            }
                        }
                    }
                }

                // Next month
                let next_month = if today.month() == 12 {
                    today.with_year(today.year() + 1)?.with_month(1)?
                } else {
                    today.with_month(today.month() + 1)?
                };

                days.first().and_then(|&day| {
                    let date = next_month.with_day(day.min(28))?; // Safe for all months
                    Some(date.and_time(*time).and_utc())
                })
            }
            Schedule::Cron { expression } => {
                // Simplified cron parsing - just support basic patterns
                parse_simple_cron(expression, now)
            }
        }
    }

    /// Calculate duration until next execution
    pub fn duration_until_next(&self) -> Option<std::time::Duration> {
        self.next_execution().map(|next| {
            let now = Utc::now();
            let duration = next - now;
            std::time::Duration::from_millis(duration.num_milliseconds().max(0) as u64)
        })
    }
}

/// Calculate days until a target weekday
fn days_until_weekday(current: Weekday, target: Weekday) -> u32 {
    let current_num = current.num_days_from_monday();
    let target_num = target.num_days_from_monday();

    if target_num >= current_num {
        target_num - current_num
    } else {
        7 - (current_num - target_num)
    }
}

/// Simple cron expression parser
fn parse_simple_cron(expression: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    // Very simplified: only support "minute hour * * *" format
    let parts: Vec<&str> = expression.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let minute: u32 = parts[0].parse().ok()?;
    let hour: u32 = parts[1].parse().ok()?;

    let today = now.date_naive();
    if let Some(time) = NaiveTime::from_hms_opt(hour, minute, 0) {
        let candidate = today.and_time(time).and_utc();
        if candidate > now {
            return Some(candidate);
        }
        // Tomorrow
        return Some((today + Duration::days(1)).and_time(time).and_utc());
    }

    None
}

/// Scheduled workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledWorkflow {
    /// Schedule ID
    pub id: String,
    /// Workflow ID to execute
    pub workflow_id: String,
    /// Schedule specification
    pub schedule: Schedule,
    /// Whether schedule is active
    pub enabled: bool,
    /// Input data for workflow
    pub input: serde_json::Value,
    /// Maximum concurrent executions
    pub max_concurrent: u32,
    /// Catch up missed executions
    pub catch_up: bool,
    /// Timezone for display
    pub timezone: String,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Last execution time
    pub last_execution: Option<DateTime<Utc>>,
    /// Next execution time
    pub next_execution: Option<DateTime<Utc>>,
    /// Tenant ID
    pub tenant_id: Option<String>,
    /// Tags
    pub tags: Vec<String>,
}

impl ScheduledWorkflow {
    pub fn new(workflow_id: &str, schedule: Schedule) -> Self {
        let next_execution = schedule.next_execution();

        Self {
            id: format!(
                "sched_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "")
            ),
            workflow_id: workflow_id.to_string(),
            schedule,
            enabled: true,
            input: serde_json::Value::Null,
            max_concurrent: 1,
            catch_up: false,
            timezone: "UTC".to_string(),
            created_at: Utc::now(),
            last_execution: None,
            next_execution,
            tenant_id: None,
            tags: Vec::new(),
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Update next execution time
    pub fn update_next_execution(&mut self) {
        self.next_execution = self.schedule.next_execution();
    }
}

/// Schedule repository trait
#[async_trait]
pub trait ScheduleRepository: Send + Sync {
    async fn save(&self, schedule: &ScheduledWorkflow) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<ScheduledWorkflow>>;
    async fn list(&self) -> Result<Vec<ScheduledWorkflow>>;
    async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<ScheduledWorkflow>>;
    async fn list_due(&self, until: DateTime<Utc>) -> Result<Vec<ScheduledWorkflow>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn update(&self, schedule: &ScheduledWorkflow) -> Result<()>;
}

/// In-memory schedule repository
pub struct InMemoryScheduleRepository {
    schedules: RwLock<HashMap<String, ScheduledWorkflow>>,
}

impl InMemoryScheduleRepository {
    pub fn new() -> Self {
        Self {
            schedules: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryScheduleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ScheduleRepository for InMemoryScheduleRepository {
    async fn save(&self, schedule: &ScheduledWorkflow) -> Result<()> {
        let mut schedules = self.schedules.write().await;
        schedules.insert(schedule.id.clone(), schedule.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<ScheduledWorkflow>> {
        let schedules = self.schedules.read().await;
        Ok(schedules.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<ScheduledWorkflow>> {
        let schedules = self.schedules.read().await;
        Ok(schedules.values().cloned().collect())
    }

    async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<ScheduledWorkflow>> {
        let schedules = self.schedules.read().await;
        Ok(schedules
            .values()
            .filter(|s| s.workflow_id == workflow_id)
            .cloned()
            .collect())
    }

    async fn list_due(&self, until: DateTime<Utc>) -> Result<Vec<ScheduledWorkflow>> {
        let schedules = self.schedules.read().await;
        Ok(schedules
            .values()
            .filter(|s| {
                s.enabled && s.next_execution.map(|t| t <= until).unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut schedules = self.schedules.write().await;
        schedules.remove(id);
        Ok(())
    }

    async fn update(&self, schedule: &ScheduledWorkflow) -> Result<()> {
        let mut schedules = self.schedules.write().await;
        if schedules.contains_key(&schedule.id) {
            schedules.insert(schedule.id.clone(), schedule.clone());
            Ok(())
        } else {
            Err(WorkflowError::NotFound(schedule.id.clone()))
        }
    }
}

/// Execution request sent to the scheduler
#[derive(Debug)]
pub struct ScheduledExecution {
    pub schedule_id: String,
    pub workflow_id: String,
    pub input: serde_json::Value,
    pub scheduled_time: DateTime<Utc>,
}

/// Workflow scheduler service
pub struct WorkflowScheduler {
    repository: Arc<dyn ScheduleRepository>,
    execution_sender: mpsc::Sender<ScheduledExecution>,
    poll_interval_seconds: u64,
    running: Arc<RwLock<bool>>,
}

impl WorkflowScheduler {
    pub fn new(
        repository: Arc<dyn ScheduleRepository>,
        execution_sender: mpsc::Sender<ScheduledExecution>,
    ) -> Self {
        Self {
            repository,
            execution_sender,
            poll_interval_seconds: 60,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_poll_interval(mut self, seconds: u64) -> Self {
        self.poll_interval_seconds = seconds;
        self
    }

    /// Create a new schedule
    pub async fn create(&self, schedule: ScheduledWorkflow) -> Result<ScheduledWorkflow> {
        self.repository.save(&schedule).await?;

        info!(
            schedule_id = %schedule.id,
            workflow_id = %schedule.workflow_id,
            next = ?schedule.next_execution,
            "Created workflow schedule"
        );

        Ok(schedule)
    }

    /// Enable a schedule
    pub async fn enable(&self, id: &str) -> Result<()> {
        if let Some(mut schedule) = self.repository.get(id).await? {
            schedule.enabled = true;
            schedule.update_next_execution();
            self.repository.update(&schedule).await?;

            info!(schedule_id = %id, "Enabled schedule");
            Ok(())
        } else {
            Err(WorkflowError::NotFound(id.to_string()))
        }
    }

    /// Disable a schedule
    pub async fn disable(&self, id: &str) -> Result<()> {
        if let Some(mut schedule) = self.repository.get(id).await? {
            schedule.enabled = false;
            self.repository.update(&schedule).await?;

            info!(schedule_id = %id, "Disabled schedule");
            Ok(())
        } else {
            Err(WorkflowError::NotFound(id.to_string()))
        }
    }

    /// Delete a schedule
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.repository.delete(id).await?;
        info!(schedule_id = %id, "Deleted schedule");
        Ok(())
    }

    /// Start the scheduler loop
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Scheduler already running");
                return;
            }
            *running = true;
        }

        info!(
            poll_interval = self.poll_interval_seconds,
            "Starting workflow scheduler"
        );

        let start = Instant::now() + std::time::Duration::from_secs(1);
        let mut interval =
            interval_at(start, std::time::Duration::from_secs(self.poll_interval_seconds));

        loop {
            interval.tick().await;

            let running = *self.running.read().await;
            if !running {
                break;
            }

            if let Err(e) = self.check_and_execute().await {
                error!(error = %e, "Error checking schedules");
            }
        }

        info!("Workflow scheduler stopped");
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopping workflow scheduler");
    }

    /// Check for due schedules and execute them
    async fn check_and_execute(&self) -> Result<()> {
        let now = Utc::now();
        let due_schedules = self.repository.list_due(now).await?;

        debug!(count = due_schedules.len(), "Checking due schedules");

        for schedule in due_schedules {
            // Send execution request
            let execution = ScheduledExecution {
                schedule_id: schedule.id.clone(),
                workflow_id: schedule.workflow_id.clone(),
                input: schedule.input.clone(),
                scheduled_time: schedule.next_execution.unwrap_or(now),
            };

            if let Err(e) = self.execution_sender.send(execution).await {
                error!(
                    schedule_id = %schedule.id,
                    error = %e,
                    "Failed to send execution request"
                );
                continue;
            }

            // Update schedule
            let mut updated = schedule.clone();
            updated.last_execution = Some(now);
            updated.update_next_execution();

            if let Err(e) = self.repository.update(&updated).await {
                error!(
                    schedule_id = %schedule.id,
                    error = %e,
                    "Failed to update schedule"
                );
            }

            info!(
                schedule_id = %schedule.id,
                workflow_id = %schedule.workflow_id,
                next = ?updated.next_execution,
                "Triggered scheduled workflow"
            );
        }

        Ok(())
    }
}

/// Workflow definition provider trait
#[async_trait]
pub trait WorkflowProvider: Send + Sync {
    /// Get workflow definition by ID
    async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>>;
}

/// Scheduled execution processor
pub struct ScheduledExecutionProcessor {
    receiver: mpsc::Receiver<ScheduledExecution>,
    engine: Arc<WorkflowEngine>,
    provider: Arc<dyn WorkflowProvider>,
}

impl ScheduledExecutionProcessor {
    pub fn new(
        receiver: mpsc::Receiver<ScheduledExecution>,
        engine: Arc<WorkflowEngine>,
        provider: Arc<dyn WorkflowProvider>,
    ) -> Self {
        Self {
            receiver,
            engine,
            provider,
        }
    }

    /// Run the processor
    pub async fn run(mut self) {
        info!("Starting scheduled execution processor");

        while let Some(execution) = self.receiver.recv().await {
            info!(
                schedule_id = %execution.schedule_id,
                workflow_id = %execution.workflow_id,
                "Processing scheduled execution"
            );

            // Get workflow definition
            let definition = match self.provider.get_workflow(&execution.workflow_id).await {
                Ok(Some(def)) => def,
                Ok(None) => {
                    error!(
                        schedule_id = %execution.schedule_id,
                        workflow_id = %execution.workflow_id,
                        "Workflow definition not found"
                    );
                    continue;
                }
                Err(e) => {
                    error!(
                        schedule_id = %execution.schedule_id,
                        error = %e,
                        "Failed to get workflow definition"
                    );
                    continue;
                }
            };

            match self.engine.execute_workflow(definition).await {
                Ok(run_id) => {
                    info!(
                        schedule_id = %execution.schedule_id,
                        run_id = %run_id,
                        "Started scheduled workflow"
                    );
                }
                Err(e) => {
                    error!(
                        schedule_id = %execution.schedule_id,
                        error = %e,
                        "Failed to start scheduled workflow"
                    );
                }
            }
        }

        info!("Scheduled execution processor stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_schedule() {
        let schedule = Schedule::Interval {
            interval_seconds: 3600,
            start_immediately: false,
        };

        let next = schedule.next_execution();
        assert!(next.is_some());

        let now = Utc::now();
        let diff = next.unwrap() - now;
        assert!(diff.num_seconds() >= 3599 && diff.num_seconds() <= 3601);
    }

    #[test]
    fn test_once_schedule() {
        let future = Utc::now() + Duration::hours(1);
        let schedule = Schedule::Once { at: future };

        let next = schedule.next_execution();
        assert!(next.is_some());
        assert_eq!(next.unwrap(), future);

        let past = Utc::now() - Duration::hours(1);
        let schedule = Schedule::Once { at: past };
        assert!(schedule.next_execution().is_none());
    }

    #[test]
    fn test_daily_schedule() {
        let times = vec![
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        ];
        let schedule = Schedule::Daily {
            times,
            timezone_offset_hours: 0,
        };

        let next = schedule.next_execution();
        assert!(next.is_some());
    }

    #[test]
    fn test_days_until_weekday() {
        assert_eq!(days_until_weekday(Weekday::Mon, Weekday::Wed), 2);
        assert_eq!(days_until_weekday(Weekday::Fri, Weekday::Mon), 3);
        assert_eq!(days_until_weekday(Weekday::Mon, Weekday::Mon), 0);
    }

    #[tokio::test]
    async fn test_schedule_repository() {
        let repo = InMemoryScheduleRepository::new();

        let schedule = ScheduledWorkflow::new(
            "wf-1",
            Schedule::Interval {
                interval_seconds: 3600,
                start_immediately: false,
            },
        );

        repo.save(&schedule).await.unwrap();

        let retrieved = repo.get(&schedule.id).await.unwrap();
        assert!(retrieved.is_some());

        let by_workflow = repo.list_by_workflow("wf-1").await.unwrap();
        assert_eq!(by_workflow.len(), 1);

        repo.delete(&schedule.id).await.unwrap();
        assert!(repo.get(&schedule.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_scheduler_create() {
        let repo = Arc::new(InMemoryScheduleRepository::new());
        let (tx, _rx) = mpsc::channel(100);
        let scheduler = WorkflowScheduler::new(repo.clone(), tx);

        let schedule = ScheduledWorkflow::new(
            "wf-1",
            Schedule::Interval {
                interval_seconds: 60,
                start_immediately: false,
            },
        );

        let created = scheduler.create(schedule).await.unwrap();
        assert!(created.enabled);
        assert!(created.next_execution.is_some());
    }
}
