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
    
    // In a real implementation, we would use req.show_browser 
    // to configure the chromiumoxide BrowserConfig.
    // For now, we execute the standard task loop.
    
    let mut agent = WebAgent::new();
    agent.execute_task(&req.url, &req.task).await
        .map_err(|e| format!("Browser Agent Error: {}", e))?;
    
    Ok(format!("Task complete. URL: {}. Result captured in DOM snapshot.", agent.current_url))
}
