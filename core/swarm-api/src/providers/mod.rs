//! Provider registry and routing logic for ClawSwarm.
//!
//! Detection priority (highest → lowest):
//!   1. `~/.clawswarm/providers.toml` model match
//!   2. Built-in model name registry
//!   3. Env var presence (GROQ_API_KEY, MISTRAL_API_KEY, etc.)
//!   4. Local server probe (Ollama on :11434, LM Studio on :1234)
//!   5. Default ClawApi

use std::future::Future;
use std::pin::Pin;

use crate::error::ApiError;
use crate::types::{MessageRequest, MessageResponse};

pub mod claw_provider;
pub mod custom;
pub mod hosted;
pub mod local;
pub mod openai_compat;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

pub trait Provider {
    type Stream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse>;

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream>;
}

// ── Provider Kinds ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// ClawSwarm native API
    ClawApi,
    /// xAI-compatible endpoint
    ClawSwarmXai,
    /// OpenAI (GPT-4o, o1, o3)
    OpenAi,
    /// Local Ollama server (localhost:11434)
    Ollama,
    /// Local LM Studio (localhost:1234)
    LmStudio,
    /// Groq Cloud (api.groq.com)
    Groq,
    /// Mistral AI (api.mistral.ai)
    Mistral,
    /// Together AI (api.together.xyz)
    Together,
    /// Custom provider from providers.toml
    Custom,
}

impl ProviderKind {
    /// Human-readable display name
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::ClawApi      => "ClawSwarm API",
            Self::ClawSwarmXai => "ClawSwarm xAI",
            Self::OpenAi       => "OpenAI",
            Self::Ollama       => "Ollama (local)",
            Self::LmStudio     => "LM Studio (local)",
            Self::Groq         => "Groq",
            Self::Mistral      => "Mistral",
            Self::Together     => "Together AI",
            Self::Custom       => "Custom",
        }
    }

    /// Whether this provider requires an API key
    #[must_use]
    pub const fn requires_api_key(self) -> bool {
        !matches!(self, Self::Ollama | Self::LmStudio)
    }
}

// ── Provider Metadata ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider: ProviderKind,
    pub auth_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

// ── Built-in Model Registry ───────────────────────────────────────────────

const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    // ClawSwarm API tier models
    (
        "swarm-opus",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "swarm-standard",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "swarm-lite",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "swarm-model-opus",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "swarm-model-sonnet",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    (
        "swarm-model-haiku",
        ProviderMetadata {
            provider: ProviderKind::ClawApi,
            auth_env: "CLAWSWARM_API_KEY",
            base_url_env: "CLAWSWARM_BASE_URL",
            default_base_url: claw_provider::DEFAULT_BASE_URL,
        },
    ),
    // xAI-compatible tier
    (
        "swarm-model-frontier",
        ProviderMetadata {
            provider: ProviderKind::ClawSwarmXai,
            auth_env: "CLAWSWARM_XAI_KEY",
            base_url_env: "CLAWSWARM_XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_CLAWSWARM_XAI_BASE_URL,
        },
    ),
    (
        "swarm-model-mini",
        ProviderMetadata {
            provider: ProviderKind::ClawSwarmXai,
            auth_env: "CLAWSWARM_XAI_KEY",
            base_url_env: "CLAWSWARM_XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_CLAWSWARM_XAI_BASE_URL,
        },
    ),
    (
        "swarm-model-legacy",
        ProviderMetadata {
            provider: ProviderKind::ClawSwarmXai,
            auth_env: "CLAWSWARM_XAI_KEY",
            base_url_env: "CLAWSWARM_XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_CLAWSWARM_XAI_BASE_URL,
        },
    ),
    // Groq-prefixed shorthand
    (
        "groq/llama-3.3-70b",
        ProviderMetadata {
            provider: ProviderKind::Groq,
            auth_env: "GROQ_API_KEY",
            base_url_env: "CLAWSWARM_GROQ_BASE_URL",
            default_base_url: hosted::GROQ_DEFAULT_URL,
        },
    ),
    (
        "groq/mixtral-8x7b",
        ProviderMetadata {
            provider: ProviderKind::Groq,
            auth_env: "GROQ_API_KEY",
            base_url_env: "CLAWSWARM_GROQ_BASE_URL",
            default_base_url: hosted::GROQ_DEFAULT_URL,
        },
    ),
    // Mistral shorthand
    (
        "mistral/large",
        ProviderMetadata {
            provider: ProviderKind::Mistral,
            auth_env: "MISTRAL_API_KEY",
            base_url_env: "CLAWSWARM_MISTRAL_BASE_URL",
            default_base_url: hosted::MISTRAL_DEFAULT_URL,
        },
    ),
    (
        "mistral/codestral",
        ProviderMetadata {
            provider: ProviderKind::Mistral,
            auth_env: "MISTRAL_API_KEY",
            base_url_env: "CLAWSWARM_MISTRAL_BASE_URL",
            default_base_url: hosted::MISTRAL_DEFAULT_URL,
        },
    ),
    // OpenAI shorthand
    (
        "gpt-4o",
        ProviderMetadata {
            provider: ProviderKind::OpenAi,
            auth_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: hosted::OPENAI_DEFAULT_URL,
        },
    ),
    (
        "gpt-4o-mini",
        ProviderMetadata {
            provider: ProviderKind::OpenAi,
            auth_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: hosted::OPENAI_DEFAULT_URL,
        },
    ),
    // Ollama common model shorthand
    (
        "llama3.2",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "llama3.1",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "phi4",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "mistral",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "deepseek-r1",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "gemma3",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
    (
        "qwen2.5-coder",
        ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        },
    ),
];

