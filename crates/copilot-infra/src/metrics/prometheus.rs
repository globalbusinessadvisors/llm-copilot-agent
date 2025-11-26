//! Prometheus metrics implementation
//!
//! Provides metric types and registration for Prometheus monitoring.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Configuration for metrics
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Application name prefix for metrics
    pub prefix: String,
    /// Whether to include default labels
    pub include_default_labels: bool,
    /// Default labels to add to all metrics
    pub default_labels: HashMap<String, String>,
    /// Whether to collect histogram metrics
    pub enable_histograms: bool,
    /// Histogram buckets for latency metrics (in seconds)
    pub latency_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            prefix: "copilot".to_string(),
            include_default_labels: true,
            default_labels: HashMap::new(),
            enable_histograms: true,
            latency_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        }
    }
}

impl MetricsConfig {
    /// Create a new config with a prefix
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            ..Default::default()
        }
    }

    /// Add a default label
    pub fn with_label(mut self, name: &str, value: &str) -> Self {
        self.default_labels.insert(name.to_string(), value.to_string());
        self
    }

    /// Set latency buckets
    pub fn with_latency_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.latency_buckets = buckets;
        self
    }
}

/// Counter metric
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    /// Create a new counter
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter by 1
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by a value
    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset the counter
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// Gauge metric
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    /// Create a new gauge
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the gauge value
    pub fn set(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Increment the gauge by 1
    pub fn inc(&self) {
        let current = f64::from_bits(self.value.load(Ordering::Relaxed));
        self.set(current + 1.0);
    }

    /// Decrement the gauge by 1
    pub fn dec(&self) {
        let current = f64::from_bits(self.value.load(Ordering::Relaxed));
        self.set(current - 1.0);
    }

    /// Get the current value
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }
}

/// Histogram metric
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<f64>,
    bucket_counts: Vec<AtomicU64>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    /// Create a new histogram with the given buckets
    pub fn new(buckets: Vec<f64>) -> Self {
        let bucket_counts = (0..buckets.len() + 1)
            .map(|_| AtomicU64::new(0))
            .collect();

        Self {
            buckets,
            bucket_counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Observe a value
    pub fn observe(&self, value: f64) {
        // Find the bucket
        let mut idx = self.buckets.len();
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                idx = i;
                break;
            }
        }

        // Increment bucket count
        self.bucket_counts[idx].fetch_add(1, Ordering::Relaxed);

        // Update sum
        let bits = value.to_bits();
        loop {
            let current = self.sum.load(Ordering::Relaxed);
            let current_f64 = f64::from_bits(current);
            let new_value = (current_f64 + value).to_bits();
            if self.sum.compare_exchange(current, new_value, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                break;
            }
        }

        // Increment count
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Start a timer that observes when dropped
    pub fn start_timer(&self) -> HistogramTimer<'_> {
        HistogramTimer {
            histogram: self,
            start: Instant::now(),
        }
    }

    /// Get the count
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get the sum
    pub fn get_sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }

    /// Get bucket counts
    pub fn get_buckets(&self) -> Vec<(f64, u64)> {
        self.buckets
            .iter()
            .enumerate()
            .map(|(i, &b)| (b, self.bucket_counts[i].load(Ordering::Relaxed)))
            .collect()
    }
}

/// Timer for histogram observations
pub struct HistogramTimer<'a> {
    histogram: &'a Histogram,
    start: Instant,
}

impl<'a> Drop for HistogramTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

/// Handle for recording metrics
pub struct MetricsHandle {
    http: HttpMetrics,
    database: DatabaseMetrics,
    cache: CacheMetrics,
    circuit_breaker: CircuitBreakerMetrics,
}

impl MetricsHandle {
    /// Get HTTP metrics
    pub fn http(&self) -> &HttpMetrics {
        &self.http
    }

    /// Get database metrics
    pub fn database(&self) -> &DatabaseMetrics {
        &self.database
    }

    /// Get cache metrics
    pub fn cache(&self) -> &CacheMetrics {
        &self.cache
    }

    /// Get circuit breaker metrics
    pub fn circuit_breaker(&self) -> &CircuitBreakerMetrics {
        &self.circuit_breaker
    }
}

