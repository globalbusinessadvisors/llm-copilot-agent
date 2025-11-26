//! Project initialization command

use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input};
use std::path::Path;

pub async fn run(path: &str, template: &str) -> Result<()> {
    let project_path = path;
    let project_dir = Path::new(&project_path);

    // Check if directory exists and has content
    if project_dir.exists() && project_dir.read_dir()?.next().is_some() {
        let confirmed = Confirm::new()
            .with_prompt("Directory is not empty. Continue anyway?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    // Create directory if needed
    if !project_dir.exists() {
        std::fs::create_dir_all(project_dir)?;
    }

    // Use provided template
    let template_name = template;

    // Get project name
    let project_name: String = Input::new()
        .with_prompt("Project name")
        .default(
            project_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "my-project".to_string()),
        )
        .interact_text()?;

    println!();
    println!(
        "{} project '{}' with template '{}'...",
        "Initializing".green(),
        project_name.cyan(),
        template_name.cyan()
    );

    // Create configuration files based on template
    match template_name {
        "rust" => create_rust_template(project_dir, &project_name)?,
        "python" => create_python_template(project_dir, &project_name)?,
        "typescript" => create_typescript_template(project_dir, &project_name)?,
        "minimal" => create_minimal_template(project_dir, &project_name)?,
        _ => create_default_template(project_dir, &project_name)?,
    }

    println!();
    println!("{} Project initialized successfully!", "✓".green());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. cd {}", project_path);
    println!("  2. Review and update .copilot/config.toml");
    println!("  3. Run 'copilot chat' to start using the assistant");

    Ok(())
}

fn create_default_template(dir: &Path, name: &str) -> Result<()> {
    let copilot_dir = dir.join(".copilot");
    std::fs::create_dir_all(&copilot_dir)?;

    // Main config
    let config = format!(
        r#"# Copilot Configuration
# Project: {name}

[project]
name = "{name}"
description = ""

[assistant]
# Default model to use
model = "gpt-4"

# System prompt customization
system_prompt = """
You are a helpful AI assistant for the {name} project.
Help with code reviews, documentation, debugging, and general development tasks.
"""

[context]
# Files and patterns to include in context
include = [
    "src/**/*.rs",
    "src/**/*.py",
    "src/**/*.ts",
    "*.md",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
]

# Files and patterns to exclude
exclude = [
    "target/",
    "node_modules/",
    ".git/",
    "*.lock",
]

[workflows]
# Enable built-in workflows
code_review = true
documentation = true
testing = true

[sandbox]
# Sandbox settings for code execution
enabled = true
timeout = 30
memory_limit = "512m"
"#
    );

    std::fs::write(copilot_dir.join("config.toml"), config)?;
    println!("  {} .copilot/config.toml", "Created".green());

    // Gitignore additions
    let gitignore_content = r#"
# Copilot
.copilot/cache/
.copilot/sessions/
.copilot/*.log
"#;

    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        let mut content = std::fs::read_to_string(&gitignore_path)?;
        if !content.contains(".copilot/") {
            content.push_str(gitignore_content);
            std::fs::write(&gitignore_path, content)?;
            println!("  {} .gitignore (appended)", "Updated".green());
        }
    } else {
        std::fs::write(&gitignore_path, gitignore_content.trim_start())?;
        println!("  {} .gitignore", "Created".green());
    }

    Ok(())
}

fn create_rust_template(dir: &Path, name: &str) -> Result<()> {
    create_default_template(dir, name)?;

    let copilot_dir = dir.join(".copilot");

    // Rust-specific workflow
    let workflow = r#"name: rust-review
description: Rust code review workflow

steps:
  - id: clippy
    type: command
    command: cargo clippy --all-targets --all-features -- -D warnings

  - id: fmt-check
    type: command
    command: cargo fmt --check

  - id: test
    type: command
    command: cargo test

  - id: ai-review
    type: llm
    prompt: |
      Review the following Rust code changes for:
      - Memory safety concerns
      - Error handling patterns
      - Performance implications
      - Idiomatic Rust usage

      {{changes}}
"#;

    std::fs::create_dir_all(copilot_dir.join("workflows"))?;
    std::fs::write(copilot_dir.join("workflows/rust-review.yaml"), workflow)?;
    println!("  {} .copilot/workflows/rust-review.yaml", "Created".green());

    Ok(())
}

fn create_python_template(dir: &Path, name: &str) -> Result<()> {
    create_default_template(dir, name)?;

    let copilot_dir = dir.join(".copilot");

    // Python-specific workflow
    let workflow = r#"name: python-review
description: Python code review workflow

steps:
  - id: ruff-check
    type: command
    command: ruff check .

  - id: ruff-format
    type: command
    command: ruff format --check .

  - id: mypy
    type: command
    command: mypy .
    continue_on_error: true

  - id: pytest
    type: command
    command: pytest

  - id: ai-review
    type: llm
    prompt: |
      Review the following Python code changes for:
      - Type safety and type hints
      - Error handling patterns
      - PEP 8 compliance
      - Security considerations

      {{changes}}
"#;

    std::fs::create_dir_all(copilot_dir.join("workflows"))?;
    std::fs::write(copilot_dir.join("workflows/python-review.yaml"), workflow)?;
    println!(
        "  {} .copilot/workflows/python-review.yaml",
        "Created".green()
    );

    Ok(())
}

fn create_typescript_template(dir: &Path, name: &str) -> Result<()> {
    create_default_template(dir, name)?;

    let copilot_dir = dir.join(".copilot");

    // TypeScript-specific workflow
    let workflow = r#"name: typescript-review
description: TypeScript code review workflow

steps:
  - id: typecheck
    type: command
    command: npx tsc --noEmit

  - id: lint
    type: command
    command: npx eslint .

  - id: test
    type: command
    command: npm test

  - id: ai-review
    type: llm
    prompt: |
      Review the following TypeScript code changes for:
      - Type safety and proper typing
      - React best practices (if applicable)
      - Error handling
      - Security considerations

      {{changes}}
"#;

    std::fs::create_dir_all(copilot_dir.join("workflows"))?;
    std::fs::write(
        copilot_dir.join("workflows/typescript-review.yaml"),
        workflow,
    )?;
    println!(
        "  {} .copilot/workflows/typescript-review.yaml",
        "Created".green()
    );

    Ok(())
}

fn create_minimal_template(dir: &Path, name: &str) -> Result<()> {
    let copilot_dir = dir.join(".copilot");
    std::fs::create_dir_all(&copilot_dir)?;

    let config = format!(
        r#"# Minimal Copilot Configuration
[project]
name = "{name}"

[assistant]
model = "gpt-4"
"#
    );

    std::fs::write(copilot_dir.join("config.toml"), config)?;
    println!("  {} .copilot/config.toml", "Created".green());

    Ok(())
}
