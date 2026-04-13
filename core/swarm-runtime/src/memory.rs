use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Memory store scopes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Global,
    Local,
}

#[derive(Debug, Clone)]
pub struct MemoryStore {
    global_dir: PathBuf,
    local_dir: PathBuf,
}

impl MemoryStore {
    pub fn new(cwd: &Path) -> Self {
        let global_dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("swarm")
            .join("memory");
            
        let local_dir = cwd.join(".swarm").join("memory");
        
        Self {
            global_dir,
            local_dir,
        }
    }

    fn get_dir(&self, scope: MemoryScope) -> &Path {
        match scope {
            MemoryScope::Global => &self.global_dir,
            MemoryScope::Local => &self.local_dir,
        }
    }

    fn get_path(&self, scope: MemoryScope, category: &str) -> PathBuf {
        self.get_dir(scope).join(format!("{}.txt", category))
    }

    /// Store a memory with optional tags
    pub fn remember(
        &self,
        scope: MemoryScope,
        category: &str,
        data: &str,
        tags: &[String],
    ) -> io::Result<()> {
        let path = self.get_path(scope, category);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;

        // Use more robust record boundaries
        writeln!(file, "--- RECORD ---")?;
        if !tags.is_empty() {
            writeln!(file, "# {}", tags.join(" "))?;
        }
        writeln!(file, "{}", data.trim_end())?;
        writeln!(file, "--- END ---\n")?;

        Ok(())
    }

    /// Retrieve memories for a specific category
    pub fn retrieve(
        &self,
        scope: MemoryScope,
        category: &str,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let path = self.get_path(scope, category);
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let mut file = fs::File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut memories = HashMap::new();
        // Split by the robust boundary
        for entry in content.split("--- RECORD ---") {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            
            // The entry might look like:
            // # tag1 tag2
            // memory data
            // --- END ---
            
            let data_without_end = if let Some(idx) = entry.find("--- END ---") {
                &entry[..idx]
            } else {
                entry
            };

            let mut lines = data_without_end.trim().lines();
            if let Some(first_line) = lines.next() {
                if let Some(stripped) = first_line.strip_prefix('#') {
                    let tags = stripped.trim().to_string();
                    let data = lines.collect::<Vec<_>>().join("\n");
                    if !data.is_empty() {
                        memories
                            .entry(tags)
                            .or_insert_with(Vec::new)
                            .push(data);
                    }
                } else {
                    let mut data_lines = vec![first_line.to_string()];
                    data_lines.extend(lines.map(String::from));
                    let data = data_lines.join("\n");
                    if !data.is_empty() {
                        memories
                            .entry("untagged".to_string())
                            .or_insert_with(Vec::new)
                            .push(data);
                    }
                }
            }
        }

        Ok(memories)
    }

    /// Retrieve all memories across all categories for a scope
    pub fn retrieve_all(&self, scope: MemoryScope) -> io::Result<HashMap<String, Vec<String>>> {
        let dir = self.get_dir(scope);
        let mut all_memories = HashMap::new();

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".txt") {
                        let category = &name[..name.len() - 4];
                        let memories = self.retrieve(scope, category)?;
                        let flat_memories: Vec<String> = memories.into_values().flatten().collect();
                        if !flat_memories.is_empty() {
                            all_memories.insert(category.to_string(), flat_memories);
                        }
                    }
                }
            }
        }

        Ok(all_memories)
    }

    pub fn clear(&self, scope: MemoryScope, category: &str) -> io::Result<()> {
        let path = self.get_path(scope, category);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn clear_all(&self, scope: MemoryScope) -> io::Result<()> {
        let dir = self.get_dir(scope);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}
