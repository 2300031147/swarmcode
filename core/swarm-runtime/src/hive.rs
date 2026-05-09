use std::collections::BTreeMap;
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn epoch_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

/// A message exchanged between agents in a team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiveMessage {
    pub from: String,
    pub to: Option<String>,
    pub body: String,
    pub timestamp_epoch_ms: u64,
}

/// A task in the shared task list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub id: String,
    pub description: String,
    pub status: TeamTaskStatus,
    pub assigned_to: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamTaskStatus {
    Pending,
    Claimed,
    Done,
}

/// Registration metadata for a single agent inside a team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiveMember {
    pub id: String,
    pub name: String,
    pub description: String,
    pub role: HiveRole,
    pub status: HiveMemberStatus,
    pub last_seen_epoch_ms: u64,
}

/// Global team usage metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamUsage {
    pub total: crate::usage::TokenUsage,
    pub by_agent: BTreeMap<String, crate::usage::TokenUsage>,
}

/// The role an agent plays within a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiveRole {
    Lead,
    Teammate,
}

/// The current execution status of a team member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiveMemberStatus {
    Active,
    Idle,
    Completed,
    Failed,
}

/// Shared state that manages an agent team.
///
/// This struct is designed to be wrapped in `Arc` so that all agents
/// (which run on separate threads) can share the same team context.
///
/// It does *not* use async channels — the Claw runtime is synchronous
/// (`std::thread`-based), so we use a simple `Mutex<Vec<HiveMessage>>`
/// per agent as a mailbox.  An agent can drain its mailbox at the top
/// of each conversation-loop iteration without blocking.
#[derive(Debug)]
pub struct SwarmHive {
    inner: Mutex<SwarmHiveInner>,
}

#[derive(Debug)]
struct SwarmHiveInner {
    members: BTreeMap<String, HiveMember>,
    /// Per-agent inbox.  Key = agent id, Value = pending messages.
    inboxes: BTreeMap<String, Vec<HiveMessage>>,
    /// Global log of all messages for audit / debug purposes.
    log: Vec<HiveMessage>,
    /// Cumulative usage data.
    usage: TeamUsage,
    /// The directory where team state is persisted.
    team_dir: Option<PathBuf>,
    /// The ID of the team.
    team_id: Option<String>,
    /// The shared task list.
    tasks: BTreeMap<String, TeamTask>,
    /// The index in the log that has been synced to disk.
    last_synced_idx: usize,
    /// [INTEGRATION] Optional listener for network-scale broadcasts.
    #[serde(skip)]
    listener: Option<Box<dyn Fn(HiveMessage) + Send + Sync>>,
    /// Notify waiters when a new message arrives.
    #[serde(skip)]
    message_condvar: std::sync::Arc<std::sync::Condvar>,
}

