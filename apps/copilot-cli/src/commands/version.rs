//! Version information command

use anyhow::Result;
use colored::Colorize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run(all: bool, format: &str) -> Result<()> {
    let version_info = VersionInfo {
        cli_version: VERSION.to_string(),
        sdk_version: copilot_sdk::VERSION.to_string(),
        build_date: option_env!("BUILD_DATE").map(String::from),
        git_commit: option_env!("GIT_COMMIT").map(String::from),
        rust_version: option_env!("RUST_VERSION").map(String::from),
    };

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&version_info)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&version_info)?);
        }
        _ => {
            println!("{} {}", "copilot".cyan().bold(), VERSION.green());

            if all {
                println!();
                println!("{}", "Components:".bold());
                println!("  SDK: {}", version_info.sdk_version.green());

                if let Some(date) = &version_info.build_date {
                    println!("  Build Date: {}", date);
                }
                if let Some(commit) = &version_info.git_commit {
                    println!("  Git Commit: {}", &commit[..7.min(commit.len())]);
                }
                if let Some(rust) = &version_info.rust_version {
                    println!("  Rust: {}", rust);
                }
            }
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct VersionInfo {
    cli_version: String,
    sdk_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_version: Option<String>,
}
