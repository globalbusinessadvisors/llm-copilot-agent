//! Canonical Benchmark Module
//!
//! This module re-exports the copilot-benchmarks crate for use within
//! the workspace. The actual implementation is in crates/copilot-benchmarks.
//!
//! # Canonical Structure
//!
//! ```text
//! benchmarks/
//! ├── mod.rs           (this file - re-exports)
//! ├── result.rs        (see crates/copilot-benchmarks/src/result.rs)
//! ├── markdown.rs      (see crates/copilot-benchmarks/src/markdown.rs)
//! ├── io.rs            (see crates/copilot-benchmarks/src/io.rs)
//! ├── adapters/        (see crates/copilot-benchmarks/src/adapters/)
//! └── output/
//!     ├── raw/         (individual benchmark results)
//!     └── summary.md   (aggregated summary)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! // Import from the crate directly
//! use copilot_benchmarks::{run_all_benchmarks, BenchmarkResult, BenchTarget};
//!
//! // Or via CLI
//! // copilot run                    # Run all benchmarks
//! // copilot benchmark run          # Same as above
//! // copilot benchmark list         # List available benchmarks
//! // copilot benchmark show <id>    # Show specific benchmark
//! ```

// Re-export everything from the crate
pub use copilot_benchmarks::*;
