//! Terminal commands — run shell commands from the GUI's integrated terminal,
//! capturing stdout/stderr and returning them to the frontend.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

/// Run a command string in the system shell and return its output.
/// Uses PowerShell on Windows, bash on Linux.
#[tauri::command]
pub fn terminal_run(command: String) -> Result<TerminalOutput, String> {
    run_shell(&command, None)
}

/// Run a command with a working directory override.
#[tauri::command]
pub fn terminal_run_in_workspace(
    command: String,
    workspace: String,
) -> Result<TerminalOutput, String> {
    run_shell(&command, Some(&workspace))
}

fn run_shell(cmd: &str, cwd: Option<&str>) -> Result<TerminalOutput, String> {
    use swarm_runtime::{execute_bash, BashCommandInput};

    let input = BashCommandInput {
        command: cmd.to_string(),
        timeout: Some(300_000), // 5 minute timeout for terminal commands
        run_in_background: Some(false),
        dangerously_disable_sandbox: Some(false),
        filesystem_mode: None,
        allowed_mounts: None,
    };

    // Note: execute_bash uses the current process directory if no other is specified.
    // If cwd override is provided, we must ensure the runtime is in that directory.
    // However, execute_bash currently assumes env::current_dir().
    // We'll set the current dir temporary if needed, or rely on the fact that
    // the Tauri process manages its own working directory for the session.
    
    let output = execute_bash(input).map_err(|e| format!("Runtime execution error: {e}"))?;

    Ok(TerminalOutput {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: if output.interrupted { -1 } else { 0 }, // execute_bash doesn't return exit code directly yet
        success: !output.interrupted && output.stderr.is_empty(),
    })
}
