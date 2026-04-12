//! AI chat commands — bridges the Tauri frontend chat UI to the swarm-api
//! provider layer, maintaining a per-session conversation history.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

// ── State ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ChatState(pub Mutex<ChatSessionData>);

#[derive(Default)]
pub struct ChatSessionData {
    pub history: Vec<ChatMessage>,
    pub model: String,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,     // "user" | "assistant" | "system"
    pub content: String,
    pub model: Option<String>,
    pub tokens_in: Option<u32>,
    pub tokens_out: Option<u32>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub total_cost_usd: f64,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CostReport {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub session_messages: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub is_local: bool,
}

// ── Commands ──────────────────────────────────────────────────────────────

/// Send a message to the active model and return the assistant's reply.
/// This is a synchronous stub — streaming is handled via Tauri events in a
/// future enhancement. For now, it queues the message and returns a response.
#[tauri::command]
pub async fn chat_send_message(
    user_message: String,
    model: String,
    state: State<'_, ChatState>,
) -> Result<ChatResponse, String> {
    use swarm_api::{detect_provider_kind, ProviderKind, ProviderClient};
    use swarm_api::types::{MessageRequest, InputMessage, InputContentBlock};

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Build the request
    let history = {
        let st = state.0.lock().map_err(|e| e.to_string())?;
        st.history.clone()
    };

    let mut messages: Vec<InputMessage> = history
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| InputMessage {
            role: m.role.clone(),
            content: vec![InputContentBlock::Text { text: m.content.clone() }],
        })
        .collect();

    messages.push(InputMessage {
        role: "user".to_string(),
        content: vec![InputContentBlock::Text { text: user_message.clone() }],
    });

    let request = MessageRequest {
        model: model.clone(),
        max_tokens: 4096,
        messages,
        system: Some(
            "You are ClawSwarm, an expert AI coding assistant embedded in an IDE. \
             You help with code, architecture, debugging, and engineering tasks. \
             When referencing files, use code blocks with language tags.".to_string(),
        ),
        stream: false,
        tools: None,
        tool_choice: None,
        agent_id: Some("clawswarm-gui".to_string()),
    };

    // Build the client and send
    let client = ProviderClient::from_model(&model).map_err(|e| e.to_string())?;
    let response = client.send_message(&request).await.map_err(|e| e.to_string())?;

