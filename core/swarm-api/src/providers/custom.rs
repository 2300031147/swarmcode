//! Custom provider loader — reads `~/.clawswarm/providers.toml`.
//!
//! Allows registering any OpenAI-compatible endpoint with a name,
//! base URL, optional API key, and list of models it serves.
//!
//! Example `~/.clawswarm/providers.toml`:
//!
//! ```toml
//! [[provider]]
//! name     = "my-remote-ollama"
//! base_url = "http://192.168.1.10:11434/v1"
//! api_key  = "none"
//! models   = ["llama3.2", "mistral-7b"]
//!
//! [[provider]]
//! name     = "groq-custom"
//! base_url = "https://api.groq.com/openai/v1"
//! api_key_env = "GROQ_API_KEY"   # read from env instead of hardcoding
//! models   = ["llama-3.3-70b-versatile"]
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

/// A single provider entry from `providers.toml`.
#[derive(Debug, Clone)]
pub struct CustomProvider {
    /// Display name (e.g. "my-remote-ollama")
    pub name: String,
    /// Base URL of the OpenAI-compatible endpoint
    pub base_url: String,
    /// Resolved API key (from inline `api_key` or `api_key_env`)
    pub api_key: String,
    /// List of model names this provider serves
    pub models: Vec<String>,
}

impl CustomProvider {
    /// Returns true if this provider handles the given model name.
    #[must_use]
    pub fn handles_model(&self, model: &str) -> bool {
        let lower = model.to_ascii_lowercase();
        self.models.iter().any(|m| m.to_ascii_lowercase() == lower)
    }
}

/// Loads all custom providers from `~/.clawswarm/providers.toml`.
/// Returns an empty vec if the file doesn't exist or cannot be parsed.
#[must_use]
pub fn load_custom_providers() -> Vec<CustomProvider> {
    let path = providers_config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    parse_providers_toml(&content)
}

/// Returns the path to `~/.clawswarm/providers.toml`.
#[must_use]
pub fn providers_config_path() -> PathBuf {
    dirs_path().join("providers.toml")
}

fn dirs_path() -> PathBuf {
    // Check env override first (useful for testing / CI)
    if let Ok(dir) = std::env::var("CLAWSWARM_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    // Default: ~/.clawswarm/
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".clawswarm")
}

/// Minimal TOML parser for `[[provider]]` arrays.
/// We implement this without pulling in `toml` crate to stay lightweight.
fn parse_providers_toml(content: &str) -> Vec<CustomProvider> {
    let mut providers = Vec::new();
    let mut current: Option<HashMap<String, String>> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line == "[[provider]]" {
            // Save previous block
            if let Some(block) = current.take() {
                if let Some(p) = build_provider(block) {
                    providers.push(p);
                }
            }
            current = Some(HashMap::new());
            continue;
        }

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some(ref mut block) = current {
            if let Some((key, val)) = line.split_once('=') {
                let k = key.trim().to_string();
                let v = val.trim().trim_matches('"').to_string();
                block.insert(k, v);
            }
        }
    }

    // Save last block
    if let Some(block) = current {
        if let Some(p) = build_provider(block) {
            providers.push(p);
        }
    }

    providers
}

fn build_provider(mut map: HashMap<String, String>) -> Option<CustomProvider> {
    let name = map.remove("name")?;
    let base_url = map.remove("base_url")?;

    // Resolve API key: check inline `api_key` first, then `api_key_env`
    let api_key = if let Some(key) = map.remove("api_key") {
        if key == "none" || key.is_empty() {
            "clawswarm-local".to_string() // dummy bearer — ignored by local servers
        } else {
            key
        }
    } else if let Some(env_name) = map.remove("api_key_env") {
        std::env::var(&env_name).unwrap_or_default()
    } else {
        String::new()
    };

    // Parse models = ["a", "b", "c"]
    let models = map
        .remove("models")
        .map(|s| {
            s.trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|m| m.trim().trim_matches('"').to_string())
                .filter(|m| !m.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Some(CustomProvider { name, base_url, api_key, models })
}

/// Creates a default `providers.toml` template at `~/.clawswarm/providers.toml`
/// if it doesn't already exist.
pub fn ensure_providers_template() {
    let path = providers_config_path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let template = r#"# ClawSwarm Provider Configuration
# Add any OpenAI-compatible LLM endpoint here.
# Documentation: https://github.com/clawswarm/clawswarm#providers
#
# [[provider]]
# name        = "my-ollama"
# base_url    = "http://localhost:11434/v1"
# api_key     = "none"
# models      = ["llama3.2", "mistral-7b", "phi4"]
#
# [[provider]]
# name        = "groq-personal"
# base_url    = "https://api.groq.com/openai/v1"
# api_key_env = "GROQ_API_KEY"
# models      = ["llama-3.3-70b-versatile", "mixtral-8x7b-32768"]
#
# [[provider]]
# name        = "remote-server"
# base_url    = "http://192.168.1.10:11434/v1"
# api_key     = "none"
# models      = ["llama3.2"]
"#;
    let _ = std::fs::write(path, template);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[provider]]
name = "local-ollama"
base_url = "http://localhost:11434/v1"
api_key = "none"
models = ["llama3.2", "mistral-7b"]

[[provider]]
name = "groq-cloud"
base_url = "https://api.groq.com/openai/v1"
api_key_env = "GROQ_API_KEY"
models = ["llama-3.3-70b-versatile"]
"#;

    #[test]
    fn parses_two_providers() {
        let providers = parse_providers_toml(SAMPLE);
        assert_eq!(providers.len(), 2);
    }

    #[test]
    fn first_provider_has_correct_fields() {
        let providers = parse_providers_toml(SAMPLE);
        let p = &providers[0];
        assert_eq!(p.name, "local-ollama");
        assert_eq!(p.base_url, "http://localhost:11434/v1");
        assert_eq!(p.api_key, "clawswarm-local"); // "none" -> dummy key
        assert!(p.handles_model("llama3.2"));
        assert!(p.handles_model("mistral-7b"));
    }

    #[test]
    fn handles_model_is_case_insensitive() {
        let providers = parse_providers_toml(SAMPLE);
        assert!(providers[0].handles_model("Llama3.2"));
        assert!(providers[0].handles_model("MISTRAL-7B"));
    }
}
