//! SLA monitoring and alerting
//!
//! Provides SLA tracking, monitoring, and alerting capabilities.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

/// SLA metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaMetric {
    /// API availability percentage
    Availability,
    /// Response time (p50)
    ResponseTimeP50,
    /// Response time (p95)
    ResponseTimeP95,
    /// Response time (p99)
    ResponseTimeP99,
    /// Error rate
    ErrorRate,
    /// Throughput (requests per second)
    Throughput,
    /// Uptime
    Uptime,
}

impl SlaMetric {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Availability => "availability",
            Self::ResponseTimeP50 => "response_time_p50",
            Self::ResponseTimeP95 => "response_time_p95",
            Self::ResponseTimeP99 => "response_time_p99",
            Self::ErrorRate => "error_rate",
            Self::Throughput => "throughput",
            Self::Uptime => "uptime",
        }
    }
}

/// SLA target definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaTarget {
    /// Metric type
    pub metric: SlaMetric,
    /// Target value
    pub target: f64,
    /// Warning threshold (percentage of target)
    pub warning_threshold: f64,
    /// Critical threshold (percentage of target)
    pub critical_threshold: f64,
    /// Description
    pub description: String,
}

impl SlaTarget {
    pub fn availability(target_percentage: f64) -> Self {
        Self {
            metric: SlaMetric::Availability,
            target: target_percentage,
            warning_threshold: 99.9,
            critical_threshold: 99.5,
            description: format!("API availability must be >= {}%", target_percentage),
        }
    }

    pub fn response_time_p99(target_ms: f64) -> Self {
        Self {
            metric: SlaMetric::ResponseTimeP99,
            target: target_ms,
            warning_threshold: 80.0, // 80% of target is warning
            critical_threshold: 100.0, // At target is critical
            description: format!("P99 response time must be <= {}ms", target_ms),
        }
    }

    pub fn error_rate(target_percentage: f64) -> Self {
        Self {
            metric: SlaMetric::ErrorRate,
            target: target_percentage,
            warning_threshold: 50.0, // 50% of target is warning
            critical_threshold: 100.0, // At target is critical
            description: format!("Error rate must be <= {}%", target_percentage),
        }
    }
}

/// SLA status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaStatus {
    /// SLA is being met
    Ok,
    /// SLA is at warning level
    Warning,
    /// SLA is being violated
    Critical,
    /// SLA status is unknown
    Unknown,
}

impl SlaStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Unknown => "unknown",
        }
    }
}

/// SLA check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaCheckResult {
    /// Metric type
    pub metric: SlaMetric,
    /// Target value
    pub target: f64,
    /// Current value
    pub current: f64,
    /// Status
    pub status: SlaStatus,
    /// Check timestamp
    pub timestamp: DateTime<Utc>,
    /// Period start
    pub period_start: DateTime<Utc>,
    /// Period end
    pub period_end: DateTime<Utc>,
    /// Message
    pub message: String,
}

impl SlaCheckResult {
    pub fn is_ok(&self) -> bool {
        self.status == SlaStatus::Ok
    }

    pub fn is_violation(&self) -> bool {
        self.status == SlaStatus::Critical
    }
}

/// SLA measurement for tracking
#[derive(Debug, Clone)]
struct SlaMeasurement {
    timestamp: DateTime<Utc>,
    value: f64,
    success: bool,
}

/// Response time tracker
#[derive(Debug, Default)]
struct ResponseTimeTracker {
    measurements: Vec<SlaMeasurement>,
}

impl ResponseTimeTracker {
    fn add(&mut self, duration_ms: f64, success: bool) {
        self.measurements.push(SlaMeasurement {
            timestamp: Utc::now(),
            value: duration_ms,
            success,
        });
    }

    fn cleanup(&mut self, max_age: Duration) {
        let cutoff = Utc::now() - max_age;
        self.measurements.retain(|m| m.timestamp > cutoff);
    }

    fn percentile(&self, p: f64, max_age: Duration) -> Option<f64> {
        let cutoff = Utc::now() - max_age;
        let mut values: Vec<f64> = self
            .measurements
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .map(|m| m.value)
            .collect();

        if values.is_empty() {
            return None;
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = ((p / 100.0) * values.len() as f64).ceil() as usize - 1;
        Some(values[index.min(values.len() - 1)])
    }

    fn availability(&self, max_age: Duration) -> f64 {
        let cutoff = Utc::now() - max_age;
        let measurements: Vec<_> = self
            .measurements
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .collect();

        if measurements.is_empty() {
            return 100.0;
        }

        let success_count = measurements.iter().filter(|m| m.success).count();
        (success_count as f64 / measurements.len() as f64) * 100.0
    }

    fn error_rate(&self, max_age: Duration) -> f64 {
        100.0 - self.availability(max_age)
    }

    fn count(&self, max_age: Duration) -> usize {
        let cutoff = Utc::now() - max_age;
        self.measurements
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .count()
    }
}

/// SLA monitor for tracking and alerting
pub struct SlaMonitor {
    /// SLA targets
    targets: Arc<RwLock<Vec<SlaTarget>>>,
    /// Response time tracker per endpoint
    trackers: Arc<RwLock<HashMap<String, ResponseTimeTracker>>>,
    /// Check results history
    results: Arc<RwLock<Vec<SlaCheckResult>>>,
    /// Alert callbacks
    alert_handlers: Arc<RwLock<Vec<Box<dyn Fn(&SlaCheckResult) + Send + Sync>>>>,
    /// Check period
    check_period: Duration,
    /// Data retention period
    retention_period: Duration,
}

impl Default for SlaMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SlaMonitor {
    pub fn new() -> Self {
        Self {
            targets: Arc::new(RwLock::new(Vec::new())),
            trackers: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(Vec::new())),
            alert_handlers: Arc::new(RwLock::new(Vec::new())),
            check_period: Duration::minutes(5),
            retention_period: Duration::hours(24),
        }
    }