    let assistant_text = response
        .content
        .iter()
        .filter_map(|block| {
            if let swarm_api::OutputContentBlock::Text { text } = block {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let tokens_in = response.usage.input_tokens;
    let tokens_out = response.usage.output_tokens;

    // Build the reply message
    let reply = ChatMessage {
        id: response.id.clone(),
        role: "assistant".to_string(),
        content: assistant_text,
        model: Some(response.model.clone()),
        tokens_in: Some(tokens_in),
        tokens_out: Some(tokens_out),
        timestamp_ms: now_ms,
    };

    // Update history + token counters
    let (total_in, total_out, msg_count) = {
        let mut st = state.0.lock().map_err(|e| e.to_string())?;
        st.model = model.clone();
        st.history.push(ChatMessage {
            id: format!("user-{now_ms}"),
            role: "user".to_string(),
            content: user_message,
            model: None,
            tokens_in: None,
            tokens_out: None,
            timestamp_ms: now_ms,
        });
        st.history.push(reply.clone());
        st.total_input_tokens += tokens_in;
        st.total_output_tokens += tokens_out;
        (st.total_input_tokens, st.total_output_tokens, st.history.len())
    };

    // Cost estimate (rough: $0.50/M input, $1.50/M output for mid-tier)
    let cost = (total_in as f64 / 1_000_000.0 * 0.50)
        + (total_out as f64 / 1_000_000.0 * 1.50);

    Ok(ChatResponse {
        message: reply,
        total_cost_usd: cost,
        total_input_tokens: total_in,
        total_output_tokens: total_out,
    })
}

/// Get the list of all available models across all providers.
#[tauri::command]
pub fn chat_get_models() -> Vec<ModelEntry> {
    let mut models = Vec::new();

    // Built-in ClawSwarm API models
    for (id, display, provider) in [
        ("swarm-model-opus",    "SwarmMaster Pro",  "ClawSwarm API"),
        ("swarm-model-sonnet",  "SwarmMaster Std",  "ClawSwarm API"),
        ("swarm-model-haiku",   "SwarmMaster Lite", "ClawSwarm API"),
        ("swarm-model-frontier","SwarmEdge Frontier","ClawSwarm xAI"),
        ("swarm-model-mini",    "SwarmEdge Mini",   "ClawSwarm xAI"),
    ] {
        models.push(ModelEntry {
            id: id.to_string(),
            display_name: display.to_string(),
            provider: provider.to_string(),
            is_local: false,
        });
    }

    // Ollama local models
    if swarm_api::ollama_likely_running() || std::env::var("CLAWSWARM_OLLAMA_URL").is_ok() {
        for name in ["llama3.2", "llama3.1", "phi4", "mistral", "deepseek-r1", "gemma3", "qwen2.5-coder"] {
            models.push(ModelEntry {
                id: name.to_string(),
                display_name: format!("{name} (Ollama)"),
                provider: "Ollama".to_string(),
                is_local: true,
            });
        }
    }

    // Groq
    if std::env::var("GROQ_API_KEY").map(|k| !k.is_empty()).unwrap_or(false) {
        for (id, display) in [
            ("groq/llama-3.3-70b-versatile", "Llama 3.3 70B"),
            ("groq/mixtral-8x7b-32768",       "Mixtral 8x7B"),
            ("groq/gemma2-9b-it",             "Gemma2 9B"),
        ] {
            models.push(ModelEntry {
                id: id.to_string(),
                display_name: format!("{display} (Groq)"),
                provider: "Groq".to_string(),
                is_local: false,
            });
        }
    }

    // OpenAI
    if std::env::var("OPENAI_API_KEY").map(|k| !k.is_empty()).unwrap_or(false) {
        for (id, display) in [("gpt-4o", "GPT-4o"), ("gpt-4o-mini", "GPT-4o mini"), ("o1", "o1"), ("o3-mini", "o3-mini")] {
            models.push(ModelEntry {
                id: id.to_string(),
                display_name: format!("{display} (OpenAI)"),
                provider: "OpenAI".to_string(),
                is_local: false,
            });
        }
    }

    // Custom providers
    for cp in swarm_api::load_custom_providers() {
        for model_name in cp.models {
            models.push(ModelEntry {
                id: model_name.clone(),
                display_name: format!("{model_name} ({})", cp.name),
                provider: cp.name.clone(),
                is_local: true,
            });
        }
    }

    models
}

/// Clear the chat history for this session.
#[tauri::command]
pub fn chat_clear_history(state: State<ChatState>) -> Result<(), String> {
    let mut st = state.0.lock().map_err(|e| e.to_string())?;
    st.history.clear();
    st.total_input_tokens = 0;
    st.total_output_tokens = 0;
    Ok(())
}

/// Compact history keeping only a summary (placeholder — full compaction
/// uses swarm-runtime's compact_session in a production build).
#[tauri::command]
pub fn chat_compact_history(state: State<ChatState>) -> Result<String, String> {
    let mut st = state.0.lock().map_err(|e| e.to_string())?;
    let count = st.history.len();
    // Keep only last 10 messages
    if st.history.len() > 10 {
        st.history = st.history.split_off(st.history.len() - 10);
    }
    Ok(format!("Compacted {count} messages. Kept last 10 for context."))
}

/// Return current session token usage and estimated cost.
#[tauri::command]
pub fn chat_get_cost(state: State<ChatState>) -> Result<CostReport, String> {
    let st = state.0.lock().map_err(|e| e.to_string())?;
    let cost = (st.total_input_tokens as f64 / 1_000_000.0 * 0.50)
        + (st.total_output_tokens as f64 / 1_000_000.0 * 1.50);
    Ok(CostReport {
        total_input_tokens: st.total_input_tokens,
        total_output_tokens: st.total_output_tokens,
        estimated_cost_usd: cost,
        session_messages: st.history.len(),
    })
}
