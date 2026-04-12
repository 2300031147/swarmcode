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
    #[cfg(target_os = "windows")]
    let (shell, flag) = ("powershell", "-Command");
    #[cfg(not(target_os = "windows"))]
    let (shell, flag) = ("bash", "-c");

    let mut builder = Command::new(shell);
    builder.arg(flag).arg(cmd);

    if let Some(dir) = cwd {
        builder.current_dir(dir);
    }

    let output = builder.output().map_err(|e| format!("Failed to run command: {e}"))?;

    Ok(TerminalOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}
