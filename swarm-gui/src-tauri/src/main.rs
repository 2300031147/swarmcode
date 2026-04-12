// ClawSwarm GUI — Tauri v2 entry point
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("CLAWSWARM_LOG")
                .unwrap_or_else(|_| "swarm_gui=info,swarm_api=info".to_string()),
        )
        .init();

    info!("ClawSwarm GUI starting...");

    // Ensure providers template exists on first run
    swarm_api::ensure_providers_template();

    tauri::Builder::default()
        // ── Tauri plugins ────────────────────────────────────────────
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        // ── IPC command handlers ─────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            // File system
            commands::files::fs_list_dir,
            commands::files::fs_read_file,
            commands::files::fs_write_file,
            commands::files::fs_create_file,
            commands::files::fs_delete_file,
            commands::files::fs_rename,
            commands::files::fs_open_dialog,
            commands::files::fs_save_dialog,
            commands::files::fs_get_workspace,
            commands::files::fs_set_workspace,
            // AI Chat
            commands::chat::chat_send_message,
            commands::chat::chat_get_models,
            commands::chat::chat_clear_history,
            commands::chat::chat_compact_history,
            commands::chat::chat_get_cost,
            // Providers
            commands::providers::providers_list,
            commands::providers::providers_test,
            commands::providers::providers_set_model,
            commands::providers::providers_get_config_path,
            commands::providers::providers_read_config,
            commands::providers::providers_write_config,
            // Agents
            commands::agents::agents_list,
            commands::agents::agents_start_swarm,
            commands::agents::agents_send_message,
            commands::agents::agents_get_hive_status,
            // Terminal
            commands::terminal::terminal_run,
            commands::terminal::terminal_run_in_workspace,
            // Senses
            commands::senses::senses_search,
            // Hands
            commands::hands::hands_run_agent,
            // App
            commands::app::app_get_version,
            commands::app::app_open_settings,
        ])
        .setup(|app| {
            // Store workspace path in app state
            app.manage(commands::files::WorkspaceState::default());
            app.manage(commands::chat::ChatState::default());
            app.manage(commands::agents::AgentState::default());
            app.manage(commands::senses::SensesState::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ClawSwarm GUI failed to start");
}
