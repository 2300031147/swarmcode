//! Local model provider configurations.
//!
//! Supports Ollama and LM Studio — both expose an OpenAI-compatible
//! endpoint, so no API key is required. Just run the local server and
//! ClawSwarm will auto-detect and route to it.
//!
//! Env overrides:
//!   CLAWSWARM_OLLAMA_URL    — override Ollama base URL (default: http://localhost:11434/v1)
//!   CLAWSWARM_LMSTUDIO_URL  — override LM Studio base URL (default: http://localhost:1234/v1)

use crate::providers::openai_compat::OpenAiCompatConfig;

pub const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434/v1";
pub const LMSTUDIO_DEFAULT_URL: &str = "http://localhost:1234/v1";

/// Config for local Ollama server.
/// No API key required — uses "ollama" as a dummy bearer token
/// which Ollama ignores.
#[must_use]
pub fn ollama_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Ollama",
        api_key_env: "CLAWSWARM_OLLAMA_KEY", // optional, Ollama ignores it
        base_url_env: "CLAWSWARM_OLLAMA_URL",
        default_base_url: OLLAMA_DEFAULT_URL,
        auth_env_vars: &["CLAWSWARM_OLLAMA_KEY"],
    }
}

/// Config for LM Studio local server.
/// No API key required — LM Studio accepts any bearer token.
#[must_use]
pub fn lmstudio_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "LM Studio",
        api_key_env: "CLAWSWARM_LMSTUDIO_KEY", // optional, LM Studio ignores it
        base_url_env: "CLAWSWARM_LMSTUDIO_URL",
        default_base_url: LMSTUDIO_DEFAULT_URL,
        auth_env_vars: &["CLAWSWARM_LMSTUDIO_KEY"],
    }
}

/// Returns the resolved Ollama base URL (env override or default).
#[must_use]
pub fn ollama_base_url() -> String {
    std::env::var("CLAWSWARM_OLLAMA_URL")
        .unwrap_or_else(|_| OLLAMA_DEFAULT_URL.to_string())
}

/// Returns the resolved LM Studio base URL (env override or default).
#[must_use]
pub fn lmstudio_base_url() -> String {
    std::env::var("CLAWSWARM_LMSTUDIO_URL")
        .unwrap_or_else(|_| LMSTUDIO_DEFAULT_URL.to_string())
}

/// Checks if Ollama appears to be running by attempting a fast TCP connect.
#[must_use]
pub fn ollama_likely_running() -> bool {
    let url = ollama_base_url();
    // Parse host:port from the URL
    if let Some(host_port) = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/').next()
    {
        std::net::TcpStream::connect_timeout(
            &host_port.parse().unwrap_or_else(|_| "127.0.0.1:11434".parse().unwrap()),
            std::time::Duration::from_millis(300),
        ).is_ok()
    } else {
        false
    }
}

/// Checks if LM Studio appears to be running by attempting a fast TCP connect.
#[must_use]
pub fn lmstudio_likely_running() -> bool {
    let url = lmstudio_base_url();
    if let Some(host_port) = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/').next()
    {
        std::net::TcpStream::connect_timeout(
            &host_port.parse().unwrap_or_else(|_| "127.0.0.1:1234".parse().unwrap()),
            std::time::Duration::from_millis(300),
        ).is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_config_has_correct_default_url() {
        assert_eq!(ollama_config().default_base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn lmstudio_config_has_correct_default_url() {
        assert_eq!(lmstudio_config().default_base_url, "http://localhost:1234/v1");
    }
}