// ── Resolution Functions ──────────────────────────────────────────────────

#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY
        .iter()
        .find_map(|(alias, metadata)| {
            (*alias == lower).then_some(match metadata.provider {
                ProviderKind::ClawApi => match *alias {
                    "swarm-opus"    => "swarm-model-opus",
                    "swarm-standard"=> "swarm-model-sonnet",
                    "swarm-lite"    => "swarm-model-haiku",
                    _ => trimmed,
                },
                ProviderKind::ClawSwarmXai => match *alias {
                    "swarm-model-frontier" => "swarm-model-frontier",
                    "swarm-model-mini"     => "swarm-model-mini",
                    "swarm-model-legacy"   => "swarm-model-legacy",
                    _ => trimmed,
                },
                _ => trimmed,
            })
        })
        .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}

#[must_use]
pub fn metadata_for_model(model: &str) -> Option<ProviderMetadata> {
    let lower = model.trim().to_ascii_lowercase();

    // 1. Check custom providers.toml first
    let custom = custom::load_custom_providers();
    if let Some(cp) = custom.iter().find(|p| p.handles_model(&lower)) {
        // Return a pseudo-metadata pointing at the custom provider
        let _ = cp; // url/key handled in client.rs via Custom variant
        return Some(ProviderMetadata {
            provider: ProviderKind::Custom,
            auth_env: "CLAWSWARM_CUSTOM_KEY",
            base_url_env: "CLAWSWARM_CUSTOM_URL",
            default_base_url: "",
        });
    }

    // 2. Built-in registry
    let canonical = resolve_model_alias(&lower);
    if let Some((_, metadata)) = MODEL_REGISTRY.iter().find(|(alias, _)| *alias == canonical) {
        return Some(*metadata);
    }

    // 3. Prefix-based routing (e.g. "ollama/..." or "groq/...")
    if lower.starts_with("ollama/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Ollama,
            auth_env: "CLAWSWARM_OLLAMA_KEY",
            base_url_env: "CLAWSWARM_OLLAMA_URL",
            default_base_url: local::OLLAMA_DEFAULT_URL,
        });
    }
    if lower.starts_with("lmstudio/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::LmStudio,
            auth_env: "CLAWSWARM_LMSTUDIO_KEY",
            base_url_env: "CLAWSWARM_LMSTUDIO_URL",
            default_base_url: local::LMSTUDIO_DEFAULT_URL,
        });
    }
    if lower.starts_with("groq/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Groq,
            auth_env: "GROQ_API_KEY",
            base_url_env: "CLAWSWARM_GROQ_BASE_URL",
            default_base_url: hosted::GROQ_DEFAULT_URL,
        });
    }
    if lower.starts_with("mistral/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Mistral,
            auth_env: "MISTRAL_API_KEY",
            base_url_env: "CLAWSWARM_MISTRAL_BASE_URL",
            default_base_url: hosted::MISTRAL_DEFAULT_URL,
        });
    }
    if lower.starts_with("together/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Together,
            auth_env: "TOGETHER_API_KEY",
            base_url_env: "CLAWSWARM_TOGETHER_BASE_URL",
            default_base_url: hosted::TOGETHER_DEFAULT_URL,
        });
    }
    if lower.starts_with("swarm-model-frontier") {
        return Some(ProviderMetadata {
            provider: ProviderKind::ClawSwarmXai,
            auth_env: "CLAWSWARM_XAI_KEY",
            base_url_env: "CLAWSWARM_XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_CLAWSWARM_XAI_BASE_URL,
        });
    }

    None
}

