//! Benchmark I/O operations
//!
//! This module handles reading and writing benchmark results to the
//! canonical output directories.

use crate::result::BenchmarkResult;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during benchmark I/O operations
#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Output directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Invalid file format: {0}")]
    InvalidFormat(String),
}

/// Result type for I/O operations
pub type IoResult<T> = Result<T, IoError>;

/// Default output directory relative to project root
pub const DEFAULT_OUTPUT_DIR: &str = "benchmarks/output";

/// Default raw output directory for individual results
pub const DEFAULT_RAW_DIR: &str = "benchmarks/output/raw";

/// Benchmark I/O handler for reading and writing results
pub struct BenchmarkIo {
    output_dir: PathBuf,
    raw_dir: PathBuf,
}

impl Default for BenchmarkIo {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkIo {
    /// Create a new BenchmarkIo with default paths
    pub fn new() -> Self {
        Self {
            output_dir: PathBuf::from(DEFAULT_OUTPUT_DIR),
            raw_dir: PathBuf::from(DEFAULT_RAW_DIR),
        }
    }

    /// Create a BenchmarkIo with custom paths
    pub fn with_paths(output_dir: impl Into<PathBuf>, raw_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            raw_dir: raw_dir.into(),
        }
    }

    /// Ensure output directories exist
    pub fn ensure_directories(&self) -> IoResult<()> {
        fs::create_dir_all(&self.output_dir)?;
        fs::create_dir_all(&self.raw_dir)?;
        Ok(())
    }

    /// Get the output directory path
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Get the raw output directory path
    pub fn raw_dir(&self) -> &Path {
        &self.raw_dir
    }

    /// Write a single benchmark result to the raw directory
    pub fn write_result(&self, result: &BenchmarkResult) -> IoResult<PathBuf> {
        self.ensure_directories()?;

        // Create filename from target_id and timestamp
        let safe_id = result.target_id.replace("::", "_").replace('/', "_");
        let timestamp = result.timestamp.format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.json", safe_id, timestamp);
        let path = self.raw_dir.join(&filename);

        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, result)?;

        Ok(path)
    }

    /// Write multiple benchmark results to the raw directory
    pub fn write_results(&self, results: &[BenchmarkResult]) -> IoResult<Vec<PathBuf>> {
        results.iter().map(|r| self.write_result(r)).collect()
    }

    /// Write all results to a single combined file
    pub fn write_combined(&self, results: &[BenchmarkResult], filename: &str) -> IoResult<PathBuf> {
        self.ensure_directories()?;

        let path = self.output_dir.join(filename);
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, results)?;

        Ok(path)
    }

    /// Read a benchmark result from a file
    pub fn read_result(&self, path: impl AsRef<Path>) -> IoResult<BenchmarkResult> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        let result = serde_json::from_reader(reader)?;
        Ok(result)
    }

    /// Read all results from the raw directory
    pub fn read_all_results(&self) -> IoResult<Vec<BenchmarkResult>> {
        let mut results = Vec::new();

        if !self.raw_dir.exists() {
            return Ok(results);
        }

        for entry in fs::read_dir(&self.raw_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match self.read_result(&path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Sort by timestamp
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(results)
    }

    /// Read combined results file
    pub fn read_combined(&self, filename: &str) -> IoResult<Vec<BenchmarkResult>> {
        let path = self.output_dir.join(filename);
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let results = serde_json::from_reader(reader)?;
        Ok(results)
    }

    /// Clean up old result files, keeping only the most recent N
    pub fn cleanup_old_results(&self, keep_count: usize) -> IoResult<usize> {
        let mut results = self.read_all_results()?;

        if results.len() <= keep_count {
            return Ok(0);
        }

        // Results are sorted by timestamp, remove oldest
        let to_remove = results.len() - keep_count;
        results.truncate(to_remove);

        let mut removed = 0;
        for result in &results {
            let safe_id = result.target_id.replace("::", "_").replace('/', "_");
            let timestamp = result.timestamp.format("%Y%m%d_%H%M%S");
            let filename = format!("{}_{}.json", safe_id, timestamp);
            let path = self.raw_dir.join(&filename);

            if path.exists() {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Get the path for the summary markdown file
    pub fn summary_path(&self) -> PathBuf {
        self.output_dir.join("summary.md")
    }

    /// Write content to the summary file
    pub fn write_summary(&self, content: &str) -> IoResult<PathBuf> {
        self.ensure_directories()?;

        let path = self.summary_path();
        let mut file = File::create(&path)?;
        file.write_all(content.as_bytes())?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn temp_io() -> BenchmarkIo {
        let base = temp_dir().join(format!("bench_io_test_{}", uuid::Uuid::new_v4()));
        BenchmarkIo::with_paths(base.join("output"), base.join("output/raw"))
    }

    #[test]
    fn test_ensure_directories() {
        let io = temp_io();
        assert!(io.ensure_directories().is_ok());
        assert!(io.output_dir().exists());
        assert!(io.raw_dir().exists());
    }

    #[test]
    fn test_write_and_read_result() {
        let io = temp_io();
        let result = BenchmarkResult::success("test::target", 100);

        let path = io.write_result(&result).unwrap();
        assert!(path.exists());

        let read_result = io.read_result(&path).unwrap();
        assert_eq!(read_result.target_id, result.target_id);
    }
}
