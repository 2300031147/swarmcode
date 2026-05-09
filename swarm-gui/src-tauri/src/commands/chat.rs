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
    
    let mut stream = client.stream_message(&request).await.map_err(|e| e.to_string())?;
    let mut events = Vec::new();
    
    while let Some(event) = stream.next_event().await.map_err(|e| e.to_string())? {
        match event {
            swarm_api::StreamEvent::ContentBlockDelta(delta) => {
                if let swarm_api::ContentBlockDelta::TextDelta { text } = delta.delta {
                    events.push(swarm_runtime::AssistantEvent::TextDelta(text));
                }
            }
            swarm_api::StreamEvent::MessageDelta(delta) => {
                events.push(swarm_runtime::AssistantEvent::Usage(swarm_runtime::TokenUsage {
                    input_tokens: delta.usage.input_tokens,
                    output_tokens: delta.usage.output_tokens,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }));
            }
            swarm_api::StreamEvent::MessageStop(_) => {
                events.push(swarm_runtime::AssistantEvent::MessageStop);
            }
            _ => {}
        }
    }

    let (convo_msg, usage_info) = swarm_runtime::build_assistant_message(events)
        .map_err(|e| e.to_string())?;
    
    // We serialize the blocks to JSON so the frontend can parse them
    let assistant_content = convo_msg.to_json().render();
    
    let tokens_in = usage_info.as_ref().map(|u| u.input_tokens).unwrap_or(0);
    let tokens_out = usage_info.as_ref().map(|u| u.output_tokens).unwrap_or(0);

    // Build the reply message
    let reply = ChatMessage {
        id: format!("asst-{now_ms}"),
        role: "assistant".to_string(),
        content: assistant_content,
        model: Some(model.clone()),
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

    // Calculate cost using the centralized usage service (Issue #10)
    use swarm_runtime::usage::{TokenUsage, pricing_for_model};
    
    let usage = TokenUsage {
        input_tokens: total_in as u64,
        output_tokens: total_out as u64,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    };
    
    let pricing = pricing_for_model(&model).unwrap_or_else(swarm_runtime::usage::ModelPricing::default_standard_tier);
    let cost_estimate = usage.estimate_cost_usd_with_pricing(pricing);
    let total_cost = cost_estimate.total_cost_usd();

    Ok(ChatResponse {
        message: reply,
        total_cost_usd: total_cost,
        total_input_tokens: total_in,
        total_output_tokens: total_out,
    })
}

// ... existing code ...

/// Compact history keeping only a summary (Issue #16).
/// We delegate to swarm-runtime's compaction logic.
#[tauri::command]
pub fn chat_compact_history(state: State<ChatState>) -> Result<String, String> {
    use swarm_runtime::compact_session;
    use swarm_runtime::session::{Session, ConversationMessage, MessageRole, ContentBlock};
    
    let mut st = state.0.lock().map_err(|e| e.to_string())?;
    let count = st.history.len();
    
    if count < 15 {
        return Ok(format!("History only has {count} messages. Compaction deferred until 15+."));
    }

    // Convert ChatMessage to Session for compaction
    let mut session = Session {
        version: 1,
        messages: st.history.iter().map(|m| ConversationMessage {
            role: match m.role.as_str() {
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => MessageRole::User,
            },
            blocks: vec![ContentBlock::Text { text: m.content.clone() }],
            usage: None, // Simplified for this stub
        }).collect(),
    };

    // Run compaction synchronously for now (placeholder for background compression)
    let result = compact_session(&mut session, None).map_err(|e| e.to_string())?;
    
    // Update local history with the result
    st.history = session.messages.iter().enumerate().map(|(i, m)| ChatMessage {
        id: format!("compacted-{i}"),
        role: match m.role {
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
        },
        content: m.blocks.iter().find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        }).unwrap_or_default(),
        model: None,
        tokens_in: None,
        tokens_out: None,
        timestamp_ms: 0, // Metadata lost in simple compaction
    }).collect();

    Ok(format!("Successfully compacted {} messages into {}. Summary: {}", count, st.history.len(), result.summary))
}

/// Return current session token usage and estimated cost (Issue #10).
#[tauri::command]
pub fn chat_get_cost(state: State<ChatState>) -> Result<CostReport, String> {
    use swarm_runtime::usage::{TokenUsage, pricing_for_model};
    
    let st = state.0.lock().map_err(|e| e.to_string())?;
    
    let usage = TokenUsage {
        input_tokens: st.total_input_tokens as u64,
        output_tokens: st.total_output_tokens as u64,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    };
    
    let pricing = pricing_for_model(&st.model).unwrap_or_else(swarm_runtime::usage::ModelPricing::default_standard_tier);
    let cost = usage.estimate_cost_usd_with_pricing(pricing).total_cost_usd();

    Ok(CostReport {
        total_input_tokens: st.total_input_tokens,
        total_output_tokens: st.total_output_tokens,
        estimated_cost_usd: cost,
        session_messages: st.history.len(),
    })
}