/// HTTP request metrics
pub struct HttpMetrics {
    /// Total requests
    pub requests_total: Arc<RwLock<HashMap<String, Counter>>>,
    /// Request duration histogram
    pub request_duration: Arc<Histogram>,
    /// Requests in progress
    pub requests_in_progress: Arc<Gauge>,
    /// Response sizes
    pub response_size_bytes: Arc<Histogram>,
}

impl HttpMetrics {
    /// Create new HTTP metrics
    pub fn new(config: &MetricsConfig) -> Self {
        Self {
            requests_total: Arc::new(RwLock::new(HashMap::new())),
            request_duration: Arc::new(Histogram::new(config.latency_buckets.clone())),
            requests_in_progress: Arc::new(Gauge::new()),
            response_size_bytes: Arc::new(Histogram::new(vec![
                100.0, 1000.0, 10000.0, 100000.0, 1000000.0,
            ])),
        }
    }

    /// Record a request
    pub async fn record_request(&self, method: &str, path: &str, status: u16, duration: Duration, size: usize) {
        let key = format!("{}:{}:{}", method, path, status);

        {
            let mut requests = self.requests_total.write().await;
            requests
                .entry(key)
                .or_insert_with(Counter::new)
                .inc();
        }

        self.request_duration.observe(duration.as_secs_f64());
        self.response_size_bytes.observe(size as f64);
    }

    /// Start tracking a request
    pub fn track_request(&self) -> RequestTracker {
        self.requests_in_progress.inc();
        RequestTracker {
            gauge: Arc::clone(&self.requests_in_progress),
            start: Instant::now(),
        }
    }
}

/// Request tracker for in-progress requests
pub struct RequestTracker {
    gauge: Arc<Gauge>,
    start: Instant,
}

