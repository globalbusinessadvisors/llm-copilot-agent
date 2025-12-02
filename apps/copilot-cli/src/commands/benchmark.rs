//! Benchmark command implementation
//!
//! This module provides the CLI `run` subcommand that invokes run_all_benchmarks()
//! and writes benchmark results to the canonical output directories.

use anyhow::Result;
use colored::Colorize;
use copilot_benchmarks::{
    run_all_benchmarks_with_config, run_benchmark, BenchmarkConfig, BenchmarkIo,
    MarkdownGenerator,
};

/// Run benchmarks subcommand
#[derive(Debug, Clone)]
pub enum BenchmarkCommand {
    /// Run all benchmarks
    Run {
        /// Only run benchmarks matching this filter (by ID prefix)
        filter: Option<String>,
        /// Run benchmarks in parallel
        parallel: bool,
        /// Output format (text, json)
        format: String,
        /// Skip writing results to disk
        no_write: bool,
    },
    /// List available benchmarks
    List,
    /// Show benchmark result details
    Show {
        /// Target ID to show
        target_id: String,
    },
}

/// Execute the benchmark command
pub async fn run(cmd: BenchmarkCommand, format: &str) -> Result<()> {
    match cmd {
        BenchmarkCommand::Run {
            filter,
            parallel,
            format: output_format,
            no_write,
        } => {
            run_benchmarks(filter, parallel, &output_format, no_write).await
        }
        BenchmarkCommand::List => list_benchmarks(format),
        BenchmarkCommand::Show { target_id } => show_benchmark(&target_id, format).await,
    }
}

async fn run_benchmarks(
    filter: Option<String>,
    parallel: bool,
    format: &str,
    no_write: bool,
) -> Result<()> {
    println!("{}", "Running benchmarks...".cyan().bold());

    let config = BenchmarkConfig {
        write_results: !no_write,
        generate_summary: !no_write,
        parallel,
        max_parallel: 4,
        filter: filter.clone(),
    };

    if let Some(ref f) = filter {
        println!("Filter: {}", f.yellow());
    }
    if parallel {
        println!("Mode: {}", "parallel".green());
    }

    let start = std::time::Instant::now();
    let results = run_all_benchmarks_with_config(config).await;
    let duration = start.elapsed();

    // Display results
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&results)?;
            println!("{}", json);
        }
        _ => {
            println!("\n{}", "Benchmark Results".green().bold());
            println!("{}", "─".repeat(60));

            let mut passed = 0;
            let mut failed = 0;

            for result in &results {
                let status = if result.is_success() {
                    passed += 1;
                    "✅".to_string()
                } else {
                    failed += 1;
                    "❌".to_string()
                };

                let duration_str = result
                    .duration_ms()
                    .map(|d| format!("{}ms", d))
                    .unwrap_or_else(|| "-".to_string());

                println!(
                    "{} {} {}",
                    status,
                    result.target_id.cyan(),
                    duration_str.dimmed()
                );
            }

            println!("{}", "─".repeat(60));
            println!(
                "Total: {} | Passed: {} | Failed: {} | Duration: {:?}",
                results.len().to_string().bold(),
                passed.to_string().green(),
                if failed > 0 {
                    failed.to_string().red()
                } else {
                    failed.to_string().green()
                },
                duration
            );

            if !no_write {
                let io = BenchmarkIo::new();
                println!(
                    "\n{} {}",
                    "Results written to:".dimmed(),
                    io.output_dir().display()
                );
            }
        }
    }

    Ok(())
}

fn list_benchmarks(format: &str) -> Result<()> {
    let targets = copilot_benchmarks::all_targets();

    match format {
        "json" => {
            let list: Vec<serde_json::Value> = targets
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id(),
                        "description": t.description()
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        _ => {
            println!("{}", "Available Benchmarks".green().bold());
            println!("{}", "─".repeat(60));

            for target in &targets {
                println!("  {} {}", "•".cyan(), target.id().bold());
                if let Some(desc) = target.description() {
                    println!("    {}", desc.dimmed());
                }
            }

            println!("{}", "─".repeat(60));
            println!("Total: {} benchmarks", targets.len().to_string().bold());
        }
    }

    Ok(())
}

async fn show_benchmark(target_id: &str, format: &str) -> Result<()> {
    println!("Running benchmark: {}", target_id.cyan().bold());

    let result = run_benchmark(target_id).await;

    match result {
        Some(result) => {
            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                _ => {
                    println!("\n{}", "Result".green().bold());
                    println!("{}", "─".repeat(40));
                    println!("Target: {}", result.target_id.cyan());
                    println!(
                        "Status: {}",
                        if result.is_success() {
                            "✅ Passed".green()
                        } else {
                            "❌ Failed".red()
                        }
                    );
                    println!("Timestamp: {}", result.timestamp);

                    if let Some(duration) = result.duration_ms() {
                        println!("Duration: {}ms", duration);
                    }

                    if let Some(error) = result.error() {
                        println!("Error: {}", error.red());
                    }

                    println!("\n{}", "Metrics:".bold());
                    println!("{}", serde_json::to_string_pretty(&result.metrics)?);
                }
            }
        }
        None => {
            eprintln!(
                "{}: Benchmark target '{}' not found",
                "Error".red().bold(),
                target_id
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
