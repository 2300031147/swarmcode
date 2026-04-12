//! File system commands — exposed via Tauri IPC to the frontend.
//! Provides directory listing, file read/write, rename/delete, and
//! native open/save dialogs using tauri-plugin-dialog.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;
use walkdir::WalkDir;

// ── Shared workspace state ────────────────────────────────────────────────

#[derive(Default)]
pub struct WorkspaceState(pub Mutex<Option<PathBuf>>);

// ── Data types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub ext: Option<String>,
    pub size: Option<u64>,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize)]
pub struct ReadFileResult {
    pub path: String,
    pub content: String,
    pub language: String,
}

// ── Commands ──────────────────────────────────────────────────────────────

/// List the directory tree rooted at `path` (up to 3 levels deep).
#[tauri::command]
pub fn fs_list_dir(path: String) -> Result<Vec<FileNode>, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    Ok(list_dir_recursive(&root, 0))
}

fn list_dir_recursive(dir: &Path, depth: usize) -> Vec<FileNode> {
    if depth > 4 {
        return vec![];
    }
    let mut entries: Vec<FileNode> = std::fs::read_dir(dir)
        .map(|iter| {
            iter.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    let name = e.file_name().to_string_lossy().to_string();
                    // Skip hidden files and build artifacts
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        return None;
                    }
                    let path_str = e.path().to_string_lossy().to_string();
                    let ext = if meta.is_file() {
                        e.path()
                            .extension()
                            .map(|x| x.to_string_lossy().to_string())
                    } else {
                        None
                    };
                    let children = if meta.is_dir() {
                        Some(list_dir_recursive(&e.path(), depth + 1))
                    } else {
                        None
                    };
                    Some(FileNode {
                        name,
                        path: path_str,
                        is_dir: meta.is_dir(),
                        ext,
                        size: if meta.is_file() { Some(meta.len()) } else { None },
                        children,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Dirs first, then files, both sorted alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    entries
}

/// Read a file's content. Detects language from extension.
#[tauri::command]
pub fn fs_read_file(path: String) -> Result<ReadFileResult, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    let language = lang_from_path(&path);
    Ok(ReadFileResult { path, content, language })
}

/// Write (overwrite) a file.
#[tauri::command]
pub fn fs_write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("Cannot write {path}: {e}"))
}

/// Create a new empty file.
#[tauri::command]
pub fn fs_create_file(path: String) -> Result<(), String> {
    if let Some(parent) = PathBuf::from(&path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create parent dir: {e}"))?;
    }
    std::fs::write(&path, "").map_err(|e| format!("Cannot create {path}: {e}"))
}

/// Delete a file or directory.
#[tauri::command]
pub fn fs_delete_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| format!("Cannot delete dir: {e}"))
    } else {
        std::fs::remove_file(&path).map_err(|e| format!("Cannot delete file: {e}"))
    }
}

/// Rename or move a file/directory.
#[tauri::command]
pub fn fs_rename(from: String, to: String) -> Result<(), String> {
    std::fs::rename(&from, &to).map_err(|e| format!("Cannot rename: {e}"))
}

/// Open native folder picker — sets workspace root.
#[tauri::command]
pub async fn fs_open_dialog(state: State<'_, WorkspaceState>) -> Result<Option<String>, String> {
    // Returns the path chosen by the user (dialog shown in frontend via Tauri JS API)
    // This command is a lightweight state setter; the actual dialog is invoked in JS.
    Ok(state.0.lock().map(|g| {
        g.as_ref().map(|p| p.to_string_lossy().to_string())
    }).ok().flatten())
}

/// Save-file dialog (returns chosen path).
#[tauri::command]
pub fn fs_save_dialog() -> Result<(), String> {
    // Dialog invoked from frontend via @tauri-apps/plugin-dialog JS API
    Ok(())
}

/// Get the current workspace root.
#[tauri::command]
pub fn fs_get_workspace(state: State<WorkspaceState>) -> Option<String> {
    state
        .0
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|p| p.to_string_lossy().to_string()))
}

/// Set the workspace root (called after user picks a folder).
#[tauri::command]
pub fn fs_set_workspace(
    path: String,
    state: State<WorkspaceState>,
) -> Result<Vec<FileNode>, String> {
    let p = PathBuf::from(&path);
    if !p.exists() || !p.is_dir() {
        return Err(format!("Not a valid directory: {path}"));
    }
    *state.0.lock().map_err(|e| e.to_string())? = Some(p.clone());
    fs_list_dir(path)
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn lang_from_path(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs"               => "rust",
        "ts" | "tsx"       => "typescript",
        "js" | "jsx"       => "javascript",
        "json"             => "json",
        "toml"             => "toml",
        "yaml" | "yml"     => "yaml",
        "md"               => "markdown",
        "css"              => "css",
        "html"             => "html",
        "py"               => "python",
        "sh" | "bash"      => "shell",
        "ps1"              => "powershell",
        "c" | "h"          => "c",
        "cpp" | "hpp"      => "cpp",
        "go"               => "go",
        "java"             => "java",
        "rb"               => "ruby",
        "sql"              => "sql",
        "xml"              => "xml",
        "proto"            => "proto",
        _                  => "plaintext",
    }
    .to_string()
}
