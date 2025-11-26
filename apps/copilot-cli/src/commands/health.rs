//! Health check command

use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;

pub async fn run(api_url: &str, detailed: bool, format: &str) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .build()?;

    let health = client.health_check(detailed).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&health)?);
        }
        _ => {
            let status_str = match health.status.as_str() {
                "healthy" => "Healthy".green(),
                "degraded" => "Degraded".yellow(),
                _ => "Unhealthy".red(),
            };

            println!("{}: {}", "Status".bold(), status_str);
            println!("{}: {}", "Version".bold(), health.version);

            if detailed {
                if !health.services.is_empty() {
                    println!();
                    println!("{}", "Services:".bold());
                    for (name, svc_health) in &health.services {
                        let status_icon = match svc_health.status.as_str() {
                            "healthy" => "✓".green(),
                            "degraded" => "!".yellow(),
                            _ => "✗".red(),
                        };
                        let latency_str = svc_health
                            .latency_ms
                            .map(|ms| format!(" ({}ms)", ms))
                            .unwrap_or_default();
                        println!("  {} {}: {}{}", status_icon, name, svc_health.status, latency_str);
                    }
                }

                if let Some(uptime) = health.uptime {
                    println!();
                    println!("{}: {}s", "Uptime".bold(), uptime);
                }
            }
        }
    }

    Ok(())
}
