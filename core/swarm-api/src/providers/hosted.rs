//! Hosted third-party LLM API provider configurations.
//!
//! All providers here implement the OpenAI Chat Completions API, so they
//! all route through `OpenAiCompatClient` with provider-specific base URLs
//! and API key env vars.
//!
//! Required env vars (per provider):
//!   GROQ_API_KEY       — Groq Cloud (https://console.groq.com)
//!   MISTRAL_API_KEY    — Mistral AI (https://console.mistral.ai)
//!   TOGETHER_API_KEY   — Together AI (https://api.together.xyz)
//!   OPENAI_API_KEY     — OpenAI (https://platform.openai.com)
//!
//! Optional base URL overrides (for proxies / custom endpoints):
//!   CLAWSWARM_GROQ_BASE_URL, CLAWSWARM_MISTRAL_BASE_URL,
//!   CLAWSWARM_TOGETHER_BASE_URL, OPENAI_BASE_URL

use crate::providers::openai_compat::OpenAiCompatConfig;

// ── Base URLs ──────────────────────────────────────────────────────────────

pub const GROQ_DEFAULT_URL: &str = "https://api.groq.com/openai/v1";
pub const MISTRAL_DEFAULT_URL: &str = "https://api.mistral.ai/v1";
pub const TOGETHER_DEFAULT_URL: &str = "https://api.together.xyz/v1";
pub const OPENAI_DEFAULT_URL: &str = "https://api.openai.com/v1";

// ── Provider configs ───────────────────────────────────────────────────────

/// Groq Cloud — ultra-fast inference via GroqChip hardware.
/// Models: llama-3.3-70b-versatile, mixtral-8x7b-32768, gemma2-9b-it, etc.
#[must_use]
pub fn groq_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Groq",
        api_key_env: "GROQ_API_KEY",
        base_url_env: "CLAWSWARM_GROQ_BASE_URL",
        default_base_url: GROQ_DEFAULT_URL,
        auth_env_vars: &["GROQ_API_KEY"],
    }
}

/// Mistral AI — European open-weight models.
/// Models: mistral-large-latest, mistral-medium, codestral-latest, etc.
#[must_use]
pub fn mistral_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Mistral",
        api_key_env: "MISTRAL_API_KEY",
        base_url_env: "CLAWSWARM_MISTRAL_BASE_URL",
        default_base_url: MISTRAL_DEFAULT_URL,
        auth_env_vars: &["MISTRAL_API_KEY"],
    }
}

/// Together AI — hundreds of open-source models on demand.
/// Models: meta-llama/Llama-3.3-70B-Instruct-Turbo, Qwen/Qwen2.5-Coder-32B-Instruct, etc.
#[must_use]
pub fn together_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Together",
        api_key_env: "TOGETHER_API_KEY",
        base_url_env: "CLAWSWARM_TOGETHER_BASE_URL",
        default_base_url: TOGETHER_DEFAULT_URL,
        auth_env_vars: &["TOGETHER_API_KEY"],
    }
}

/// OpenAI — GPT-4o, o1, o3 family.
/// Models: gpt-4o, gpt-4o-mini, o1, o3-mini, etc.
#[must_use]
pub fn openai_config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "OpenAI",
        api_key_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: OPENAI_DEFAULT_URL,
        auth_env_vars: &["OPENAI_API_KEY"],
    }
}

// ── Utility ────────────────────────────────────────────────────────────────

/// Returns list of all hosted providers with their detection status.
#[must_use]
pub fn detect_available_hosted() -> Vec<HostedProviderStatus> {
    vec![
        HostedProviderStatus {
            name: "Groq",
            key_set: std::env::var("GROQ_API_KEY").map(|k| !k.is_empty()).unwrap_or(false),
            base_url: std::env::var("CLAWSWARM_GROQ_BASE_URL")
                .unwrap_or_else(|_| GROQ_DEFAULT_URL.to_string()),
        },
        HostedProviderStatus {
            name: "Mistral",
            key_set: std::env::var("MISTRAL_API_KEY").map(|k| !k.is_empty()).unwrap_or(false),
            base_url: std::env::var("CLAWSWARM_MISTRAL_BASE_URL")
                .unwrap_or_else(|_| MISTRAL_DEFAULT_URL.to_string()),
        },
        HostedProviderStatus {
            name: "Together",
            key_set: std::env::var("TOGETHER_API_KEY").map(|k| !k.is_empty()).unwrap_or(false),
            base_url: std::env::var("CLAWSWARM_TOGETHER_BASE_URL")
                .unwrap_or_else(|_| TOGETHER_DEFAULT_URL.to_string()),
        },
        HostedProviderStatus {
            name: "OpenAI",
            key_set: std::env::var("OPENAI_API_KEY").map(|k| !k.is_empty()).unwrap_or(false),
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| OPENAI_DEFAULT_URL.to_string()),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct HostedProviderStatus {
    pub name: &'static str,
    pub key_set: bool,
    pub base_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groq_config_has_correct_defaults() {
        let cfg = groq_config();
        assert_eq!(cfg.provider_name, "Groq");
        assert_eq!(cfg.api_key_env, "GROQ_API_KEY");
        assert_eq!(cfg.default_base_url, GROQ_DEFAULT_URL);
    }

    #[test]
    fn mistral_config_has_correct_defaults() {
        let cfg = mistral_config();
        assert_eq!(cfg.provider_name, "Mistral");
        assert_eq!(cfg.default_base_url, MISTRAL_DEFAULT_URL);
    }

    #[test]
    fn together_config_has_correct_defaults() {
        let cfg = together_config();
        assert_eq!(cfg.provider_name, "Together");
        assert_eq!(cfg.default_base_url, TOGETHER_DEFAULT_URL);
    }

    #[test]
    fn detect_available_hosted_returns_four_providers() {
        let providers = detect_available_hosted();
        assert_eq!(providers.len(), 4);
        assert!(providers.iter().any(|p| p.name == "Groq"));
        assert!(providers.iter().any(|p| p.name == "OpenAI"));
    }
}
