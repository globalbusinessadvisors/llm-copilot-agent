//! Code execution within sandboxes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{E2BError, Result};

/// Result of code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution ID
    pub id: String,

    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Exit code (0 = success)
    pub exit_code: i32,

    /// Execution duration
    pub duration_ms: u64,

    /// Start timestamp
    pub started_at: DateTime<Utc>,

    /// End timestamp
    pub ended_at: DateTime<Utc>,

    /// Whether execution was successful
    pub success: bool,

    /// Any execution errors
    pub error: Option<ExecutionError>,
}

impl ExecutionResult {
    /// Create a successful execution result
    pub fn success(stdout: String, stderr: String, duration_ms: u64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            stdout,
            stderr,
            exit_code: 0,
            duration_ms,
            started_at: now - chrono::Duration::milliseconds(duration_ms as i64),
            ended_at: now,
            success: true,
            error: None,
        }
    }

    /// Create a failed execution result
    pub fn failure(exit_code: i32, stderr: String, error: ExecutionError) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            stdout: String::new(),
            stderr,
            exit_code,
            duration_ms: 0,
            started_at: now,
            ended_at: now,
            success: false,
            error: Some(error),
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.success && self.exit_code == 0
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Execution error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<String>,
}

impl ExecutionError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// Code executor for sandbox environments
pub struct CodeExecutor {
    timeout: Duration,
}

impl CodeExecutor {
    /// Create a new code executor
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Execute Python code
    pub async fn execute_python(&self, code: &str) -> Result<ExecutionResult> {
        info!("Executing Python code");
        debug!("Code: {}", code);

        let start = std::time::Instant::now();

        // In production, this would call E2B's sandbox execution API
        // For now, we simulate execution
        let (stdout, stderr, _exit_code) = self.simulate_execution(code, "python").await?;

        let duration = start.elapsed();

        Ok(ExecutionResult::success(
            stdout,
            stderr,
            duration.as_millis() as u64,
        ))
    }

    /// Execute JavaScript/Node.js code
    pub async fn execute_nodejs(&self, code: &str) -> Result<ExecutionResult> {
        info!("Executing Node.js code");
        debug!("Code: {}", code);

        let start = std::time::Instant::now();

        let (stdout, stderr, _exit_code) = self.simulate_execution(code, "node").await?;

        let duration = start.elapsed();

        Ok(ExecutionResult::success(
            stdout,
            stderr,
            duration.as_millis() as u64,
        ))
    }

    /// Execute shell command
    pub async fn execute_shell(&self, command: &str) -> Result<ExecutionResult> {
        info!("Executing shell command");
        debug!("Command: {}", command);

        let start = std::time::Instant::now();

        let (stdout, stderr, _exit_code) = self.simulate_execution(command, "bash").await?;

        let duration = start.elapsed();

        Ok(ExecutionResult::success(
            stdout,
            stderr,
            duration.as_millis() as u64,
        ))
    }

    /// Execute generic code with specified runtime
    pub async fn execute(&self, code: &str, runtime: &str) -> Result<ExecutionResult> {
        match runtime {
            "python" | "python3" => self.execute_python(code).await,
            "node" | "nodejs" | "javascript" => self.execute_nodejs(code).await,
            "bash" | "shell" | "sh" => self.execute_shell(code).await,
            _ => Err(E2BError::execution(format!("Unsupported runtime: {}", runtime))),
        }
    }

    /// Simulate code execution (for development/testing)
    async fn simulate_execution(
        &self,
        code: &str,
        _runtime: &str,
    ) -> Result<(String, String, i32)> {
        // Simulate some execution time
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Simple simulation: echo back a success message
        let stdout = format!("[Simulated] Code executed successfully\n{} bytes of code processed", code.len());
        let stderr = String::new();
        let exit_code = 0;

        Ok((stdout, stderr, exit_code))
    }
}

/// File operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    pub path: String,
    pub success: bool,
    pub message: String,
}

/// Filesystem operations within sandbox
pub struct SandboxFilesystem;

impl SandboxFilesystem {
    /// Write content to a file
    pub async fn write_file(path: &str, content: &str) -> Result<FileOperationResult> {
        info!("Writing to file: {}", path);

        // In production, this would call E2B's filesystem API
        Ok(FileOperationResult {
            path: path.to_string(),
            success: true,
            message: format!("Written {} bytes", content.len()),
        })
    }

    /// Read content from a file
    pub async fn read_file(path: &str) -> Result<String> {
        info!("Reading file: {}", path);

        // In production, this would call E2B's filesystem API
        Ok(format!("[Simulated content of {}]", path))
    }

    /// List directory contents
    pub async fn list_directory(path: &str) -> Result<Vec<String>> {
        info!("Listing directory: {}", path);

        // In production, this would call E2B's filesystem API
        Ok(vec![
            format!("{}/file1.txt", path),
            format!("{}/file2.py", path),
            format!("{}/subdir/", path),
        ])
    }

    /// Delete a file
    pub async fn delete_file(path: &str) -> Result<FileOperationResult> {
        info!("Deleting file: {}", path);

        Ok(FileOperationResult {
            path: path.to_string(),
            success: true,
            message: "File deleted".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success(
            "Hello, World!".to_string(),
            String::new(),
            100,
        );

        assert!(result.is_success());
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "Hello, World!");
    }

    #[test]
    fn test_execution_result_failure() {
        let error = ExecutionError::new("EXEC_FAILED", "Syntax error")
            .with_details("Line 1: unexpected token");

        let result = ExecutionResult::failure(1, "Error on line 1".to_string(), error);

        assert!(!result.is_success());
        assert_eq!(result.exit_code, 1);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_code_executor() {
        let executor = CodeExecutor::new(Duration::from_secs(30));

        let result = executor.execute_python("print('hello')").await.unwrap();
        assert!(result.is_success());

        let result = executor.execute_nodejs("console.log('hello')").await.unwrap();
        assert!(result.is_success());
    }
}
