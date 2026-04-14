use chrono::Utc;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const LARGE_TEXT_THRESHOLD_BYTES: usize = 250_000;
const PURGE_THRESHOLD_DAYS: i64 = 1;

#[derive(Debug, Clone)]
pub struct LargeResponseHandler {
    temp_dir: PathBuf,
}

impl LargeResponseHandler {
    pub fn new() -> Self {
        use rand::Rng;
        let session_id: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();

        let temp_dir = std::env::temp_dir()
            .join("swarm_tool_responses")
            .join(session_id);

        let _ = std::fs::create_dir_all(&temp_dir);
        let handler = Self { temp_dir };
        handler.purge_old_responses().ok();
        handler
    }

    /// Process a tool output string. If it's too large, write to a file and return a redirection message.
    pub fn handle_output(&self, output: String) -> String {
        let bytes_len = output.len();
        if bytes_len > LARGE_TEXT_THRESHOLD_BYTES {
            match self.write_to_file(&output) {
                Ok(file_path) => {
                    format!(
                        "🛡️ Large Response Redirected: The output was too large ({} bytes) for the context window. \
                        It has been saved to a temporary file. You can read it using file tools if needed: {}",
                        bytes_len,
                        file_path.display()
                    )
                }
                Err(e) => {
                    format!(
                        "⚠️ Warning: Large response detected ({} bytes), but failed to save to file: {}. \
                        Showing full response (Risk of context overflow):\n\n{}",
                        bytes_len,
                        e,
                        output
                    )
                }
            }
        } else {
            output
        }
    }

    fn write_to_file(&self, content: &str) -> std::io::Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.6f");
        let filename = format!("tool_resp_{}.txt", timestamp);
        let file_path = self.temp_dir.join(&filename);

        let mut file = File::create(&file_path)?;
        file.write_all(content.as_bytes())?;

        Ok(file_path)
    }

    /// Deletes tool response files older than 24 hours to prevent disk leak.
    pub fn purge_old_responses(&self) -> std::io::Result<()> {
        if !self.temp_dir.exists() {
            return Ok(());
        }

        let now = Utc::now();
        for entry in std::fs::read_dir(&self.temp_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    let modified: chrono::DateTime<Utc> = modified.into();
                    if now.signed_duration_since(modified).num_days() >= PURGE_THRESHOLD_DAYS {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for LargeResponseHandler {
    fn default() -> Self {
        Self::new()
    }
}
