use serde::{Deserialize, Serialize};
use swarm_hands::WebAgent;
use tauri::State;

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct HandsTaskRequest {
    pub url: String,
    pub task: String,
    pub show_browser: bool,
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn hands_run_agent(req: HandsTaskRequest) -> Result<String, String> {
    println!("🤖 SwarmHands: Starting agent task on: {}", req.url);
    
    // We now properly pipe the frontend toggle down to Chromiumoxide
    let mut agent = WebAgent::new();
    agent.execute_task(&req.url, &req.task, req.show_browser).await
        .map_err(|e| format!("Browser Agent Error: {}", e))?;
    
    Ok(format!("Task complete. URL: {}. Result captured in DOM snapshot.", agent.current_url))
}
