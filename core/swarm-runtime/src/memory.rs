use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Limits ────────────────────────────────────────────────────────────────────
/// Maximum number of records kept per category file before oldest are rotated out.
const MAX_RECORDS_PER_CATEGORY: usize = 200;
/// Maximum file size in bytes per category before rotation is forced.
const MAX_FILE_BYTES: u64 = 512 * 1024; // 512 KB

// ── Scope ─────────────────────────────────────────────────────────────────────

/// Memory store scopes.
///
/// - `Global`  — `~/.config/swarm/memory/`  (follows the user across all projects)
/// - `Project` — `<git-root>/.swarm/memory/` (follows the repo regardless of cwd)
/// - `Local`   — `<cwd>/.swarm/memory/`      (tied to the exact working directory)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Global,
    /// Resolved to the git repository root so memories travel with the repo.
    Project,
    Local,
}

// ── Record ────────────────────────────────────────────────────────────────────

/// A single parsed memory record with full metadata.
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub data: String,
    pub tags: Vec<String>,
    /// Unix timestamp (seconds) when this record was first written.
    pub created_at: u64,
    /// How many times this record has been retrieved.
    pub accessed_count: u32,
    /// Importance score in [0.0, 1.0]. Higher = surfaced first, evicted last.
    pub importance: f32,
}

