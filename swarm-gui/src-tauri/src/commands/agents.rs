//! Agent commands — bridges the SwarmHive to the frontend, providing
//! live hive member status, message dispatch, and swarm spawning.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;
use swarm_runtime::{SwarmHive, HiveMember, HiveRole, HiveMemberStatus, team_message};

// ── State ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AgentState(pub Mutex<Option<Arc<SwarmHive>>>);

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub role: String,
    pub status: String,
    pub last_seen_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HiveStatus {
    pub agents: Vec<AgentEntry>,
    pub total: usize,
    pub active: usize,
    pub idle: usize,
}

// ── Commands ──────────────────────────────────────────────────────────────

/// Ensure the hive is initialized and return current member list.
#[tauri::command]
pub fn agents_list(state: State<AgentState>) -> Result<HiveStatus, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Arc::new(
            SwarmHive::new().with_persistence("clawswarm-gui".to_string()),
        ));
    }
    let hive = guard.as_ref().unwrap().clone();

    // Register the GUI itself as a member if not yet registered
    hive.register(HiveMember {
        id: "gui-orchestrator".to_string(),
        name: "GUI Orchestrator".to_string(),
        description: "ClawSwarm desktop GUI agent".to_string(),
        role: HiveRole::Lead,
        status: HiveMemberStatus::Active,
        last_seen_epoch_ms: now_ms(),
    });

    build_hive_status(&hive)
}

/// Start the engineering swarm (Security, Performance, Docs agents).
#[tauri::command]
pub fn agents_start_swarm(state: State<AgentState>) -> Result<HiveStatus, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Arc::new(
            SwarmHive::new().with_persistence("clawswarm-gui".to_string()),
        ));
    }
    let hive = guard.as_ref().unwrap().clone();

    // Register specialized agents
    for (id, name, desc, role) in [
        ("security-auditor",  "Security Auditor",     "Audits for vulnerabilities",        HiveRole::Teammate),
        ("performance-tuner", "Performance Tuner",    "Optimizes CPU/Memory paths",         HiveRole::Teammate),
        ("doc-lead",          "Documentation Lead",   "Ensures API coverage & consistency", HiveRole::Teammate),
        ("code-reviewer",     "Code Reviewer",        "Reviews PRs & enforces standards",   HiveRole::Teammate),
        ("test-engineer",     "Test Engineer",        "Writes unit & integration tests",    HiveRole::Teammate),
    ] {
        hive.register(HiveMember {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            role,
            status: HiveMemberStatus::Idle,
            last_seen_epoch_ms: now_ms(),
        });
    }

    hive.broadcast(team_message(
        "gui-orchestrator",
        None,
        "🚀 Engineering Swarm initialized via GUI. All specialists online.",
    ));

    build_hive_status(&hive)
}

/// Send a message to a specific agent or broadcast to all.
#[tauri::command]
pub fn agents_send_message(
    to: Option<String>,
    message: String,
    state: State<AgentState>,
) -> Result<String, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let hive = guard
        .as_ref()
        .ok_or("Hive not initialized. Call agents_list first.")?;

    let msg = team_message(
        "gui-orchestrator",
        to.as_deref(),
        &message,
    );
    if to.is_some() {
        hive.send_to(msg).map_err(|e| e.to_string())?;
    } else {
        hive.broadcast(msg);
    }
    Ok("Message sent to hive.".to_string())
}

/// Get the hive status without registering new agents.
#[tauri::command]
pub fn agents_get_hive_status(state: State<AgentState>) -> Result<HiveStatus, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let hive = guard
        .as_ref()
        .ok_or("Hive not initialized.")?;
    build_hive_status(hive)
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn build_hive_status(hive: &SwarmHive) -> Result<HiveStatus, String> {
    let members = hive.members();
    let agents: Vec<AgentEntry> = members
        .iter()
        .map(|m| AgentEntry {
            id: m.id.clone(),
            name: m.name.clone(),
            description: m.description.clone(),
            role: format!("{:?}", m.role),
            status: format!("{:?}", m.status),
            last_seen_ms: m.last_seen_epoch_ms,
        })
        .collect();

    let active = agents.iter().filter(|a| a.status == "Active").count();
    let idle = agents.iter().filter(|a| a.status == "Idle").count();
    let total = agents.len();

    Ok(HiveStatus { agents, total, active, idle })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