    /// Create with standard SLA targets
    pub fn with_standard_targets() -> Self {
        let monitor = Self::new();
        monitor.add_target(SlaTarget::availability(99.9));
        monitor.add_target(SlaTarget::response_time_p99(100.0));
        monitor.add_target(SlaTarget::error_rate(1.0));
        monitor
    }

    /// Add an SLA target
    pub fn add_target(&self, target: SlaTarget) {
        self.targets.write().push(target);
    }

    /// Set check period
    pub fn with_check_period(mut self, period: Duration) -> Self {
        self.check_period = period;
        self
    }

    /// Set retention period
    pub fn with_retention_period(mut self, period: Duration) -> Self {
        self.retention_period = period;
        self
    }

    /// Add alert handler
    pub fn on_alert<F>(&self, handler: F)
    where
        F: Fn(&SlaCheckResult) + Send + Sync + 'static,
    {
        self.alert_handlers.write().push(Box::new(handler));
    }

    /// Record a request
    pub fn record_request(&self, endpoint: &str, duration_ms: f64, success: bool) {
        let mut trackers = self.trackers.write();
        trackers
            .entry(endpoint.to_string())
            .or_default()
            .add(duration_ms, success);
    }

    /// Check all SLA targets
    pub fn check_all(&self) -> Vec<SlaCheckResult> {
        let targets = self.targets.read();
        let trackers = self.trackers.read();
        let now = Utc::now();
        let period_start = now - self.check_period;

        let mut results = Vec::new();

        for target in targets.iter() {
            let result = self.check_target(target, &trackers, period_start, now);
            results.push(result);
        }

        // Store results
        self.results.write().extend(results.clone());

        // Trigger alerts for violations
        for result in &results {
            if result.status == SlaStatus::Critical || result.status == SlaStatus::Warning {
                self.trigger_alert(result);
            }
        }

        results
    }

