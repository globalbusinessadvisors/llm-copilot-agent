//! Shell completions generation

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn run(shell_str: &str) -> Result<()> {
    let shell = match shell_str {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        _ => {
            anyhow::bail!("Unsupported shell: {}", shell_str);
        }
    };

    let mut cmd = crate::Cli::command();
    generate(shell, &mut cmd, "copilot", &mut io::stdout());

    Ok(())
}