/// Detects which provider to use, falling back through the priority chain.
#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    // 1. Model-level routing
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }

    // 2. Env var presence
    if openai_compat::has_api_key("GROQ_API_KEY") {
        return ProviderKind::Groq;
    }
    if openai_compat::has_api_key("MISTRAL_API_KEY") {
        return ProviderKind::Mistral;
    }
    if openai_compat::has_api_key("TOGETHER_API_KEY") {
        return ProviderKind::Together;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    if openai_compat::has_api_key("CLAWSWARM_XAI_KEY") {
        return ProviderKind::ClawSwarmXai;
    }

    // 3. Local server probing (fast TCP connect)
    if std::env::var("CLAWSWARM_OLLAMA_URL").is_ok() || local::ollama_likely_running() {
        return ProviderKind::Ollama;
    }
    if std::env::var("CLAWSWARM_LMSTUDIO_URL").is_ok() || local::lmstudio_likely_running() {
        return ProviderKind::LmStudio;
    }

    // 4. ClawSwarm API (requires CLAWSWARM_API_KEY or saved OAuth)
    if claw_provider::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::ClawApi;
    }

    // 5. Default
    ProviderKind::ClawApi
}

#[must_use]
pub fn max_tokens_for_model(model: &str) -> u32 {
    let canonical = resolve_model_alias(model);
    if canonical.contains("swarm-opus") || canonical.contains("llama3") {
        32_000
    } else {
        64_000
    }
}

/// Returns a human-readable status summary of all providers for the startup banner.
#[must_use]
pub fn provider_status_report() -> Vec<(String, bool, String)> {
    let mut report = Vec::new();

    // Local
    let ollama_running = local::ollama_likely_running()
        || std::env::var("CLAWSWARM_OLLAMA_URL").is_ok();
    report.push((
        "Ollama (local)".to_string(),
        ollama_running,
        local::ollama_base_url(),
    ));

    let lmstudio_running = local::lmstudio_likely_running()
        || std::env::var("CLAWSWARM_LMSTUDIO_URL").is_ok();
    report.push((
        "LM Studio (local)".to_string(),
        lmstudio_running,
        local::lmstudio_base_url(),
    ));

    // Hosted
    for hs in hosted::detect_available_hosted() {
        report.push((hs.name.to_string(), hs.key_set, hs.base_url));
    }

    // Custom
    for cp in custom::load_custom_providers() {
        report.push((format!("Custom: {}", cp.name), true, cp.base_url));
    }

    report
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_swarm_aliases() {
        assert_eq!(resolve_model_alias("swarm-model-frontier"), "swarm-model-frontier");
        assert_eq!(resolve_model_alias("swarm-model-mini"), "swarm-model-mini");
        assert_eq!(resolve_model_alias("swarm-opus"), "swarm-model-opus");
        assert_eq!(resolve_model_alias("swarm-standard"), "swarm-model-sonnet");
        assert_eq!(resolve_model_alias("swarm-lite"), "swarm-model-haiku");
    }

    #[test]
    fn ollama_prefix_routes_to_ollama() {
        let meta = metadata_for_model("ollama/llama3.2").unwrap();
        assert_eq!(meta.provider, ProviderKind::Ollama);
    }

    #[test]
    fn groq_prefix_routes_to_groq() {
        let meta = metadata_for_model("groq/mixtral-8x7b").unwrap();
        assert_eq!(meta.provider, ProviderKind::Groq);
    }

    #[test]
    fn mistral_prefix_routes_to_mistral() {
        let meta = metadata_for_model("mistral/large").unwrap();
        assert_eq!(meta.provider, ProviderKind::Mistral);
    }

    #[test]
    fn known_ollama_models_resolve_correctly() {
        assert_eq!(metadata_for_model("llama3.2").unwrap().provider, ProviderKind::Ollama);
        assert_eq!(metadata_for_model("phi4").unwrap().provider, ProviderKind::Ollama);
        assert_eq!(metadata_for_model("deepseek-r1").unwrap().provider, ProviderKind::Ollama);
        assert_eq!(metadata_for_model("gemma3").unwrap().provider, ProviderKind::Ollama);
    }

    #[test]
    fn openai_models_resolve_correctly() {
        assert_eq!(metadata_for_model("gpt-4o").unwrap().provider, ProviderKind::OpenAi);
        assert_eq!(metadata_for_model("gpt-4o-mini").unwrap().provider, ProviderKind::OpenAi);
    }

    #[test]
    fn max_tokens_heuristic() {
        assert_eq!(max_tokens_for_model("swarm-opus"), 32_000);
        assert_eq!(max_tokens_for_model("swarm-model-frontier"), 64_000);
    }
}
