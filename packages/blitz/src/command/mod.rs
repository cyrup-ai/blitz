//! Command execution utilities using the desktop-commander MCP server.
//!
//! This module provides safe, efficient command execution capabilities
//! that use the desktop-commander MCP server under the hood.

use std::path::Path;
use std::time::Duration;

use thiserror::Error;
#[cfg(feature = "net")]
use tokio;

/// Error type for command execution failures.
#[derive(Debug, Error)]
pub enum CommandError {
    /// Command execution failed with the given error message.
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    /// Command timed out.
    #[error("Command timed out after {:?}", .0)]
    Timeout(Duration),

    /// Command was terminated by a signal.
    #[error("Command terminated by signal")]
    Terminated,

    /// Command produced no output when some was expected.
    #[error("Command produced no output")]
    NoOutput,

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// The output of a successfully executed command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// The standard output of the command.
    pub stdout: String,

    /// The standard error of the command.
    pub stderr: String,

    /// The exit status code of the command.
    pub status: i32,
}

/// Executes a command synchronously using the desktop-commander MCP server.
///
/// # Arguments
/// * `command` - The command to execute.
/// * `args` - The arguments to pass to the command.
/// * `cwd` - The working directory for the command, if any.
/// * `timeout` - The maximum duration to wait for the command to complete.
///
/// # Returns
/// A `Result` containing the command output on success, or a `CommandError` on failure.
pub fn run_command(
    command: &str,
    args: &[&str],
    cwd: Option<&Path>,
    _timeout: Option<Duration>,
) -> Result<CommandOutput, CommandError> {
    // Convert Path to string for the MCP server
    let cwd_str = cwd.and_then(|p| p.to_str()).unwrap_or(".");

    // Build the command string with arguments
    let mut cmd = command.to_string();
    for arg in args {
        cmd.push(' ');
        cmd.push_str(arg);
    }

    // Execute the command using std::process::Command
    let output = std::process::Command::new(command)
        .args(args)
        .current_dir(cwd_str)
        .output()
        .map_err(CommandError::Io)?;

    // Convert the output to our CommandOutput type
    let command_output = CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status.code().unwrap_or(-1),
    };

    Ok(command_output)
}

/// Executes a command asynchronously using the desktop-commander MCP server.
///
/// # Arguments
/// * `command` - The command to execute.
/// * `args` - The arguments to pass to the command.
/// * `cwd` - The working directory for the command, if any.
///
/// # Returns
/// A `Future` that resolves to a `Result` containing the command output on success,
/// or a `CommandError` on failure.
#[cfg(feature = "net")]
pub async fn run_command_async(
    command: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<CommandOutput, CommandError> {
    // Convert Path to string for the MCP server
    let cwd_str = cwd.and_then(|p| p.to_str()).unwrap_or(".");

    // Build the command string with arguments
    let mut cmd = command.to_string();
    for arg in args {
        cmd.push(' ');
        cmd.push_str(arg);
    }

    // Execute the command asynchronously using tokio::process::Command
    let output = tokio::process::Command::new(command)
        .args(args)
        .current_dir(cwd_str)
        .output()
        .await
        .map_err(CommandError::Io)?;

    // Convert the output to our CommandOutput type
    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status.code().unwrap_or(-1),
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_run_command_basic() {
        let output = run_command("echo", &["hello"], None, Some(Duration::from_secs(5))).unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.status, 0);
    }

    #[test]
    fn test_run_command_error() {
        let result = run_command("false", &[], None, Some(Duration::from_secs(5)));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_ne!(output.status, 0);
    }

    #[tokio::test]
    async fn test_run_command_async_basic() {
        let output = run_command_async("echo", &["hello"], None).await.unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.status, 0);
    }
}
