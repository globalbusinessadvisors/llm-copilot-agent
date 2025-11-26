//! System metrics collector
//!
//! Collects system-level metrics like CPU, memory, and process stats.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// System metrics
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// Process uptime in seconds
    pub uptime_secs: f64,
    /// Process start time (Unix timestamp)
    pub start_time: i64,
    /// Memory usage in bytes (resident set size)
    pub memory_rss_bytes: u64,
    /// Virtual memory usage in bytes
    pub memory_virtual_bytes: u64,
    /// Number of open file descriptors
    pub open_fds: u64,
    /// Maximum allowed file descriptors
    pub max_fds: u64,
    /// Number of threads
    pub num_threads: u64,
    /// CPU usage (0.0 to 1.0)
    pub cpu_usage: f64,
}

impl SystemMetrics {
    /// Render system metrics in Prometheus format
    pub fn render_prometheus(&self, prefix: &str) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "# HELP {}_process_uptime_seconds Process uptime in seconds\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_uptime_seconds gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_uptime_seconds {}\n",
            prefix, self.uptime_secs
        ));

        output.push_str(&format!(
            "# HELP {}_process_start_time_seconds Process start time as Unix timestamp\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_start_time_seconds gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_start_time_seconds {}\n",
            prefix, self.start_time
        ));

        output.push_str(&format!(
            "# HELP {}_process_resident_memory_bytes Resident memory size in bytes\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_resident_memory_bytes gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_resident_memory_bytes {}\n",
            prefix, self.memory_rss_bytes
        ));

        output.push_str(&format!(
            "# HELP {}_process_virtual_memory_bytes Virtual memory size in bytes\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_virtual_memory_bytes gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_virtual_memory_bytes {}\n",
            prefix, self.memory_virtual_bytes
        ));

        output.push_str(&format!(
            "# HELP {}_process_open_fds Number of open file descriptors\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_open_fds gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_open_fds {}\n",
            prefix, self.open_fds
        ));

        output.push_str(&format!(
            "# HELP {}_process_max_fds Maximum number of file descriptors\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_max_fds gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_max_fds {}\n",
            prefix, self.max_fds
        ));

        output.push_str(&format!(
            "# HELP {}_process_threads Number of threads\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_threads gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_threads {}\n",
            prefix, self.num_threads
        ));

        output.push_str(&format!(
            "# HELP {}_process_cpu_usage CPU usage (0.0 to 1.0)\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_process_cpu_usage gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_process_cpu_usage {}\n",
            prefix, self.cpu_usage
        ));

        output
    }
}

/// Configuration for the metrics collector
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Prefix for metric names
    pub prefix: String,
    /// Collection interval
    pub interval: Duration,
    /// Whether to collect system metrics
    pub collect_system: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            prefix: "copilot".to_string(),
            interval: Duration::from_secs(15),
            collect_system: true,
        }
    }
}

/// Metrics collector
pub struct MetricsCollector {
    config: CollectorConfig,
    start_time: Instant,
    start_timestamp: i64,
    system_metrics: Arc<RwLock<SystemMetrics>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: CollectorConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            start_timestamp: chrono::Utc::now().timestamp(),
            system_metrics: Arc::new(RwLock::new(SystemMetrics::default())),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(CollectorConfig::default())
    }

    /// Start the collector background task
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let interval = self.config.interval;
        let start_time = self.start_time;
        let start_timestamp = self.start_timestamp;
        let system_metrics = Arc::clone(&self.system_metrics);
        let collect_system = self.config.collect_system;

        tokio::spawn(async move {
            loop {
                if collect_system {
                    let uptime = start_time.elapsed().as_secs_f64();
                    let metrics = collect_system_metrics(uptime, start_timestamp);

                    let mut current = system_metrics.write().await;
                    *current = metrics;

                    debug!("System metrics collected: uptime={:.2}s", uptime);
                }

                tokio::time::sleep(interval).await;
            }
        })
    }

    /// Get current system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }

    /// Manually collect and update system metrics
    pub async fn collect_now(&self) {
        let uptime = self.start_time.elapsed().as_secs_f64();
        let metrics = collect_system_metrics(uptime, self.start_timestamp);

        let mut current = self.system_metrics.write().await;
        *current = metrics;
    }

    /// Render all metrics in Prometheus format
    pub async fn render_prometheus(&self) -> String {
        let system = self.system_metrics.read().await;
        system.render_prometheus(&self.config.prefix)
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Collect system metrics
fn collect_system_metrics(uptime: f64, start_timestamp: i64) -> SystemMetrics {
    let mut metrics = SystemMetrics {
        uptime_secs: uptime,
        start_time: start_timestamp,
        ..Default::default()
    };

    // Try to read from /proc on Linux
    #[cfg(target_os = "linux")]
    {
        // Read memory from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(value) = extract_kb_value(line) {
                        metrics.memory_rss_bytes = value * 1024;
                    }
                } else if line.starts_with("VmSize:") {
                    if let Some(value) = extract_kb_value(line) {
                        metrics.memory_virtual_bytes = value * 1024;
                    }
                } else if line.starts_with("Threads:") {
                    if let Some(value) = extract_number(line) {
                        metrics.num_threads = value;
                    }
                }
            }
        }

        // Read file descriptors from /proc/self/fd
        if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
            metrics.open_fds = entries.count() as u64;
        }

        // Read max fds from /proc/self/limits
        if let Ok(limits) = std::fs::read_to_string("/proc/self/limits") {
            for line in limits.lines() {
                if line.contains("Max open files") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let Ok(max) = parts[3].parse() {
                            metrics.max_fds = max;
                        }
                    }
                }
            }
        }
    }

    // Fallback for non-Linux or when /proc is not available
    #[cfg(not(target_os = "linux"))]
    {
        // Use reasonable defaults for development
        metrics.memory_rss_bytes = 50 * 1024 * 1024; // 50 MB placeholder
        metrics.memory_virtual_bytes = 200 * 1024 * 1024; // 200 MB placeholder
        metrics.open_fds = 50;
        metrics.max_fds = 1024;
        metrics.num_threads = 4;
    }

    metrics
}

#[cfg(target_os = "linux")]
fn extract_kb_value(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn extract_number(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics_render() {
        let metrics = SystemMetrics {
            uptime_secs: 100.5,
            start_time: 1700000000,
            memory_rss_bytes: 50 * 1024 * 1024,
            memory_virtual_bytes: 200 * 1024 * 1024,
            open_fds: 50,
            max_fds: 1024,
            num_threads: 4,
            cpu_usage: 0.15,
        };

        let output = metrics.render_prometheus("test");

        assert!(output.contains("test_process_uptime_seconds 100.5"));
        assert!(output.contains("test_process_resident_memory_bytes"));
        assert!(output.contains("test_process_threads 4"));
    }

    #[tokio::test]
    async fn test_collector() {
        let collector = MetricsCollector::new(CollectorConfig {
            collect_system: true,
            interval: Duration::from_secs(60),
            ..Default::default()
        });

        collector.collect_now().await;
        let metrics = collector.get_system_metrics().await;

        assert!(metrics.uptime_secs >= 0.0);
    }
}
