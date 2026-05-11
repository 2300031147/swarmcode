use serde::{Deserialize, Serialize};
use swarm_hands::WebAgent;

#[derive(Debug, Serialize, Deserialize)]
pub struct HandsTaskRequest {
    pub url: String,
    pub task: String,
    pub show_browser: bool,
}

#[derive(Debug, Serialize)]
pub struct HandsTaskResult {
    pub url: String,
    pub result: String,
    pub dom_snapshot: String,
}

#[tauri::command]
pub async fn hands_run_agent(req: HandsTaskRequest) -> Result<HandsTaskResult, String> {
    println!("🤖 SwarmHands: task='{}' url='{}'", req.task, req.url);

    let mut agent = WebAgent::new();
    agent
        .execute_task(&req.url, &req.task, req.show_browser)
        .await
        .map_err(|e| format!("Browser Agent Error: {e}"))?;

    Ok(HandsTaskResult {
        url: agent.current_url,
        result: agent.last_result,
        dom_snapshot: agent.dom_snapshot,
    })
}