impl MemoryRecord {
    fn new(data: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            data: data.into(),
            tags,
            created_at: epoch_secs(),
            accessed_count: 0,
            importance: 0.5,
        }
    }

    /// Composite score used for ranking retrieval results and choosing eviction
    /// candidates. Combines importance, recency, and access frequency.
    ///
    /// Score = importance * 0.5 + recency_factor * 0.3 + access_factor * 0.2
    fn relevance_score(&self) -> f32 {
        let now = epoch_secs();
        let age_secs = now.saturating_sub(self.created_at) as f32;
        // Decay to ~0 after 30 days.
        let recency = (-age_secs / (30.0 * 24.0 * 3600.0)).exp();
        let access_factor = (self.accessed_count as f32 / 10.0).min(1.0);
        self.importance * 0.5 + recency * 0.3 + access_factor * 0.2
    }

    // ── Serialisation ─────────────────────────────────────────────────────────

    fn serialize(&self) -> String {
        let tags_line = if self.tags.is_empty() {
            String::new()
        } else {
            format!("# {}\n", self.tags.join(" "))
        };
        format!(
            "--- RECORD ---\n\
             @created:{}\n\
             @accessed:{}\n\
             @importance:{:.3}\n\
             {}{}\n\
             --- END ---\n\n",
            self.created_at,
            self.accessed_count,
            self.importance,
            tags_line,
            self.data.trim_end(),
        )
    }

    fn deserialize(raw: &str) -> Option<Self> {
        // Strip the trailing "--- END ---" if present
        let body = if let Some(idx) = raw.find("--- END ---") {
            raw[..idx].trim()
        } else {
            raw.trim()
        };

        let mut created_at = epoch_secs();
        let mut accessed_count = 0u32;
        let mut importance = 0.5f32;
        let mut tags = Vec::new();
        let mut data_lines: Vec<&str> = Vec::new();

        for line in body.lines() {
            if let Some(v) = line.strip_prefix("@created:") {
                created_at = v.trim().parse().unwrap_or(epoch_secs());
            } else if let Some(v) = line.strip_prefix("@accessed:") {
                accessed_count = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("@importance:") {
                importance = v.trim().parse().unwrap_or(0.5);
            } else if let Some(stripped) = line.strip_prefix('#') {
                // Tag line: "# tag1 tag2"
                tags = stripped.trim().split_whitespace().map(String::from).collect();
            } else {
                data_lines.push(line);
            }
        }

        let data = data_lines.join("\n").trim().to_string();
        if data.is_empty() {
            return None;
        }

        Some(Self { data, tags, created_at, accessed_count, importance })
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MemoryStore {
    global_dir: PathBuf,
    project_dir: Option<PathBuf>,
    local_dir: PathBuf,
}

impl MemoryStore {
    /// Create a new store rooted at `cwd`.
    /// Attempts to resolve the git repo root for the `Project` scope.
    pub fn new(cwd: &Path) -> Self {
        let global_dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("swarm")
            .join("memory");

        let project_dir = resolve_git_root(cwd)
            .map(|root| root.join(".swarm").join("memory"));

        let local_dir = cwd.join(".swarm").join("memory");

        Self { global_dir, project_dir, local_dir }
    }

    // ── Path helpers ──────────────────────────────────────────────────────────

    fn get_dir(&self, scope: MemoryScope) -> Option<&Path> {
        match scope {
            MemoryScope::Global => Some(&self.global_dir),
            MemoryScope::Project => self.project_dir.as_deref(),
            MemoryScope::Local => Some(&self.local_dir),
        }
    }

    /// Returns the file path for a category, validating the category name to
    /// prevent path traversal (fixes the security bug identified in audit).
    fn get_path(&self, scope: MemoryScope, category: &str) -> io::Result<PathBuf> {
        let dir = self.get_dir(scope).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Project scope unavailable: not inside a git repository",
            )
        })?;

        // Reject any category name that contains path separators or dots that
        // could escape the memory directory.
        if category.contains('/') || category.contains('\\') || category.contains("..") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid memory category name: '{category}'"),
            ));
        }

        Ok(dir.join(format!("{category}.mem")))
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Persist a new memory record.
    ///
    /// If the category file exceeds [`MAX_RECORDS_PER_CATEGORY`] records or
    /// [`MAX_FILE_BYTES`] bytes, the lowest-scoring records are evicted first.
    pub fn remember(
        &self,
        scope: MemoryScope,
        category: &str,
        data: &str,
        tags: &[String],
        importance: Option<f32>,
    ) -> io::Result<()> {
        let path = self.get_path(scope, category)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut records = self.load_records(&path);
        let mut new_record = MemoryRecord::new(data, tags.to_vec());
        if let Some(imp) = importance {
            new_record.importance = imp.clamp(0.0, 1.0);
        }
        records.push(new_record);

        // Rotate if over limits
        let needs_rotation = records.len() > MAX_RECORDS_PER_CATEGORY
            || path.metadata().map(|m| m.len()).unwrap_or(0) > MAX_FILE_BYTES;

        if needs_rotation {
            records = rotate_records(records);
        }

        self.save_records(&path, &records)
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Retrieve all records for a category, sorted by relevance score descending.
    /// Increments the access counter for each returned record.
    pub fn retrieve_records(
        &self,
        scope: MemoryScope,
        category: &str,
    ) -> io::Result<Vec<MemoryRecord>> {
        let path = self.get_path(scope, category)?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut records = self.load_records(&path);

        // Increment access counters
        for r in &mut records {
            r.accessed_count = r.accessed_count.saturating_add(1);
        }

        // Sort by relevance: highest score first
        records.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Persist updated access counts
        self.save_records(&path, &records)?;

        Ok(records)
    }

    /// Retrieve records as a flat `HashMap<tag_string, Vec<data>>` — keeps
    /// backward-compatibility with the old `retrieve()` API used by `prompt.rs`.
    pub fn retrieve(
        &self,
        scope: MemoryScope,
        category: &str,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let records = self.retrieve_records(scope, category)?;
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for record in records {
            let key = if record.tags.is_empty() {
                "untagged".to_string()
            } else {
                record.tags.join(" ")
            };
            map.entry(key).or_default().push(record.data);
        }
        Ok(map)
    }

    /// Retrieve all records across every category for a scope.
    pub fn retrieve_all(&self, scope: MemoryScope) -> io::Result<HashMap<String, Vec<String>>> {
        let dir = match self.get_dir(scope) {
            Some(d) => d,
            None => return Ok(HashMap::new()),
        };
        let mut all: HashMap<String, Vec<String>> = HashMap::new();

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".mem") {
                    let category = &name[..name.len() - 4];
                    let records = self.retrieve_records(scope, category)?;
                    let flat: Vec<String> = records.into_iter().map(|r| r.data).collect();
                    if !flat.is_empty() {
                        all.insert(category.to_string(), flat);
                    }
                }
            }
        }
        Ok(all)
    }

    // ── Search ────────────────────────────────────────────────────────────────

    /// Simple keyword search across all records in a scope.
    /// Returns records whose data or tags contain *all* of the given keywords
    /// (case-insensitive), ranked by relevance score.
    pub fn search(
        &self,
        scope: MemoryScope,
        keywords: &[&str],
    ) -> io::Result<Vec<(String, MemoryRecord)>> {
        let dir = match self.get_dir(scope) {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let mut results: Vec<(String, MemoryRecord)> = Vec::new();

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.ends_with(".mem") {
                    continue;
                }
                let category = name[..name.len() - 4].to_string();
                let path = self.get_path(scope, &category)?;
                let records = self.load_records(&path);

                for record in records {
                    let haystack = format!(
                        "{} {}",
                        record.data.to_lowercase(),
                        record.tags.join(" ").to_lowercase()
                    );
                    let matches = keywords
                        .iter()
                        .all(|kw| haystack.contains(&kw.to_lowercase()));
                    if matches {
                        results.push((category.clone(), record));
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            b.1.relevance_score()
                .partial_cmp(&a.1.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    pub fn clear(&self, scope: MemoryScope, category: &str) -> io::Result<()> {
        let path = self.get_path(scope, category)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn clear_all(&self, scope: MemoryScope) -> io::Result<()> {
        if let Some(dir) = self.get_dir(scope) {
            if dir.exists() {
                fs::remove_dir_all(dir)?;
            }
        }
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn load_records(&self, path: &Path) -> Vec<MemoryRecord> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        content
            .split("--- RECORD ---")
            .skip(1) // first element is always empty (content before first marker)
            .filter_map(MemoryRecord::deserialize)
            .collect()
    }

    fn save_records(&self, path: &Path, records: &[MemoryRecord]) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        for record in records {
            file.write_all(record.serialize().as_bytes())?;
        }
        Ok(())
    }
}

// ── Rotation ──────────────────────────────────────────────────────────────────

/// Evict the lowest-scoring records, keeping at most `MAX_RECORDS_PER_CATEGORY`.
fn rotate_records(mut records: Vec<MemoryRecord>) -> Vec<MemoryRecord> {
    // Sort descending by score — best records first
    records.sort_by(|a, b| {
        b.relevance_score()
            .partial_cmp(&a.relevance_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    records.truncate(MAX_RECORDS_PER_CATEGORY);
    records
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Walk up from `start` until a `.git` directory is found.
fn resolve_git_root(start: &Path) -> Option<PathBuf> {
    // Fast path: ask git directly (works even in worktrees)
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
    {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !root.is_empty() {
                return Some(PathBuf::from(root));
            }
        }
    }

    // Fallback: manual walk (no git binary required)
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> (MemoryStore, PathBuf) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("swarm-memory-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let store = MemoryStore::new(&dir);
        (store, dir)
    }

    #[test]
    fn remember_and_retrieve_roundtrip() {
        let (store, dir) = temp_store();
        store
            .remember(
                MemoryScope::Local,
                "prefs",
                "prefer snake_case",
                &[String::from("style")],
                Some(0.8),
            )
            .unwrap();

        let records = store.retrieve_records(MemoryScope::Local, "prefs").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data, "prefer snake_case");
        assert_eq!(records[0].tags, vec!["style"]);
        assert!((records[0].importance - 0.8).abs() < 0.01);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_path_traversal_in_category() {
        let (store, dir) = temp_store();
        let result = store.remember(
            MemoryScope::Local,
            "../../etc/passwd",
            "bad",
            &[],
            None,
        );
        assert!(result.is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rotates_when_over_record_limit() {
        let (store, dir) = temp_store();
        for i in 0..=MAX_RECORDS_PER_CATEGORY + 5 {
            store
                .remember(
                    MemoryScope::Local,
                    "overflow",
                    &format!("entry {i}"),
                    &[],
                    None,
                )
                .unwrap();
        }
        let records = store
            .retrieve_records(MemoryScope::Local, "overflow")
            .unwrap();
        assert!(
            records.len() <= MAX_RECORDS_PER_CATEGORY,
            "should rotate excess records, got {}",
            records.len()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn access_count_increments_on_retrieve() {
        let (store, dir) = temp_store();
        store
            .remember(MemoryScope::Local, "cats", "data", &[], None)
            .unwrap();
        let _ = store.retrieve_records(MemoryScope::Local, "cats").unwrap();
        let records = store.retrieve_records(MemoryScope::Local, "cats").unwrap();
        assert!(records[0].accessed_count >= 1, "access count should increment");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn search_filters_by_keyword() {
        let (store, dir) = temp_store();
        store
            .remember(MemoryScope::Local, "code", "use async/await in Rust", &[], None)
            .unwrap();
        store
            .remember(MemoryScope::Local, "code", "prefer Python for scripts", &[], None)
            .unwrap();

        let results = store.search(MemoryScope::Local, &["rust"]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.data.contains("Rust"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn backward_compatible_retrieve_returns_hashmap() {
        let (store, dir) = temp_store();
        store
            .remember(
                MemoryScope::Local,
                "legacy",
                "some data",
                &[String::from("tag1")],
                None,
            )
            .unwrap();
        let map = store.retrieve(MemoryScope::Local, "legacy").unwrap();
        assert!(!map.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn high_importance_records_rank_first() {
        let (store, dir) = temp_store();
        store
            .remember(MemoryScope::Local, "rank", "low priority", &[], Some(0.1))
            .unwrap();
        store
            .remember(MemoryScope::Local, "rank", "high priority", &[], Some(0.95))
            .unwrap();

        let records = store.retrieve_records(MemoryScope::Local, "rank").unwrap();
        assert_eq!(records[0].data, "high priority");
        fs::remove_dir_all(dir).unwrap();
    }
}