impl RequestTracker {
    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for RequestTracker {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

/// Database metrics
pub struct DatabaseMetrics {
    /// Query duration histogram
    pub query_duration: Arc<Histogram>,
    /// Total queries by type
    pub queries_total: Arc<RwLock<HashMap<String, Counter>>>,
    /// Connection pool size
    pub pool_connections: Arc<Gauge>,
    /// Idle connections
    pub pool_idle_connections: Arc<Gauge>,
}

impl DatabaseMetrics {
    /// Create new database metrics
    pub fn new(config: &MetricsConfig) -> Self {
        Self {
            query_duration: Arc::new(Histogram::new(config.latency_buckets.clone())),
            queries_total: Arc::new(RwLock::new(HashMap::new())),
            pool_connections: Arc::new(Gauge::new()),
            pool_idle_connections: Arc::new(Gauge::new()),
        }
    }

    /// Record a query
    pub async fn record_query(&self, query_type: &str, duration: Duration, success: bool) {
        let key = format!("{}:{}", query_type, if success { "success" } else { "error" });

        {
            let mut queries = self.queries_total.write().await;
            queries
                .entry(key)
                .or_insert_with(Counter::new)
                .inc();
        }

        self.query_duration.observe(duration.as_secs_f64());
    }

    /// Update pool metrics
    pub fn update_pool(&self, total: usize, idle: usize) {
        self.pool_connections.set(total as f64);
        self.pool_idle_connections.set(idle as f64);
    }
}

/// Cache metrics
pub struct CacheMetrics {
    /// Cache hits
    pub hits: Arc<Counter>,
    /// Cache misses
    pub misses: Arc<Counter>,
    /// Cache sets
    pub sets: Arc<Counter>,
    /// Cache deletes
    pub deletes: Arc<Counter>,
    /// Cache operation duration
    pub operation_duration: Arc<Histogram>,
}

impl CacheMetrics {
    /// Create new cache metrics
    pub fn new(config: &MetricsConfig) -> Self {
        Self {
            hits: Arc::new(Counter::new()),
            misses: Arc::new(Counter::new()),
            sets: Arc::new(Counter::new()),
            deletes: Arc::new(Counter::new()),
            operation_duration: Arc::new(Histogram::new(config.latency_buckets.clone())),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&self, duration: Duration) {
        self.hits.inc();
        self.operation_duration.observe(duration.as_secs_f64());
    }

    /// Record a cache miss
    pub fn record_miss(&self, duration: Duration) {
        self.misses.inc();
        self.operation_duration.observe(duration.as_secs_f64());
    }

    /// Record a cache set
    pub fn record_set(&self, duration: Duration) {
        self.sets.inc();
        self.operation_duration.observe(duration.as_secs_f64());
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.get() as f64;
        let misses = self.misses.get() as f64;
        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }
}

/// Circuit breaker metrics
pub struct CircuitBreakerMetrics {
    /// State changes
    pub state_changes: Arc<RwLock<HashMap<String, Counter>>>,
    /// Current state (0 = closed, 1 = half-open, 2 = open)
    pub current_state: Arc<RwLock<HashMap<String, Gauge>>>,
    /// Successes
    pub successes: Arc<Counter>,
    /// Failures
    pub failures: Arc<Counter>,
    /// Rejections (when open)
    pub rejections: Arc<Counter>,
}

impl CircuitBreakerMetrics {
    /// Create new circuit breaker metrics
    pub fn new() -> Self {
        Self {
            state_changes: Arc::new(RwLock::new(HashMap::new())),
            current_state: Arc::new(RwLock::new(HashMap::new())),
            successes: Arc::new(Counter::new()),
            failures: Arc::new(Counter::new()),
            rejections: Arc::new(Counter::new()),
        }
    }

    /// Record a state change
    pub async fn record_state_change(&self, name: &str, new_state: &str) {
        let key = format!("{}:{}", name, new_state);
        {
            let mut changes = self.state_changes.write().await;
            changes
                .entry(key)
                .or_insert_with(Counter::new)
                .inc();
        }

        let state_value = match new_state {
            "closed" => 0.0,
            "half-open" => 1.0,
            "open" => 2.0,
            _ => 0.0,
        };

        {
            let mut states = self.current_state.write().await;
            states
                .entry(name.to_string())
                .or_insert_with(Gauge::new)
                .set(state_value);
        }
    }

    /// Record a success
    pub fn record_success(&self) {
        self.successes.inc();
    }

    /// Record a failure
    pub fn record_failure(&self) {
        self.failures.inc();
    }

    /// Record a rejection
    pub fn record_rejection(&self) {
        self.rejections.inc();
    }
}

impl Default for CircuitBreakerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Main Prometheus metrics registry
pub struct PrometheusMetrics {
    config: MetricsConfig,
    handle: MetricsHandle,
}

impl PrometheusMetrics {
    /// Create a new metrics registry
    pub fn new(config: MetricsConfig) -> Self {
        let handle = MetricsHandle {
            http: HttpMetrics::new(&config),
            database: DatabaseMetrics::new(&config),
            cache: CacheMetrics::new(&config),
            circuit_breaker: CircuitBreakerMetrics::new(),
        };

        Self { config, handle }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(MetricsConfig::default())
    }

    /// Get the metrics handle
    pub fn handle(&self) -> &MetricsHandle {
        &self.handle
    }

    /// Render metrics in Prometheus format
    pub async fn render(&self) -> String {
        let mut output = String::new();
        let prefix = &self.config.prefix;

        // HTTP metrics
        output.push_str(&format!(
            "# HELP {}_http_request_duration_seconds HTTP request duration in seconds\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_http_request_duration_seconds histogram\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_http_request_duration_seconds_count {}\n",
            prefix,
            self.handle.http.request_duration.get_count()
        ));
        output.push_str(&format!(
            "{}_http_request_duration_seconds_sum {}\n",
            prefix,
            self.handle.http.request_duration.get_sum()
        ));

        // Request buckets
        let mut cumulative = 0u64;
        for (bucket, count) in self.handle.http.request_duration.get_buckets() {
            cumulative += count;
            output.push_str(&format!(
                "{}_http_request_duration_seconds_bucket{{le=\"{}\"}} {}\n",
                prefix, bucket, cumulative
            ));
        }
        output.push_str(&format!(
            "{}_http_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            prefix,
            self.handle.http.request_duration.get_count()
        ));

        // In-progress requests
        output.push_str(&format!(
            "# HELP {}_http_requests_in_progress Number of HTTP requests in progress\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_http_requests_in_progress gauge\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_http_requests_in_progress {}\n",
            prefix,
            self.handle.http.requests_in_progress.get()
        ));

        // Cache metrics
        output.push_str(&format!(
            "# HELP {}_cache_hits_total Total cache hits\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_cache_hits_total counter\n", prefix));
        output.push_str(&format!(
            "{}_cache_hits_total {}\n",
            prefix,
            self.handle.cache.hits.get()
        ));

        output.push_str(&format!(
            "# HELP {}_cache_misses_total Total cache misses\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_cache_misses_total counter\n", prefix));
        output.push_str(&format!(
            "{}_cache_misses_total {}\n",
            prefix,
            self.handle.cache.misses.get()
        ));

        output.push_str(&format!(
            "# HELP {}_cache_hit_rate Cache hit rate\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_cache_hit_rate gauge\n", prefix));
        output.push_str(&format!(
            "{}_cache_hit_rate {}\n",
            prefix,
            self.handle.cache.hit_rate()
        ));

        // Circuit breaker metrics
        output.push_str(&format!(
            "# HELP {}_circuit_breaker_successes_total Total circuit breaker successes\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_circuit_breaker_successes_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_circuit_breaker_successes_total {}\n",
            prefix,
            self.handle.circuit_breaker.successes.get()
        ));

        output.push_str(&format!(
            "# HELP {}_circuit_breaker_failures_total Total circuit breaker failures\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_circuit_breaker_failures_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_circuit_breaker_failures_total {}\n",
            prefix,
            self.handle.circuit_breaker.failures.get()
        ));

        output.push_str(&format!(
            "# HELP {}_circuit_breaker_rejections_total Total circuit breaker rejections\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_circuit_breaker_rejections_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_circuit_breaker_rejections_total {}\n",
            prefix,
            self.handle.circuit_breaker.rejections.get()
        ));

        // Database metrics
        output.push_str(&format!(
            "# HELP {}_db_query_duration_seconds Database query duration in seconds\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_db_query_duration_seconds histogram\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_db_query_duration_seconds_count {}\n",
            prefix,
            self.handle.database.query_duration.get_count()
        ));
        output.push_str(&format!(
            "{}_db_query_duration_seconds_sum {}\n",
            prefix,
            self.handle.database.query_duration.get_sum()
        ));

        output.push_str(&format!(
            "# HELP {}_db_pool_connections Database pool connections\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_db_pool_connections gauge\n", prefix));
        output.push_str(&format!(
            "{}_db_pool_connections {}\n",
            prefix,
            self.handle.database.pool_connections.get()
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0.0);

        gauge.set(42.5);
        assert_eq!(gauge.get(), 42.5);

        gauge.inc();
        assert_eq!(gauge.get(), 43.5);

        gauge.dec();
        assert_eq!(gauge.get(), 42.5);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(vec![0.1, 0.5, 1.0]);

        histogram.observe(0.05);
        histogram.observe(0.3);
        histogram.observe(0.8);
        histogram.observe(2.0);

        assert_eq!(histogram.get_count(), 4);
        assert!((histogram.get_sum() - 3.15).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_http_metrics() {
        let config = MetricsConfig::default();
        let http = HttpMetrics::new(&config);

        http.record_request("GET", "/api/users", 200, Duration::from_millis(50), 1024).await;

        assert_eq!(http.request_duration.get_count(), 1);
    }

    #[test]
    fn test_cache_metrics() {
        let config = MetricsConfig::default();
        let cache = CacheMetrics::new(&config);

        cache.record_hit(Duration::from_millis(1));
        cache.record_hit(Duration::from_millis(1));
        cache.record_miss(Duration::from_millis(1));

        assert_eq!(cache.hits.get(), 2);
        assert_eq!(cache.misses.get(), 1);
        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_render_metrics() {
        let metrics = PrometheusMetrics::default_config();

        metrics.handle.cache.record_hit(Duration::from_millis(1));
        metrics.handle.circuit_breaker.record_success();

        let output = metrics.render().await;

        assert!(output.contains("copilot_cache_hits_total 1"));
        assert!(output.contains("copilot_circuit_breaker_successes_total 1"));
    }
}
