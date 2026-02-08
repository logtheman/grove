use crate::claude::parser::{self, ClaudeSession};
use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const DEBOUNCE_MS: u64 = 1000;
const POLL_INTERVAL_MS: u64 = 200;

pub struct ClaudeMonitorHandle {
    _watcher: RecommendedWatcher,
    shutdown_tx: mpsc::Sender<()>,
}

impl ClaudeMonitorHandle {
    pub fn stop(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Start monitoring Claude Code sessions.
/// Maps sessions to workspaces via the cwd field in JSONL.
/// Emits "claude-sessions-updated" with all active/recent sessions.
pub fn start_monitoring(
    app: AppHandle,
    workspace_paths: Vec<(String, PathBuf)>, // (workspace_id, workspace_path)
) -> Result<ClaudeMonitorHandle> {
    let projects_dir = parser::claude_projects_dir();

    if !projects_dir.exists() {
        anyhow::bail!("~/.claude/projects/ does not exist");
    }

    let (fs_tx, fs_rx) = mpsc::channel::<Event>();
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = fs_tx.send(event);
            }
        },
        Config::default(),
    )?;

    // Watch the projects directory for new/modified JSONL files
    watcher.watch(&projects_dir, RecursiveMode::Recursive)?;

    // Also watch todos directory
    let todos_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude/todos");
    if todos_dir.exists() {
        let _ = watcher.watch(&todos_dir, RecursiveMode::NonRecursive);
    }

    // Build workspace path -> id lookup
    let ws_lookup: HashMap<String, String> = workspace_paths
        .into_iter()
        .map(|(id, path)| (path.to_string_lossy().to_string(), id))
        .collect();

    std::thread::spawn(move || {
        let mut last_check = Instant::now() - Duration::from_secs(60);
        let mut pending = true; // Force initial scan

        loop {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            // Drain events
            while fs_rx.try_recv().is_ok() {
                pending = true;
            }

            let now = Instant::now();
            if pending && now.duration_since(last_check) >= Duration::from_millis(DEBOUNCE_MS) {
                pending = false;
                last_check = now;

                let sessions = scan_and_map_sessions(&projects_dir, &ws_lookup);
                let _ = app.emit("claude-sessions-updated", &sessions);
            }

            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    });

    Ok(ClaudeMonitorHandle {
        _watcher: watcher,
        shutdown_tx,
    })
}

/// Represents a Claude session mapped to a workspace.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedClaudeSession {
    pub session_id: String,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub last_message_type: Option<String>,
    pub last_timestamp: Option<String>,
    pub task_count: parser::TaskCounts,
    pub active: bool,
}

fn scan_and_map_sessions(
    projects_dir: &PathBuf,
    ws_lookup: &HashMap<String, String>,
) -> Vec<MappedClaudeSession> {
    let mut results = Vec::new();

    let entries = match std::fs::read_dir(projects_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let sessions = parser::scan_project_sessions(&path);
        for session in sessions {
            // Only include active sessions (last 10 min) or sessions with in-progress tasks
            if !session.active && session.task_count.in_progress == 0 {
                continue;
            }

            // Map to workspace via cwd
            let workspace_id = session
                .cwd
                .as_ref()
                .and_then(|cwd| ws_lookup.get(cwd).cloned());

            // Also load todo counts from ~/.claude/todos/
            let todo_counts = parser::load_todo_counts(&session.session_id);
            let task_count = if todo_counts.pending > 0
                || todo_counts.in_progress > 0
                || todo_counts.completed > 0
            {
                todo_counts
            } else {
                session.task_count
            };

            results.push(MappedClaudeSession {
                session_id: session.session_id,
                workspace_id,
                workspace_name: None,
                cwd: session.cwd,
                git_branch: session.git_branch,
                model: session.model,
                last_message_type: session.last_message_type,
                last_timestamp: session.last_timestamp,
                task_count,
                active: session.active,
            });
        }
    }

    // Sort by active first, then by timestamp
    results.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.last_timestamp.cmp(&a.last_timestamp))
    });

    results
}
