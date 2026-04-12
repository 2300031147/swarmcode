mod client;
mod error;
mod providers;
mod sse;
mod types;

pub use client::{
    oauth_token_is_expired, read_base_url, read_clawswarm_xai_url, resolve_saved_oauth_token,
    resolve_startup_auth_source, MessageStream, OAuthTokenSet, ProviderClient,
};
pub use error::ApiError;
pub use providers::claw_provider::{AuthSource, ClawApiClient, ClawApiClient as ApiClient};
pub use providers::openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};
pub use providers::{
    detect_provider_kind, max_tokens_for_model, metadata_for_model, provider_status_report,
    resolve_model_alias, ProviderKind, ProviderMetadata,
};
pub use providers::custom::{CustomProvider, load_custom_providers, providers_config_path, ensure_providers_template};
pub use providers::hosted::{detect_available_hosted, HostedProviderStatus};
pub use providers::local::{ollama_base_url, ollama_likely_running, lmstudio_base_url, lmstudio_likely_running};
pub use sse::{parse_frame, SseParser};
pub use types::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageDelta, MessageDeltaEvent, MessageRequest,
    MessageResponse, MessageStartEvent, MessageStopEvent, OutputContentBlock, StreamEvent,
    ToolChoice, ToolDefinition, ToolResultContentBlock, Usage,
};
