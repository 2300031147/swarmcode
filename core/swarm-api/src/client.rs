use crate::error::ApiError;
use crate::providers::claw_provider::{self, AuthSource, ClawApiClient};
use crate::providers::openai_compat::{self, OpenAiCompatClient, OpenAiCompatConfig};
use crate::providers::{self, custom, hosted, local, Provider, ProviderKind};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

async fn send_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<MessageResponse, ApiError> {
    provider.send_message(request).await
}

async fn stream_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<P::Stream, ApiError> {
    provider.stream_message(request).await
}

// ── Client Variants ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ProviderClient {
    /// ClawSwarm native API client
    ClawApi(ClawApiClient),
    /// Any OpenAI-compatible endpoint (xAI, Groq, Mistral, Together, OpenAI,
    /// Ollama, LM Studio, or custom)
    Compat(OpenAiCompatClient),
}

impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_default_auth(model, None)
    }

    pub fn from_model_with_default_auth(
        model: &str,
        default_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved = providers::resolve_model_alias(model);
        let kind = providers::detect_provider_kind(&resolved);

        match kind {
            // ── Native ClawSwarm API ───────────────────────────────────
            ProviderKind::ClawApi => Ok(Self::ClawApi(match default_auth {
                Some(auth) => ClawApiClient::from_auth(auth),
                None => ClawApiClient::from_env()?,
            })),

            // ── xAI-compatible ────────────────────────────────────────
            ProviderKind::ClawSwarmXai => Ok(Self::Compat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::ClawSwarm())?,
            )),

            // ── OpenAI ────────────────────────────────────────────────
            ProviderKind::OpenAi => Ok(Self::Compat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::openai())?,
            )),

            // ── Groq ──────────────────────────────────────────────────
            ProviderKind::Groq => Ok(Self::Compat(
                OpenAiCompatClient::from_env(hosted::groq_config())?,
            )),

            // ── Mistral ───────────────────────────────────────────────
            ProviderKind::Mistral => Ok(Self::Compat(
                OpenAiCompatClient::from_env(hosted::mistral_config())?,
            )),

            // ── Together AI ───────────────────────────────────────────
            ProviderKind::Together => Ok(Self::Compat(
                OpenAiCompatClient::from_env(hosted::together_config())?,
            )),

            // ── Ollama (local, no auth) ───────────────────────────────
            ProviderKind::Ollama => Ok(Self::Compat(
                local::ollama_config().into_keyless_client(),
            )),

            // ── LM Studio (local, no auth) ────────────────────────────
            ProviderKind::LmStudio => Ok(Self::Compat(
                local::lmstudio_config().into_keyless_client(),
            )),

            // ── Custom (providers.toml) ───────────────────────────────
            ProviderKind::Custom => {
                let providers = custom::load_custom_providers();
                let cp = providers
                    .into_iter()
                    .find(|p| p.handles_model(&resolved))
                    .ok_or_else(|| ApiError::missing_credentials("Custom", &[]))?;

                Ok(Self::Compat(
                    OpenAiCompatClient::new(&cp.api_key, OpenAiCompatConfig {
                        provider_name: "Custom",
                        api_key_env: "CLAWSWARM_CUSTOM_KEY",
                        base_url_env: "CLAWSWARM_CUSTOM_URL",
                        default_base_url: "",
                        auth_env_vars: &[],
                    })
                    .with_base_url(cp.base_url),
                ))
            }
        }
    }

    #[must_use]
    pub fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::ClawApi(_) => ProviderKind::ClawApi,
            Self::Compat(_)  => {
                // The kind is already resolved during construction; we return
                // a sentinel — callers route by model name, not client variant.
                ProviderKind::Custom
            }
        }
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        match self {
            Self::ClawApi(c) => send_via_provider(c, request).await,
            Self::Compat(c)  => send_via_provider(c, request).await,
        }
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        match self {
            Self::ClawApi(c) => stream_via_provider(c, request)
                .await
                .map(MessageStream::ClawApi),
            Self::Compat(c) => stream_via_provider(c, request)
                .await
                .map(MessageStream::Compat),
        }
    }
}

// ── Message Stream ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum MessageStream {
    ClawApi(claw_provider::MessageStream),
    Compat(openai_compat::MessageStream),
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::ClawApi(s) => s.request_id(),
            Self::Compat(s)  => s.request_id(),
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::ClawApi(s) => s.next_event().await,
            Self::Compat(s)  => s.next_event().await,
        }
    }
}

// ── Re-exports ────────────────────────────────────────────────────────────

pub use claw_provider::{
    oauth_token_is_expired, resolve_saved_oauth_token, resolve_startup_auth_source, OAuthTokenSet,
};

#[must_use]
pub fn read_base_url() -> String {
    claw_provider::read_base_url()
}

#[must_use]
pub fn read_clawswarm_xai_url() -> String {
    openai_compat::read_base_url(OpenAiCompatConfig::ClawSwarm())
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::providers::{detect_provider_kind, resolve_model_alias, ProviderKind};

    #[test]
    fn resolves_swarm_model_aliases() {
        assert_eq!(resolve_model_alias("swarm-opus"), "swarm-model-opus");
        assert_eq!(resolve_model_alias("swarm-model-frontier"), "swarm-model-frontier");
        assert_eq!(resolve_model_alias("swarm-model-mini"), "swarm-model-mini");
    }

    #[test]
    fn provider_detection_prefers_model_family() {
        assert_eq!(detect_provider_kind("swarm-model-frontier"), ProviderKind::ClawSwarmXai);
        assert_eq!(detect_provider_kind("swarm-model-sonnet"), ProviderKind::ClawApi);
        assert_eq!(detect_provider_kind("llama3.2"), ProviderKind::Ollama);
        assert_eq!(detect_provider_kind("gpt-4o"), ProviderKind::OpenAi);
        assert_eq!(detect_provider_kind("groq/llama-3.3-70b"), ProviderKind::Groq);
    }

    #[test]
    fn ollama_model_names_route_to_ollama() {
        assert_eq!(detect_provider_kind("phi4"), ProviderKind::Ollama);
        assert_eq!(detect_provider_kind("deepseek-r1"), ProviderKind::Ollama);
        assert_eq!(detect_provider_kind("gemma3"), ProviderKind::Ollama);
        assert_eq!(detect_provider_kind("qwen2.5-coder"), ProviderKind::Ollama);
    }
}