    /// Check a single target
    fn check_target(
        &self,
        target: &SlaTarget,
        trackers: &HashMap<String, ResponseTimeTracker>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> SlaCheckResult {
        let duration = period_end - period_start;

        // Aggregate across all endpoints
        let mut all_measurements = ResponseTimeTracker::default();
        for tracker in trackers.values() {
            for m in &tracker.measurements {
                if m.timestamp >= period_start && m.timestamp <= period_end {
                    all_measurements.add(m.value, m.success);
                }
            }
        }

        let (current, status, message) = match target.metric {
            SlaMetric::Availability => {
                let availability = all_measurements.availability(duration);
                let status = if availability >= target.target {
                    SlaStatus::Ok
                } else if availability >= target.critical_threshold {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    availability,
                    status,
                    format!("Availability: {:.2}% (target: {}%)", availability, target.target),
                )
            }
            SlaMetric::ResponseTimeP50 => {
                let p50 = all_measurements.percentile(50.0, duration).unwrap_or(0.0);
                let status = if p50 <= target.target {
                    SlaStatus::Ok
                } else if p50 <= target.target * (target.critical_threshold / 100.0) {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    p50,
                    status,
                    format!("P50 response time: {:.2}ms (target: {}ms)", p50, target.target),
                )
            }
            SlaMetric::ResponseTimeP95 => {
                let p95 = all_measurements.percentile(95.0, duration).unwrap_or(0.0);
                let status = if p95 <= target.target {
                    SlaStatus::Ok
                } else if p95 <= target.target * (target.critical_threshold / 100.0) {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    p95,
                    status,
                    format!("P95 response time: {:.2}ms (target: {}ms)", p95, target.target),
                )
            }
            SlaMetric::ResponseTimeP99 => {
                let p99 = all_measurements.percentile(99.0, duration).unwrap_or(0.0);
                let status = if p99 <= target.target {
                    SlaStatus::Ok
                } else if p99 <= target.target * (target.critical_threshold / 100.0) {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    p99,
                    status,
                    format!("P99 response time: {:.2}ms (target: {}ms)", p99, target.target),
                )
            }
            SlaMetric::ErrorRate => {
                let error_rate = all_measurements.error_rate(duration);
                let status = if error_rate <= target.target {
                    SlaStatus::Ok
                } else if error_rate <= target.target * 2.0 {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    error_rate,
                    status,
                    format!("Error rate: {:.2}% (target: {}%)", error_rate, target.target),
                )
            }
            SlaMetric::Throughput => {
                let count = all_measurements.count(duration);
                let duration_secs = duration.num_seconds() as f64;
                let throughput = if duration_secs > 0.0 {
                    count as f64 / duration_secs
                } else {
                    0.0
                };
                let status = if throughput >= target.target {
                    SlaStatus::Ok
                } else if throughput >= target.target * (target.warning_threshold / 100.0) {
                    SlaStatus::Warning
                } else {
                    SlaStatus::Critical
                };
                (
                    throughput,
                    status,
                    format!("Throughput: {:.2} req/s (target: {} req/s)", throughput, target.target),
                )
            }
            SlaMetric::Uptime => {
                // Uptime would be calculated differently, simplified here
                (100.0, SlaStatus::Ok, "Uptime check".to_string())
            }
        };

        SlaCheckResult {
            metric: target.metric,
            target: target.target,
            current,
            status,
            timestamp: period_end,
            period_start,
            period_end,
            message,
        }
    }

    /// Trigger alert handlers
    fn trigger_alert(&self, result: &SlaCheckResult) {
        let handlers = self.alert_handlers.read();

        if result.status == SlaStatus::Critical {
            warn!(
                metric = ?result.metric,
                current = result.current,
                target = result.target,
                "SLA VIOLATION: {}",
                result.message
            );
        } else {
            info!(
                metric = ?result.metric,
                current = result.current,
                target = result.target,
                "SLA WARNING: {}",
                result.message
            );
        }

        for handler in handlers.iter() {
            handler(result);
        }
    }

    /// Get current SLA status summary
    pub fn get_status_summary(&self) -> SlaSummary {
        let results = self.check_all();

        let ok_count = results.iter().filter(|r| r.status == SlaStatus::Ok).count();
        let warning_count = results.iter().filter(|r| r.status == SlaStatus::Warning).count();
        let critical_count = results.iter().filter(|r| r.status == SlaStatus::Critical).count();

        let overall_status = if critical_count > 0 {
            SlaStatus::Critical
        } else if warning_count > 0 {
            SlaStatus::Warning
        } else {
            SlaStatus::Ok
        };

        SlaSummary {
            overall_status,
            total_targets: results.len(),
            ok_count,
            warning_count,
            critical_count,
            checks: results,
            timestamp: Utc::now(),
        }
    }

    /// Cleanup old data
    pub fn cleanup(&self) {
        let cutoff = Utc::now() - self.retention_period;

        // Clean up trackers
        for tracker in self.trackers.write().values_mut() {
            tracker.cleanup(self.retention_period);
        }

        // Clean up results
        self.results.write().retain(|r| r.timestamp > cutoff);

        debug!("Cleaned up SLA monitoring data");
    }
}

/// SLA status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaSummary {
    /// Overall status
    pub overall_status: SlaStatus,
    /// Total targets
    pub total_targets: usize,
    /// OK count
    pub ok_count: usize,
    /// Warning count
    pub warning_count: usize,
    /// Critical count
    pub critical_count: usize,
    /// Individual checks
    pub checks: Vec<SlaCheckResult>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sla_target() {
        let target = SlaTarget::availability(99.9);
        assert_eq!(target.metric, SlaMetric::Availability);
        assert_eq!(target.target, 99.9);
    }

    #[test]
    fn test_sla_monitor() {
        let monitor = SlaMonitor::with_standard_targets();

        // Record some requests
        for _ in 0..100 {
            monitor.record_request("/api/chat", 50.0, true);
        }
        // Add some failures
        for _ in 0..2 {
            monitor.record_request("/api/chat", 200.0, false);
        }

        let summary = monitor.get_status_summary();

        assert!(summary.total_targets > 0);
        println!("SLA Summary: {:?}", summary);
    }

    #[test]
    fn test_response_time_tracker() {
        let mut tracker = ResponseTimeTracker::default();

        tracker.add(10.0, true);
        tracker.add(20.0, true);
        tracker.add(30.0, true);
        tracker.add(100.0, false);

        let p50 = tracker.percentile(50.0, Duration::hours(1)).unwrap();
        let p99 = tracker.percentile(99.0, Duration::hours(1)).unwrap();
        let availability = tracker.availability(Duration::hours(1));
        let error_rate = tracker.error_rate(Duration::hours(1));

        assert!(p50 <= p99);
        assert_eq!(availability, 75.0);
        assert_eq!(error_rate, 25.0);
    }

    #[test]
    fn test_alert_handler() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let monitor = SlaMonitor::new();
        let alert_count = Arc::new(AtomicUsize::new(0));
        let alert_count_clone = alert_count.clone();

        monitor.on_alert(move |_result| {
            alert_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Add a target that will be violated
        monitor.add_target(SlaTarget::availability(100.0));

        // Record a failure
        monitor.record_request("/api/test", 50.0, false);

        // Check will trigger alert
        monitor.check_all();

        // Alert should have been triggered
        assert!(alert_count.load(Ordering::SeqCst) > 0);
    }
}
