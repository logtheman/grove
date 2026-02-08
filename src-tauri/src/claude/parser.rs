use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// A parsed Claude Code session with its latest state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSession {
    pub session_id: String,
    pub project_path: String,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub last_message_type: Option<String>,
    pub last_timestamp: Option<String>,
    pub task_count: TaskCounts,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TaskCounts {
    pub pending: u32,
    pub in_progress: u32,
    pub completed: u32,
}

/// Lightweight message fields we extract from JSONL lines.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    timestamp: Option<String>,
    message: Option<MessageInfo>,
    #[allow(dead_code)]
    todos: Option<Vec<TodoItem>>,
}

#[derive(Deserialize)]
struct MessageInfo {
    #[allow(dead_code)]
    role: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct TodoItem {
    status: Option<String>,
}

/// Parse the tail of a JSONL session file to extract current state.
/// Only reads the last N bytes to avoid loading huge files.
pub fn parse_session_tail(jsonl_path: &Path, max_bytes: u64) -> Result<ClaudeSession> {
    let session_id = jsonl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let project_path = jsonl_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(decode_project_path)
        .unwrap_or_default();

    let file = fs::File::open(jsonl_path)?;
    let file_len = file.metadata()?.len();

    let mut reader = BufReader::new(file);

    // Seek to near the end for large files
    if file_len > max_bytes {
        reader.seek(SeekFrom::End(-(max_bytes as i64)))?;
        // Skip partial first line
        let mut discard = String::new();
        let _ = reader.read_line(&mut discard);
    }

    let mut session = ClaudeSession {
        session_id,
        project_path,
        cwd: None,
        git_branch: None,
        model: None,
        last_message_type: None,
        last_timestamp: None,
        task_count: TaskCounts::default(),
        active: false,
    };

    let mut last_todos: Vec<TodoItem> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        let entry: JournalEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if let Some(ref t) = entry.entry_type {
            // Skip snapshot entries
            if t == "file-history-snapshot" {
                continue;
            }
            session.last_message_type = Some(t.clone());
        }

        if let Some(cwd) = entry.cwd {
            session.cwd = Some(cwd);
        }
        if let Some(branch) = entry.git_branch {
            session.git_branch = Some(branch);
        }
        if let Some(ts) = entry.timestamp {
            session.last_timestamp = Some(ts);
        }
        if let Some(msg) = entry.message {
            if let Some(model) = msg.model {
                session.model = Some(model);
            }
        }
        if let Some(todos) = entry.todos {
            if !todos.is_empty() {
                last_todos = todos;
            }
        }
    }

    // Count todos
    for todo in &last_todos {
        match todo.status.as_deref() {
            Some("pending") => session.task_count.pending += 1,
            Some("in_progress") => session.task_count.in_progress += 1,
            Some("completed") => session.task_count.completed += 1,
            _ => {}
        }
    }

    // Determine if session is "active" based on last timestamp
    session.active = is_recently_active(&session.last_timestamp);

    Ok(session)
}

/// Scan all JSONL files in a project directory.
pub fn scan_project_sessions(project_dir: &Path) -> Vec<ClaudeSession> {
    let mut sessions = Vec::new();

    let entries = match fs::read_dir(project_dir) {
        Ok(e) => e,
        Err(_) => return sessions,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            // Only read last 64KB of each file
            if let Ok(session) = parse_session_tail(&path, 64 * 1024) {
                sessions.push(session);
            }
        }
    }

    // Sort by timestamp descending (most recent first)
    sessions.sort_by(|a, b| b.last_timestamp.cmp(&a.last_timestamp));
    sessions
}

/// Load task counts from ~/.claude/todos/ for a session.
pub fn load_todo_counts(session_id: &str) -> TaskCounts {
    let todos_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude/todos");

    let mut counts = TaskCounts::default();

    // Todo files can be named {session_id}.json or {session_id}-agent-{...}.json
    let entries = match fs::read_dir(&todos_dir) {
        Ok(e) => e,
        Err(_) => return counts,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(session_id) || !name.ends_with(".json") {
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let todos: Vec<TodoItem> = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for todo in &todos {
            match todo.status.as_deref() {
                Some("pending") => counts.pending += 1,
                Some("in_progress") => counts.in_progress += 1,
                Some("completed") => counts.completed += 1,
                _ => {}
            }
        }
    }

    counts
}

/// Convert encoded project path back to real path.
/// e.g. "-home-bento-carrot-customers-ursa" -> "/home/bento/carrot/customers/ursa"
fn decode_project_path(encoded: &str) -> String {
    if encoded.starts_with('-') {
        format!("/{}", encoded[1..].replace('-', "/"))
    } else {
        encoded.replace('-', "/")
    }
}

/// Check if a timestamp is within the last 10 minutes.
fn is_recently_active(timestamp: &Option<String>) -> bool {
    let ts = match timestamp {
        Some(t) => t,
        None => return false,
    };

    // Parse ISO 8601 timestamp
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) {
        let age = chrono::Utc::now() - parsed.with_timezone(&chrono::Utc);
        age.num_minutes() < 10
    } else {
        false
    }
}

/// Get the Claude projects directory.
pub fn claude_projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude/projects")
}
