//! Miscellaneous app-level commands.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub tauri_version: String,
}

/// Return version and build info.
#[tauri::command]
pub fn app_get_version() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "ClawSwarm".to_string(),
        tauri_version: tauri::VERSION.to_string(),
    }
}

/// Open the settings TOML file in the system default editor.
#[tauri::command]
pub fn app_open_settings() -> Result<String, String> {
    swarm_api::ensure_providers_template();
    let path = swarm_api::providers_config_path();
    Ok(path.to_string_lossy().to_string())
}
