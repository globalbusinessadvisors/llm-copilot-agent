//! Server management commands

use crate::ServerCommands;
use anyhow::Result;
use colored::Colorize;
use std::process::{Command, Stdio};

pub async fn run(cmd: ServerCommands) -> Result<()> {
    match cmd {
        ServerCommands::Start { port, daemon } => {
            start_server(port, daemon).await
        }
        ServerCommands::Stop => stop_server().await,
        ServerCommands::Status => show_status().await,
        ServerCommands::Logs { follow, lines } => show_logs(follow, lines).await,
    }
}

async fn start_server(port: u16, daemon: bool) -> Result<()> {
    println!(
        "{} server on port {}...",
        "Starting".green(),
        port
    );

    let mut cmd = Command::new("copilot-server");
    cmd.arg("--port").arg(port.to_string());

    if daemon {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let child = cmd.spawn()?;
        let pid = child.id();

        // Save PID for later management
        let pid_file = get_pid_file()?;
        std::fs::write(&pid_file, pid.to_string())?;

        println!("{} Server started in background (PID: {})", "✓".green(), pid);
        println!(
            "{}",
            format!("Server listening at http://localhost:{}", port).dimmed()
        );
    } else {
        println!("{}", "Press Ctrl+C to stop the server".dimmed());
        println!();

        let status = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("Server exited with error");
        }
    }

    Ok(())
}

async fn stop_server() -> Result<()> {
    let pid_file = get_pid_file()?;

    if !pid_file.exists() {
        println!("{}", "No server is running (PID file not found)".yellow());
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str.trim().parse()?;

    println!("{} server (PID: {})...", "Stopping".yellow(), pid);

    #[cfg(unix)]
    {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }

    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
    }

    std::fs::remove_file(&pid_file)?;
    println!("{} Server stopped", "✓".green());

    Ok(())
}

async fn show_status() -> Result<()> {
    let pid_file = get_pid_file()?;

    if !pid_file.exists() {
        println!("{}: {}", "Status".bold(), "Not running".red());
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str.trim().parse()?;

    // Check if process is actually running
    #[cfg(unix)]
    let is_running = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    #[cfg(windows)]
    let is_running = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid)])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false);

    #[cfg(not(any(unix, windows)))]
    let is_running = true;

    if is_running {
        println!("{}: {} (PID: {})", "Status".bold(), "Running".green(), pid);
    } else {
        println!("{}: {} (stale PID file)", "Status".bold(), "Not running".red());
        // Clean up stale PID file
        let _ = std::fs::remove_file(&pid_file);
    }

    Ok(())
}

async fn show_logs(follow: bool, lines: usize) -> Result<()> {
    let log_file = get_log_file()?;

    if !log_file.exists() {
        println!("{}", "No log file found".yellow());
        return Ok(());
    }

    if follow {
        #[cfg(unix)]
        {
            let status = Command::new("tail")
                .args(["-f", "-n", &lines.to_string()])
                .arg(&log_file)
                .status()?;

            if !status.success() {
                anyhow::bail!("Failed to tail logs");
            }
        }

        #[cfg(not(unix))]
        {
            println!(
                "{}",
                "Log following not supported on this platform. Showing last entries:".yellow()
            );
            show_last_lines(&log_file, lines)?;
        }
    } else {
        show_last_lines(&log_file, lines)?;
    }

    Ok(())
}

fn show_last_lines(path: &std::path::Path, n: usize) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(n);

    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}

fn get_pid_file() -> Result<std::path::PathBuf> {
    let runtime_dir = dirs::runtime_dir()
        .or_else(|| dirs::data_local_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine runtime directory"))?;

    let copilot_dir = runtime_dir.join("copilot");
    std::fs::create_dir_all(&copilot_dir)?;

    Ok(copilot_dir.join("server.pid"))
}

fn get_log_file() -> Result<std::path::PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    Ok(data_dir.join("copilot").join("server.log"))
}