impl SwarmHive {
    /// Create a new empty hub.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SwarmHiveInner {
                members: BTreeMap::new(),
                inboxes: BTreeMap::new(),
                log: Vec::new(),
                usage: TeamUsage::default(),
                team_dir: None,
                team_id: None,
                tasks: BTreeMap::new(),
                last_synced_idx: 0,
                listener: None,
                message_condvar: std::sync::Arc::new(std::sync::Condvar::new()),
            }),
        }
    }

    /// Return the condvar used for message notifications.
    pub fn message_condvar(&self) -> std::sync::Arc<std::sync::Condvar> {
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.message_condvar.clone()
    }

    /// Block the current thread until a message arrives or timeout.
    pub fn wait_for_messages(&self, timeout: std::time::Duration) {
        let condvar = self.message_condvar();
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        let _ = condvar.wait_timeout(inner, timeout).ok();
    }

    /// Enable file-backed persistence for this hub.
    pub fn with_persistence(self, team_id: String) -> Self {
        let team_dir = team_dir().join(&team_id);
        fs::create_dir_all(&team_dir).ok();
        fs::create_dir_all(team_dir.join("locks")).ok();
        fs::create_dir_all(team_dir.join("tasks")).ok();

        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.team_id = Some(team_id);
            inner.team_dir = Some(team_dir);
        }

        // Load existing messages if the file exists
        self.reload_from_disk();
        self
    }

    /// Register a new agent in the team.
    pub fn register(&self, mut member: HiveMember) {
        member.last_seen_epoch_ms = epoch_ms_now();
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            let id = member.id.clone();
            inner.members.insert(id.clone(), member);
            inner.inboxes.entry(id).or_default();
        }
        self.sync_members_to_disk();
    }


    /// Remove an agent from the team (e.g. on completion / failure).
    pub fn unregister(&self, agent_id: &str) {
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.members.remove(agent_id);
            inner.inboxes.remove(agent_id);
        }
        self.sync_members_to_disk();
    }


    /// Update the status of an existing member.
    pub fn set_status(&self, agent_id: &str, status: HiveMemberStatus) {
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            if let Some(member) = inner.members.get_mut(agent_id) {
                member.status = status;
                member.last_seen_epoch_ms = epoch_ms_now();
            }
        }
        self.sync_members_to_disk();
    }

    /// Record usage for a specific member and update the team total.
    pub fn record_usage(&self, agent_id: &str, usage: crate::usage::TokenUsage) {
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            let agent_usage = inner.usage.by_agent.entry(agent_id.to_string()).or_default();
            
            agent_usage.input_tokens += usage.input_tokens;
            agent_usage.output_tokens += usage.output_tokens;
            agent_usage.cache_creation_input_tokens += usage.cache_creation_input_tokens;
            agent_usage.cache_read_input_tokens += usage.cache_read_input_tokens;

            inner.usage.total.input_tokens += usage.input_tokens;
            inner.usage.total.output_tokens += usage.output_tokens;
            inner.usage.total.cache_creation_input_tokens += usage.cache_creation_input_tokens;
            inner.usage.total.cache_read_input_tokens += usage.cache_read_input_tokens;
            
            if let Some(member) = inner.members.get_mut(agent_id) {
                member.last_seen_epoch_ms = epoch_ms_now();
            }
        }
        self.sync_usage_to_disk();
    }
    
    /// Add a new task to the team.
    pub fn add_task(&self, description: &str) -> String {
        let id = format!("task-{}", epoch_ms_now());
        let task = TeamTask {
            id: id.clone(),
            description: description.to_string(),
            status: TeamTaskStatus::Pending,
            assigned_to: None,
            created_at_ms: epoch_ms_now(),
        };
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.tasks.insert(id.clone(), task);
        }
        self.sync_tasks_to_disk();
        id
    }

    /// List all tasks in the team.
    pub fn tasks(&self) -> Vec<TeamTask> {
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.tasks.values().cloned().collect()
    }

    /// Claim a task for a specific agent.
    pub fn claim_task(&self, task_id: &str, agent_id: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        if let Some(task) = inner.tasks.get_mut(task_id) {
            task.status = TeamTaskStatus::Claimed;
            task.assigned_to = Some(agent_id.to_string());
            drop(inner);
            self.sync_tasks_to_disk();
            Ok(())
        } else {
            Err(format!("Task {task_id} not found"))
        }
    }

    /// Mark a task as completed.
    pub fn complete_task(&self, task_id: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        if let Some(task) = inner.tasks.get_mut(task_id) {
            task.status = TeamTaskStatus::Done;
            drop(inner);
            self.sync_tasks_to_disk();
            Ok(())
        } else {
            Err(format!("Task {task_id} not found"))
        }
    }

    fn sync_tasks_to_disk(&self) {
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        if let Some(dir) = &inner.team_dir {
            let path = dir.join("tasks.jsonl");
            if let Ok(mut file) = OpenOptions::new().create(true).write(true).truncate(true).open(path) {
                for task in inner.tasks.values() {
                    if let Ok(json) = serde_json::to_string(task) {
                        writeln!(file, "{json}").ok();
                    }
                }
            }
        }
    }

    /// Prune members who haven't sent a heartbeat for more than 60 seconds.
    pub fn prune_stale_members(&self) {
        let now = epoch_ms_now();
        let mut changed = false;
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            for member in inner.members.values_mut() {
                if member.status == HiveMemberStatus::Active && (now - member.last_seen_epoch_ms) > 60_000 {
                    member.status = HiveMemberStatus::Failed;
                    changed = true;
                }
            }
        }
        if changed {
            self.sync_members_to_disk();
        }
    }

    /// Return the total team usage.
    pub fn total_usage(&self) -> crate::usage::TokenUsage {
        self.reload_from_disk();
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.usage.total
    }

    fn sync_usage_to_disk(&self) {
        let team_dir = {
            let inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.team_dir.clone()
        };
        let Some(team_dir) = team_dir else { return };
        let usage_path = team_dir.join("usage.json");
        
        let _ = self.with_file_lock(&usage_path, || {
            let inner = self.inner.lock().expect("SwarmHive lock poisoned");
            let json = serde_json::to_string(&inner.usage).expect("Usage serialization failed");
            fs::write(&usage_path, json).ok();
            Ok(())
        });
    }

    fn sync_members_to_disk(&self) {
        let team_dir = {
            let inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.team_dir.clone()
        };
        let Some(team_dir) = team_dir else { return };
        let members_path = team_dir.join("members.json");
        
        let _ = self.with_file_lock(&members_path, || {
            let inner = self.inner.lock().expect("SwarmHive lock poisoned");
            let json = serde_json::to_string(&inner.members).expect("Members serialization failed");
            fs::write(&members_path, json).ok();
            Ok(())
        });
    }


    /// Set a listener to be notified of all messages.
    pub fn set_listener<F>(&self, f: F)
    where
        F: Fn(HiveMessage) + Send + Sync + 'static,
    {
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.listener = Some(Box::new(f));
    }

    /// Send a message to a specific agent.
    ///
    /// Returns `Err` if the recipient is not registered.
    pub fn send_to(&self, message: HiveMessage) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            
            // Notify listener if present
            if let Some(ref listener) = inner.listener {
                listener(message.clone());
            }

            let recipient = message
                .to
                .as_deref()
                .ok_or_else(|| String::from("send_to requires a recipient"))?;
            let inbox = inner
                .inboxes
                .get_mut(recipient)
                .ok_or_else(|| format!("unknown teammate: {recipient}"))?;
            inbox.push(message.clone());
            inner.log.push(message);
            inner.message_condvar.notify_all();
        }
        self.sync_to_disk();
        Ok(())
    }

    /// Broadcast a message to all agents except the sender.
    pub fn broadcast(&self, message: HiveMessage) {
        {
            let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
            
            // Notify listener if present
            if let Some(ref listener) = inner.listener {
                listener(message.clone());
            }

            for (id, inbox) in &mut inner.inboxes {
                if *id != message.from {
                    inbox.push(message.clone());
                }
            }
            inner.log.push(message);
            inner.message_condvar.notify_all();
        }
        self.sync_to_disk();
    }

    /// Drain all pending messages for the given agent.
    pub fn drain_inbox(&self, agent_id: &str) -> Vec<HiveMessage> {
        self.reload_from_disk();
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner
            .inboxes
            .get_mut(agent_id)
            .map(std::mem::take)
            .unwrap_or_default()
    }

    fn sync_to_disk(&self) {
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        let Some(team_dir) = &inner.team_dir else { return };
        let messages_path = team_dir.join("messages.jsonl");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(messages_path)
            .expect("Failed to open messages.jsonl for syncing");

        let to_sync = &inner.log[inner.last_synced_idx..];
        for msg in to_sync {
            let json = serde_json::to_string(msg).expect("Message serialization failed");
            writeln!(file, "{json}").ok();
        }
        inner.last_synced_idx = inner.log.len();
    }

    fn reload_from_disk(&self) {
        let mut inner = self.inner.lock().expect("SwarmHive lock poisoned");
        let Some(team_dir) = &inner.team_dir else { return };
        let messages_path = team_dir.join("messages.jsonl");

        if !messages_path.exists() { return; }

        let file = fs::File::open(messages_path).expect("Failed to open messages.jsonl");
        let reader = io::BufReader::new(file);
        
        let mut lines = reader.lines().peekable();
        // Skip lines we've already seen in the log
        // This is a simple heuristic: we assume the file is append-only 
        // and matches our log's initial segments.
        let mut current_idx = 0;
        
        while lines.peek().is_some() {
            let line = lines.next().unwrap().unwrap_or_default();
            if line.trim().is_empty() { continue; }
            
            if let Ok(msg) = serde_json::from_str::<HiveMessage>(line.trim()) {
                if current_idx >= inner.log.len() {
                    // New message found!
                    inner.log.push(msg.clone());

                    // Sort into individual inboxes
                    if let Some(recipient) = &msg.to {
                        if let Some(inbox) = inner.inboxes.get_mut(recipient) {
                            inbox.push(msg.clone());
                        }
                    } else {
                        for (id, inbox) in &mut inner.inboxes {
                            if *id != msg.from {
                                inbox.push(msg.clone());
                            }
                        }
                    }
                }
                current_idx += 1;
            }
        }
        
        inner.last_synced_idx = inner.log.len();

        // Load existing members if the file exists
        let members_path = team_dir.join("members.json");
        if members_path.exists() {
            if let Ok(content) = fs::read_to_string(members_path) {
                if let Ok(members) = serde_json::from_str::<BTreeMap<String, HiveMember>>(&content) {
                    for (id, member) in members {
                        if !inner.members.contains_key(&id) {
                            inner.members.insert(id.clone(), member);
                            inner.inboxes.entry(id).or_default();
                        }
                    }
                }
            }
        }

        // Load usage if the file exists
        let usage_path = team_dir.join("usage.json");
        if usage_path.exists() {
            if let Ok(content) = fs::read_to_string(usage_path) {
                if let Ok(usage) = serde_json::from_str::<TeamUsage>(&content) {
                    inner.usage = usage;
                }
            }
        }
    }

    /// List all current members and their statuses.
    #[must_use]
    pub fn members(&self) -> Vec<HiveMember> {
        self.prune_stale_members();
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.members.values().cloned().collect()
    }

    /// Return the full message log (for debugging / UI).
    #[must_use]
    pub fn message_log(&self) -> Vec<HiveMessage> {
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.log.clone()
    }

    /// Return the team ID.
    pub fn team_id(&self) -> Option<String> {
        let inner = self.inner.lock().expect("SwarmHive lock poisoned");
        inner.team_id.clone()
    }

    /// Run a function under a distributed file lock.
    /// This prevents multiple agents from editing the same file simultaneously.
    pub fn with_file_lock<F, T>(&self, path: &Path, f: F) -> Result<T, String>
    where
        F: FnOnce() -> T,
    {
        let team_dir = {
            let inner = self.inner.lock().expect("SwarmHive lock poisoned");
            inner.team_dir.clone()
        };
        let Some(team_dir) = team_dir else {
            return Ok(f());
        };

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let lock_name = format!("{:x}.lock", hasher.finalize());
        let lock_path = team_dir.join("locks").join(lock_name);

        // Simple spin-lock with retries and stale lock detection
        let mut attempts = 0;
        loop {
            match fs::create_dir(&lock_path) {
                Ok(_) => {
                    // Lock acquired!
                    let result = f();
                    let _ = fs::remove_dir(&lock_path);
                    return Ok(result);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Check for stale lock
                    if let Ok(metadata) = fs::metadata(&lock_path) {
                        if let Ok(created) = metadata.created() {
                            if let Ok(elapsed) = created.elapsed() {
                                if elapsed.as_secs() > 300 {
                                    // Lock is older than 5 minutes, likely stale
                                    let _ = fs::remove_dir(&lock_path);
                                    continue; // Try again immediately
                                }
                            }
                        }
                    }

                    attempts += 1;
                    if attempts > 300 {
                        return Err(format!(
                            "Timeout waiting for lock on file: {}. Please check if another agent is stuck.",
                            path.display()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(format!("Failed to create lock: {e}")),
            }
        }
    }
}

impl Default for SwarmHive {
    fn default() -> Self {
        Self::new()
    }
}

fn team_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".clawswarm").join("teams")
}

/// Convenience constructor for a `HiveMessage`.
#[must_use]
pub fn team_message(from: &str, to: Option<&str>, body: &str) -> HiveMessage {
    HiveMessage {
        from: from.to_string(),
        to: to.map(ToString::to_string),
        body: body.to_string(),
        timestamp_epoch_ms: epoch_ms_now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lead() -> HiveMember {
        HiveMember {
            id: "lead-1".to_string(),
            name: "lead".to_string(),
            description: "team lead".to_string(),
            role: HiveRole::Lead,
            status: HiveMemberStatus::Active,
        }
    }

    fn teammate(id: &str) -> HiveMember {
        HiveMember {
            id: id.to_string(),
            name: id.to_string(),
            description: format!("teammate {id}"),
            role: HiveRole::Teammate,
            status: HiveMemberStatus::Active,
        }
    }

    #[test]
    fn register_and_list_members() {
        let hub = SwarmHive::new();
        hub.register(lead());
        hub.register(teammate("t-1"));
        let members = hub.members();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn send_and_drain_inbox() {
        let hub = SwarmHive::new();
        hub.register(lead());
        hub.register(teammate("t-1"));

        hub.send_to(team_message("lead-1", Some("t-1"), "hello teammate"))
            .expect("send should succeed");

        let msgs = hub.drain_inbox("t-1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].body, "hello teammate");

        // Inbox should be empty after drain.
        assert!(hub.drain_inbox("t-1").is_empty());
    }

    #[test]
    fn broadcast_excludes_sender() {
        let hub = SwarmHive::new();
        hub.register(lead());
        hub.register(teammate("t-1"));
        hub.register(teammate("t-2"));

        hub.broadcast(team_message("lead-1", None, "status update"));

        assert_eq!(hub.drain_inbox("t-1").len(), 1);
        assert_eq!(hub.drain_inbox("t-2").len(), 1);
        assert!(hub.drain_inbox("lead-1").is_empty());
    }

    #[test]
    fn send_to_unknown_agent_returns_error() {
        let hub = SwarmHive::new();
        hub.register(lead());

        let result = hub.send_to(team_message("lead-1", Some("ghost"), "msg"));
        assert!(result.is_err());
    }

    #[test]
    fn unregister_removes_member_and_inbox() {
        let hub = SwarmHive::new();
        hub.register(lead());
        hub.register(teammate("t-1"));

        hub.unregister("t-1");
        assert_eq!(hub.members().len(), 1);
        assert!(hub.drain_inbox("t-1").is_empty());
    }

    #[test]
    fn set_status_updates_member() {
        let hub = SwarmHive::new();
        hub.register(teammate("t-1"));

        hub.set_status("t-1", HiveMemberStatus::Completed);
        let members = hub.members();
        assert_eq!(members[0].status, HiveMemberStatus::Completed);
    }
}
