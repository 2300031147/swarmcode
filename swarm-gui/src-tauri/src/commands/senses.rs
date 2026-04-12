use serde::{Deserialize, Serialize};
use swarm_senses::{CodeGraph, CodeNode, initialize_swarm_senses};
use std::sync::Mutex;
use tauri::State;

// ── State ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct SensesState(pub Mutex<Option<CodeGraph>>);

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolResult {
    pub name: String,
    pub file: String,
    pub kind: String,
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn senses_search(query: String, state: State<SensesState>) -> Result<Vec<SymbolResult>, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    
    // Initialize graph if not already done
    if guard.is_none() {
        *guard = Some(initialize_swarm_senses());
    }
    
    let graph = guard.as_ref().unwrap();
    let symbols = graph.find_symbols(&query);
    
    let results = symbols.into_iter().map(|node| SymbolResult {
        name: node.symbol_name,
        file: node.file_path.to_string_lossy().to_string(),
        kind: format!("{:?}", node.node_type),
    }).collect();
    
    Ok(results)
}
