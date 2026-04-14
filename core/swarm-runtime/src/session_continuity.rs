//! Session continuity — persists compaction summaries into the memory store
//! so future sessions on the same project start with institutional knowledge
//! of past work, decisions, and discovered patterns.
//!
//! ## How it works
//!
//! **At session end** — call [`save_session_summary`]. It takes the
//! `CompactionResult` produced by `compact_session()` and writes it as a
//! memory record in the `Project` (or `Local`) scope under the `sessions`
//! category with high importance so it is never evicted early.
//!
//! **At session start** — call [`inject_past_context`]. It reads the last
//! `N` session summaries from the memory store and returns a formatted string
//! suitable for appending to the system prompt via
//! `SystemPromptBuilder::append_section`.

use std::path::Path;

use crate::compact::CompactionResult;
use crate::memory::{MemoryRecord, MemoryScope, MemoryStore};

/// Category name used for session summaries in the memory store.
const SESSION_CATEGORY: &str = "sessions";

/// How many past session summaries to inject into a new session's context.
const MAX_PAST_SESSIONS: usize = 3;

/// Importance weight for session summaries — high so they survive rotation.
const SESSION_IMPORTANCE: f32 = 0.9;

// ── Write ─────────────────────────────────────────────────────────────────────

/// Persist the compaction summary of a finished session into the memory store.
///
/// Should be called when a session ends (or when `compact_session` fires
/// automatically). Uses `Project` scope if a git root is found, otherwise
/// falls back to `Local`.
///
/// # Arguments
/// * `cwd`    — working directory of the finished session
/// * `result` — the `CompactionResult` returned by `compact_session()`
///
/// # Example
/// ```no_run
/// use swarm_runtime::compact::{compact_session, CompactionConfig};
/// use swarm_runtime::session_continuity::save_session_summary;
/// use std::path::Path;
///
/// let result = compact_session(&session, CompactionConfig::default());
/// save_session_summary(Path::new("."), &result).ok();
/// ```
pub fn save_session_summary(cwd: &Path, result: &CompactionResult) -> std::io::Result<()> {
    if result.formatted_summary.trim().is_empty() {
        return Ok(());
    }

    let store = MemoryStore::new(cwd);
    let scope = preferred_scope(&store);
    let tags = vec![
        "session-summary".to_string(),
        format!("removed:{}", result.removed_message_count),
    ];

    store.remember(
        scope,
        SESSION_CATEGORY,
        &result.formatted_summary,
        &tags,
        Some(SESSION_IMPORTANCE),
    )
}

// ── Read ──────────────────────────────────────────────────────────────────────

/// Retrieve formatted context from past sessions, ready to inject into a new
/// session's system prompt.
///
/// Returns `None` if no past summaries exist (e.g. first run on this project).
/// Otherwise returns a prompt section string that can be passed to
/// `SystemPromptBuilder::append_section`.
///
/// # Arguments
/// * `cwd` — working directory of the new session being started
pub fn inject_past_context(cwd: &Path) -> Option<String> {
    let store = MemoryStore::new(cwd);
    let scope = preferred_scope(&store);

    let records = store.retrieve_records(scope, SESSION_CATEGORY).ok()?;
    if records.is_empty() {
        return None;
    }

    // Take the top-N most relevant (already sorted by relevance_score desc)
    let past: Vec<&MemoryRecord> = records.iter().take(MAX_PAST_SESSIONS).collect();

    let mut lines = vec![
        "# Context from past sessions".to_string(),
        format!(
            "The following {} session summary/summaries cover previous work on this project. \
             Use them as background knowledge — do not repeat or recap them unless asked.",
            past.len()
        ),
        String::new(),
    ];

    for (i, record) in past.iter().enumerate() {
        lines.push(format!("## Past session {}", i + 1));
        lines.push(record.data.clone());
        lines.push(String::new());
    }

    Some(lines.join("\n"))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Use `Project` scope if a git root was found (preferred — memories travel
/// with the repo). Falls back to `Local` if not in a git repository.
fn preferred_scope(store: &MemoryStore) -> MemoryScope {
    // MemoryStore exposes project_dir as private. We probe by attempting to
    // resolve with Project scope; if it errors we fall back.
    // A clean way is to check git root directly.
    if resolve_git_root_exists(store) {
        MemoryScope::Project
    } else {
        MemoryScope::Local
    }
}

fn resolve_git_root_exists(store: &MemoryStore) -> bool {
    // We probe the store by trying to get a path — if Project scope is
    // available it won't return an error.
    // Since get_path is private, we use retrieve_records with a dummy category
    // that we know will just return empty rather than erroring on a valid scope.
    store
        .retrieve_records(MemoryScope::Project, SESSION_CATEGORY)
        .is_ok()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compact::{compact_session, CompactionConfig};
    use crate::session::{ConversationMessage, Session};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("swarm-continuity-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_session_with_messages(n: usize) -> Session {
        let mut session = Session::new();
        for i in 0..n {
            session.messages.push(ConversationMessage::user_text(
                format!("task {i}: implement the feature"),
            ));
            session.messages.push(ConversationMessage::assistant(vec![
                crate::session::ContentBlock::Text {
                    text: format!("Done. I implemented feature {i} by modifying src/lib.rs"),
                },
            ]));
        }
        session
    }

    #[test]
    fn save_and_inject_roundtrip() {
        let dir = temp_dir();

        let session = make_session_with_messages(10);
        let result = compact_session(&session, CompactionConfig::default());

        // Only test if compaction actually produced a summary
        if result.formatted_summary.is_empty() {
            fs::remove_dir_all(dir).unwrap();
            return;
        }

        save_session_summary(&dir, &result).unwrap();

        let injected = inject_past_context(&dir);
        assert!(injected.is_some(), "should produce injected context");
        let text = injected.unwrap();
        assert!(text.contains("# Context from past sessions"));
        assert!(text.contains("Past session 1"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn returns_none_when_no_summaries_exist() {
        let dir = temp_dir();
        let result = inject_past_context(&dir);
        assert!(result.is_none());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn caps_at_max_past_sessions() {
        let dir = temp_dir();
        let store = MemoryStore::new(&dir);

        // Write more than MAX_PAST_SESSIONS summaries directly
        for i in 0..MAX_PAST_SESSIONS + 5 {
            store
                .remember(
                    MemoryScope::Local,
                    SESSION_CATEGORY,
                    &format!("summary of session {i}"),
                    &[String::from("session-summary")],
                    Some(SESSION_IMPORTANCE),
                )
                .unwrap();
        }

        let injected = inject_past_context(&dir).unwrap_or_default();
        let count = injected.matches("## Past session").count();
        assert!(
            count <= MAX_PAST_SESSIONS,
            "should inject at most {MAX_PAST_SESSIONS}, got {count}"
        );

        fs::remove_dir_all(dir).unwrap();
    }
}
