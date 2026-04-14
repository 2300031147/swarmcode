use std::sync::OnceLock;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

pub trait SubprocessExt {
    fn set_no_window(&mut self) -> &mut Self;
}

impl SubprocessExt for Command {
    fn set_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            self.creation_flags(CREATE_NO_WINDOW_FLAG);
        }
        self
    }
}

impl SubprocessExt for std::process::Command {
    fn set_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW_FLAG);
        }
        self
    }
}

/// Resolve the user's full PATH by running a login shell.
/// Ported from Goose.
#[cfg(not(windows))]
fn resolve_login_shell_path() -> Option<String> {
    use std::path::PathBuf;
    use std::process::Stdio;

    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| PathBuf::from(s).is_file())
        .unwrap_or_else(|| {
            if PathBuf::from("/bin/bash").is_file() {
                "/bin/bash".to_string()
            } else {
                "sh".to_string()
            }
        });

    std::process::Command::new(&shell)
        .args(["-l", "-i", "-c", "echo $PATH"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
            } else {
                None
            }
        })
}

/// Returns the user's full login shell PATH, resolved once and cached.
#[cfg(not(windows))]
pub fn user_login_path() -> Option<&'static str> {
    static CACHED: OnceLock<Option<String>> = OnceLock::new();
    CACHED.get_or_init(resolve_login_shell_path).as_deref()
}

/// Merge the login shell PATH with the current process PATH.
#[cfg(not(windows))]
pub fn merged_path() -> Option<String> {
    let login = user_login_path()?;
    let current = std::env::var("PATH").unwrap_or_default();
    if current.is_empty() {
        return Some(login.to_string());
    }
    let login_entries: Vec<&str> = login.split(':').collect();
    let mut seen: std::collections::HashSet<&str> = login_entries.iter().copied().collect();
    let mut merged = login_entries;
    for entry in current.split(':') {
        if seen.insert(entry) {
            merged.push(entry);
        }
    }
    Some(merged.join(":"))
}

pub const SENSITIVE_ENV_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GITHUB_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "STRIPE_API_KEY",
    "DATABASE_URL",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
    "GROQ_API_KEY",
    "MISTRAL_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "GCP_SERVICE_ACCOUNT",
    "NPM_TOKEN",
    "PYPI_TOKEN",
    "CARGO_REGISTRY_TOKEN",
];

/// Scrub sensitive environment variables from a command.
pub fn scrub_env<C: SubprocessCommand>(command: &mut C) {
    for key in SENSITIVE_ENV_VARS {
        command.env_remove(key);
    }
}

// ── Background Task Management ───────────────────────────────────────────

use std::collections::HashMap;
use std::sync::Mutex;
use std::process::Child;

lazy_static::lazy_static! {
    static ref BACKGROUND_TASKS: Mutex<HashMap<String, Child>> = Mutex::new(HashMap::new());
}

/// Register a background child process for tracking.
pub fn register_background_task(id: String, child: Child) {
    if let Ok(mut tasks) = BACKGROUND_TASKS.lock() {
        tasks.insert(id, child);
    }
}

/// Terminate a background task by ID.
pub fn kill_background_task(id: &str) -> bool {
    if let Ok(mut tasks) = BACKGROUND_TASKS.lock() {
        if let Some(mut child) = tasks.remove(id) {
            return child.kill().is_ok();
        }
    }
    false
}

/// List currently tracked background task IDs.
pub fn list_background_tasks() -> Vec<String> {
    if let Ok(tasks) = BACKGROUND_TASKS.lock() {
        tasks.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

pub trait SubprocessCommand {
    fn env_remove(&mut self, key: &str) -> &mut Self;
}

impl SubprocessCommand for Command {
    fn env_remove(&mut self, key: &str) -> &mut Self {
        self.env_remove(key)
    }
}

impl SubprocessCommand for std::process::Command {
    fn env_remove(&mut self, key: &str) -> &mut Self {
        self.env_remove(key)
    }
}
