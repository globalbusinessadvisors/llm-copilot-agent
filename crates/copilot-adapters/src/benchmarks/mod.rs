//! Benchmark Runtime Integrations
//!
//! This module provides runtime integrations for benchmark services.
//! These are NOT compile-time dependencies - they use HTTP/SDK-based
//! or file-based ingestion only.
//!
//! Phase 2B: Runtime benchmark integrations

pub mod test_bench;
pub mod benchmark_exchange;

pub use test_bench::{TestBenchRuntimeAdapter, TestBenchRuntimeClient};
pub use benchmark_exchange::{BenchmarkExchangeAdapter, BenchmarkExchangeClient};

use serde::{Deserialize, Serialize};

/// Common benchmark result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_id: String,
    pub name: String,
    pub score: f64,
    pub metrics: std::collections::HashMap<String, f64>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Benchmark corpus reference (file-based or SDK retrieval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCorpus {
    pub corpus_id: String,
    pub name: String,
    pub version: String,
    pub source: CorpusSource,
    pub item_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorpusSource {
    /// File-based corpus (local or remote URL)
    File { path: String },
    /// SDK-based retrieval
    SDK { endpoint: String },
    /// Remote HTTP endpoint
    HTTP { url: String },
}

/// Configuration for benchmark runtime integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRuntimeConfig {
    /// LLM-Test-Bench service URL
    pub test_bench_url: String,
    /// LLM-Benchmark-Exchange service URL
    pub benchmark_exchange_url: String,
    /// Local corpus cache directory
    pub corpus_cache_dir: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for BenchmarkRuntimeConfig {
    fn default() -> Self {
        Self {
            test_bench_url: "http://localhost:8110".to_string(),
            benchmark_exchange_url: "http://localhost:8111".to_string(),
            corpus_cache_dir: "/tmp/benchmark-corpus".to_string(),
            timeout_seconds: 60,
        }
    }
}

/// Unified benchmark runtime hub
pub struct BenchmarkRuntimeHub {
    pub test_bench: TestBenchRuntimeClient,
    pub benchmark_exchange: BenchmarkExchangeClient,
}

impl BenchmarkRuntimeHub {
    pub fn new(config: BenchmarkRuntimeConfig) -> Self {
        Self {
            test_bench: TestBenchRuntimeClient::new(&config.test_bench_url),
            benchmark_exchange: BenchmarkExchangeClient::new(
                &config.benchmark_exchange_url,
                &config.corpus_cache_dir,
            ),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(BenchmarkRuntimeConfig::default())
    }
}
