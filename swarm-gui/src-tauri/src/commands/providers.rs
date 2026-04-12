//! Provider commands — expose swarm-api detection, testing, and config
//! management to the frontend via Tauri IPC.

use serde::{Deserialize, Serialize};
use swarm_api::{
    detect_provider_kind, detect_available_hosted, ensure_providers_template,
    load_custom_providers, lmstudio_base_url, lmstudio_likely_running,
    ollama_base_url, ollama_likely_running, providers_config_path, ProviderKind,
};

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderEntry {
    pub name: String,
    pub kind: String,
    pub available: bool,
    pub url: String,
    pub requires_key: bool,
    pub key_set: bool,
    pub models: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSwitchResult {
    pub model: String,
    pub provider: String,
    pub success: bool,
}

// ── Commands ──────────────────────────────────────────────────────────────

/// Returns the full list of detected providers with their status.
#[tauri::command]
pub fn providers_list() -> Vec<ProviderEntry> {
    let mut list = Vec::new();

    // Ollama
    let ollama_ok = ollama_likely_running()
        || std::env::var("CLAWSWARM_OLLAMA_URL").is_ok();
    list.push(ProviderEntry {
        name: "Ollama".to_string(),
        kind: "local".to_string(),
        available: ollama_ok,
        url: ollama_base_url(),
        requires_key: false,
        key_set: true,
        models: vec![
            "llama3.2".into(), "llama3.1".into(), "phi4".into(),
            "mistral".into(), "deepseek-r1".into(), "gemma3".into(),
            "qwen2.5-coder".into(),
        ],
    });

    // LM Studio
    let lmstudio_ok = lmstudio_likely_running()
        || std::env::var("CLAWSWARM_LMSTUDIO_URL").is_ok();
    list.push(ProviderEntry {
        name: "LM Studio".to_string(),
        kind: "local".to_string(),
        available: lmstudio_ok,
        url: lmstudio_base_url(),
        requires_key: false,
        key_set: true,
        models: vec!["(any GGUF model)".into()],
    });

    // Hosted providers
    for hs in detect_available_hosted() {
        list.push(ProviderEntry {
            name: hs.name.to_string(),
            kind: "hosted".to_string(),
            available: hs.key_set,
            url: hs.base_url,
            requires_key: true,
            key_set: hs.key_set,
            models: provider_default_models(hs.name),
        });
    }

    // Custom providers from providers.toml
    for cp in load_custom_providers() {
        list.push(ProviderEntry {
            name: cp.name.clone(),
            kind: "custom".to_string(),
            available: true,
            url: cp.base_url,
            requires_key: false,
            key_set: true,
            models: cp.models,
        });
    }

    list
}

/// Test connectivity to a named provider (ollama or lmstudio only via TCP).
#[tauri::command]
pub fn providers_test(name: String) -> serde_json::Value {
    match name.to_ascii_lowercase().as_str() {
        "ollama" => serde_json::json!({
            "name": "Ollama",
            "reachable": ollama_likely_running(),
            "url": ollama_base_url(),
        }),
        "lmstudio" | "lm studio" => serde_json::json!({
            "name": "LM Studio",
            "reachable": lmstudio_likely_running(),
            "url": lmstudio_base_url(),
        }),
        _ => serde_json::json!({
            "name": name,
            "reachable": null,
            "message": "Test connectivity by setting the API key and making a real request.",
        }),
    }
}

/// Detect which provider would handle a given model name.
#[tauri::command]
pub fn providers_set_model(model: String) -> ModelSwitchResult {
    let kind = detect_provider_kind(&model);
    ModelSwitchResult {
        model: model.clone(),
        provider: kind.display_name().to_string(),
        success: true,
    }
}

/// Get the path of providers.toml.
#[tauri::command]
pub fn providers_get_config_path() -> String {
    ensure_providers_template();
    providers_config_path()
        .to_string_lossy()
        .to_string()
}

/// Read the raw content of providers.toml.
#[tauri::command]
pub fn providers_read_config() -> Result<String, String> {
    let path = providers_config_path();
    ensure_providers_template();
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write new content to providers.toml.
#[tauri::command]
pub fn providers_write_config(content: String) -> Result<(), String> {
    let path = providers_config_path();
    std::fs::write(path, content).map_err(|e| e.to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn provider_default_models(name: &str) -> Vec<String> {
    match name {
        "Groq" => vec![
            "llama-3.3-70b-versatile".into(),
            "llama-3.1-8b-instant".into(),
            "mixtral-8x7b-32768".into(),
            "gemma2-9b-it".into(),
        ],
        "Mistral" => vec![
            "mistral-large-latest".into(),
            "mistral-medium".into(),
            "codestral-latest".into(),
        ],
        "Together" => vec![
            "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            "Qwen/Qwen2.5-Coder-32B-Instruct".into(),
            "mistralai/Mixtral-8x7B-Instruct".into(),
        ],
        "OpenAI" => vec![
            "gpt-4o".into(),
            "gpt-4o-mini".into(),
            "o1".into(),
            "o3-mini".into(),
        ],
        _ => vec![],
    }
}
