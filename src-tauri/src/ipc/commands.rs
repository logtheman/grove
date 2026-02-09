use crate::claude::monitor as claude_monitor;
use crate::git::status as git_status;
use crate::git::watcher;
use crate::ipc::events;
use crate::pty::manager::PtySession;
use crate::state::AppState;
use crate::workspace::manager::Workspace;
use crate::workspace::state as ws_state;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

// ── Terminal commands ──

#[derive(Serialize)]
pub struct TerminalInfo {
    pub id: String,
    pub cwd: String,
}

#[tauri::command]
pub fn write_debug_log(path: String, content: String) -> Result<(), String> {
    use std::fs;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write debug log: {}", e))?;
    eprintln!("[grove-cmd] Debug log written to: {}", path);
    Ok(())
}

#[tauri::command]
pub fn create_terminal(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    cwd: Option<String>,
) -> Result<TerminalInfo, String> {
    let terminal_id = uuid::Uuid::new_v4().to_string();
    let working_dir = cwd.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    });

    eprintln!("[grove-cmd] create_terminal: id={}, cwd={}", terminal_id, working_dir);

    let data_event_id = terminal_id.clone();
    let data_app_handle = app.clone();

    let exit_event_id = terminal_id.clone();
    let exit_app_handle = app.clone();

    let session = PtySession::spawn(
        &working_dir,
        24,
        80,
        move |data| {
            let event_name = format!("{}:{}", events::TERMINAL_DATA, data_event_id);
            eprintln!("[grove-cmd] emitting {} with {} bytes", event_name, data.len());
            let result = data_app_handle.emit(&event_name, &data);
            if let Err(e) = result {
                eprintln!("[grove-cmd] emit data error: {}", e);
            } else {
                eprintln!("[grove-cmd] emit successful");
            }
        },
        move || {
            eprintln!("[grove-cmd] terminal exited: {}", exit_event_id);
            let event_name = format!("{}:{}", events::TERMINAL_EXIT, exit_event_id);
            let _ = exit_app_handle.emit(&event_name, ());
        },
    )
    .map_err(|e| format!("Failed to create terminal: {}", e))?;

    let info = TerminalInfo {
        id: terminal_id.clone(),
        cwd: working_dir,
    };

    state.terminals.lock().insert(terminal_id.clone(), session);
    eprintln!("[grove-cmd] terminal created successfully: {}", terminal_id);

    Ok(info)
}

#[tauri::command]
pub fn close_terminal(
    state: tauri::State<'_, Arc<AppState>>,
    terminal_id: String,
) -> Result<(), String> {
    let mut terminals = state.terminals.lock();
    if let Some(session) = terminals.remove(&terminal_id) {
        session.shutdown();
        Ok(())
    } else {
        Err(format!("Terminal {} not found", terminal_id))
    }
}

#[tauri::command]
pub fn write_to_terminal(
    state: tauri::State<'_, Arc<AppState>>,
    terminal_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut terminals = state.terminals.lock();
    if let Some(session) = terminals.get_mut(&terminal_id) {
        session
            .write(&data)
            .map_err(|e| {
                eprintln!("[grove-cmd] write_to_terminal error: {}", e);
                format!("Write failed: {}", e)
            })
    } else {
        eprintln!("[grove-cmd] write_to_terminal: terminal {} not found", terminal_id);
        Err(format!("Terminal {} not found", terminal_id))
    }
}

#[tauri::command]
pub fn resize_terminal(
    state: tauri::State<'_, Arc<AppState>>,
    terminal_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let terminals = state.terminals.lock();
    if let Some(session) = terminals.get(&terminal_id) {
        session
            .resize(rows, cols)
            .map_err(|e| format!("Resize failed: {}", e))
    } else {
        Err(format!("Terminal {} not found", terminal_id))
    }
}

#[tauri::command]
pub fn list_terminals(state: tauri::State<'_, Arc<AppState>>) -> Vec<String> {
    state.terminals.lock().keys().cloned().collect()
}

#[tauri::command]
pub fn get_terminal_cwd(
    state: tauri::State<'_, Arc<AppState>>,
    terminal_id: String,
) -> Result<String, String> {
    let terminals = state.terminals.lock();
    if let Some(session) = terminals.get(&terminal_id) {
        Ok(session.get_cwd())
    } else {
        Err(format!("Terminal {} not found", terminal_id))
    }
}

#[tauri::command]
pub async fn execute_in_terminal(
    state: tauri::State<'_, Arc<AppState>>,
    terminal_id: String,
    command: String,
) -> Result<String, String> {
    eprintln!("[grove-cmd] execute_in_terminal: {} in {}", command, terminal_id);

    // Generate a unique ID for this command execution
    let cmd_id = uuid::Uuid::new_v4().to_string();

    // Wrap command with markers for output capture
    let wrapped_command = format!(
        "echo 'GROVE_EXEC_START_{}' && {} && echo 'GROVE_EXEC_END_{}'\n",
        cmd_id, command, cmd_id
    );

    // Send command to terminal
    {
        let mut terminals = state.terminals.lock();
        if let Some(session) = terminals.get_mut(&terminal_id) {
            session.write(wrapped_command.as_bytes())
                .map_err(|e| format!("Failed to write command: {}", e))?;
        } else {
            return Err(format!("Terminal {} not found", terminal_id));
        }
    }

    // Wait for command to complete (poll with timeout)
    let max_wait = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_millis(100);

    loop {
        if start.elapsed() > max_wait {
            // Clean up on timeout
            let terminals = state.terminals.lock();
            if let Some(session) = terminals.get(&terminal_id) {
                let mut pending_commands = session.pending_commands.lock().unwrap();
                pending_commands.remove(&cmd_id);
                eprintln!("[grove-cmd] execute_in_terminal: timeout, cleaned up command {}", cmd_id);
            }
            return Err("Command execution timeout".to_string());
        }

        // Check if command is complete
        let is_complete = {
            let terminals = state.terminals.lock();
            if let Some(session) = terminals.get(&terminal_id) {
                let pending_commands = session.pending_commands.lock().unwrap();
                if let Some(exec) = pending_commands.get(&cmd_id) {
                    if exec.complete {
                        Some(exec.output.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                return Err(format!("Terminal {} not found", terminal_id));
            }
        }; // Drop the lock here before await

        if let Some(output) = is_complete {
            eprintln!("[grove-cmd] execute_in_terminal: command complete, {} bytes", output.len());

            // Clean up the completed command from pending_commands
            let terminals = state.terminals.lock();
            if let Some(session) = terminals.get(&terminal_id) {
                let mut pending_commands = session.pending_commands.lock().unwrap();
                pending_commands.remove(&cmd_id);
                eprintln!("[grove-cmd] execute_in_terminal: cleaned up command {}", cmd_id);
            }

            return Ok(output);
        }

        tokio::time::sleep(poll_interval).await;
    }
}

// ── Workspace commands ──

#[tauri::command]
pub fn add_workspace(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<Workspace, String> {
    let mut workspaces = state.workspaces.lock();
    let ws = workspaces
        .add_workspace(&path)
        .map_err(|e| format!("Failed to add workspace: {}", e))?;

    // Persist
    let _ = ws_state::save_workspaces(&workspaces.list_workspaces());

    // Start watchers for the new workspace
    restart_git_watcher(&app, &state, &workspaces);
    restart_claude_monitor(&app, &state, &workspaces);

    Ok(ws)
}

#[tauri::command]
pub fn discover_workspaces(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    repo_path: String,
) -> Result<Vec<Workspace>, String> {
    let mut workspaces = state.workspaces.lock();
    let added = workspaces
        .add_from_discovery(&repo_path)
        .map_err(|e| format!("Discovery failed: {}", e))?;

    // Persist
    let _ = ws_state::save_workspaces(&workspaces.list_workspaces());

    // Restart watchers with new paths
    restart_git_watcher(&app, &state, &workspaces);
    restart_claude_monitor(&app, &state, &workspaces);

    Ok(added)
}

#[tauri::command]
pub fn remove_workspace(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    workspace_id: String,
) -> Result<(), String> {
    let mut workspaces = state.workspaces.lock();
    workspaces
        .remove_workspace(&workspace_id)
        .ok_or_else(|| format!("Workspace {} not found", workspace_id))?;

    let _ = ws_state::save_workspaces(&workspaces.list_workspaces());

    restart_git_watcher(&app, &state, &workspaces);
    restart_claude_monitor(&app, &state, &workspaces);

    Ok(())
}

#[tauri::command]
pub fn list_workspaces(state: tauri::State<'_, Arc<AppState>>) -> Vec<Workspace> {
    state.workspaces.lock().list_workspaces()
}

#[tauri::command]
pub async fn scan_for_workspaces(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    cwd: Option<String>,
    terminal_id: Option<String>,
) -> Result<Vec<Workspace>, String> {
    use crate::workspace::discovery;

    eprintln!("[grove-cmd] scan_for_workspaces: cwd={:?}, terminal_id={:?}", cwd, terminal_id);

    // If terminal_id is provided, execute in terminal (for remote SSH sessions)
    let discovered = if let Some(tid) = terminal_id {
        eprintln!("[grove-cmd] scan_for_workspaces: executing in terminal {}", tid);

        // Execute git worktree list in the terminal
        let output = execute_in_terminal(
            state.clone(),
            tid,
            "git worktree list --porcelain".to_string()
        ).await?;

        eprintln!("[grove-cmd] scan_for_workspaces: got output {} bytes", output.len());

        // Parse the porcelain output
        parse_worktree_output(&output)
    } else {
        // Fall back to local execution
        let cwd_path = cwd.unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string())
        });

        eprintln!("[grove-cmd] scan_for_workspaces: running local git worktree list in: {}", cwd_path);

        match discovery::discover_worktrees(&PathBuf::from(&cwd_path)) {
            Ok(worktrees) => worktrees,
            Err(e) => {
                eprintln!("[grove-cmd] scan_for_workspaces: git worktree list failed: {}", e);
                Vec::new()
            }
        }
    };

    eprintln!("[grove-cmd] scan_for_workspaces: discovery found {} total worktrees", discovered.len());

    let mut workspaces = state.workspaces.lock();
    let existing_count = workspaces.list_workspaces().len();
    eprintln!("[grove-cmd] scan_for_workspaces: {} existing workspaces", existing_count);

    let added = workspaces.add_multiple_from_discovery(discovered);
    eprintln!("[grove-cmd] scan_for_workspaces: added {} new workspaces", added.len());

    if added.is_empty() {
        eprintln!("[grove-cmd] scan_for_workspaces: all discovered workspaces already tracked");
    }

    // Persist
    let _ = ws_state::save_workspaces(&workspaces.list_workspaces());

    // Restart watchers with new paths
    if !added.is_empty() {
        restart_git_watcher(&app, &state, &workspaces);
        restart_claude_monitor(&app, &state, &workspaces);
    }

    Ok(added)
}

fn parse_worktree_output(output: &str) -> Vec<crate::workspace::discovery::WorktreeInfo> {
    use std::path::PathBuf;

    let mut worktrees = Vec::new();
    let mut current_worktree: Option<crate::workspace::discovery::WorktreeInfo> = None;

    // Strip ANSI escape codes using a regex-free approach
    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut in_escape = false;
        let bytes = s.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                // Start of ANSI escape sequence
                in_escape = true;
                i += 2;
            } else if in_escape {
                // Skip until we find a letter (end of escape sequence)
                if bytes[i].is_ascii_alphabetic() {
                    in_escape = false;
                }
                i += 1;
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
        result
    }

    // Clean the output - remove ANSI escape codes and markers
    let clean_output = output
        .lines()
        .map(|line| strip_ansi(line))
        .filter(|line| {
            !line.contains("GROVE_EXEC_START_") &&
            !line.contains("GROVE_EXEC_END_") &&
            !line.trim().is_empty()
        })
        .collect::<Vec<String>>()
        .join("\n");

    for line in clean_output.lines() {
        let line = line.trim();

        if line.starts_with("worktree ") {
            // Save previous worktree
            if let Some(wt) = current_worktree.take() {
                worktrees.push(wt);
            }

            // Start new worktree
            let path = PathBuf::from(line.trim_start_matches("worktree "));
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "main".to_string());

            current_worktree = Some(crate::workspace::discovery::WorktreeInfo {
                name,
                path,
                branch: None,
                is_main: worktrees.is_empty(),
            });
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current_worktree {
                let branch_ref = line.trim_start_matches("branch ");
                let branch = branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string();
                wt.branch = Some(branch);
            }
        } else if line.starts_with("HEAD ") {
            if let Some(ref mut wt) = current_worktree {
                if wt.branch.is_none() {
                    let sha = line.trim_start_matches("HEAD ");
                    if sha.len() >= 7 {
                        wt.branch = Some(sha[..7].to_string());
                    }
                }
            }
        }
    }

    // Add last worktree
    if let Some(wt) = current_worktree {
        worktrees.push(wt);
    }

    eprintln!("[grove-cmd] parse_worktree_output: parsed {} worktrees", worktrees.len());
    worktrees
}

#[tauri::command]
pub fn get_git_status(
    state: tauri::State<'_, Arc<AppState>>,
    workspace_id: String,
) -> Result<git_status::GitStatus, String> {
    let workspaces = state.workspaces.lock();
    let ws = workspaces
        .get_workspace(&workspace_id)
        .ok_or_else(|| format!("Workspace {} not found", workspace_id))?;

    git_status::query_status(&workspace_id, &PathBuf::from(&ws.path))
        .map_err(|e| format!("Git status failed: {}", e))
}

#[tauri::command]
pub async fn get_remote_git_status(
    state: tauri::State<'_, Arc<AppState>>,
    workspace_id: String,
    terminal_id: String,
) -> Result<git_status::GitStatus, String> {
    let workspace_path = {
        let workspaces = state.workspaces.lock();
        let ws = workspaces
            .get_workspace(&workspace_id)
            .ok_or_else(|| format!("Workspace {} not found", workspace_id))?;
        ws.path.clone()
    };

    eprintln!("[grove-cmd] get_remote_git_status: workspace_id={}, path={}", workspace_id, workspace_path);

    // Execute git commands in the terminal to get status
    let commands = format!(
        "cd {} && git rev-parse --abbrev-ref HEAD 2>/dev/null; \
         git rev-parse --abbrev-ref @{{u}} 2>/dev/null; \
         git rev-list --left-right --count HEAD...@{{u}} 2>/dev/null; \
         git status --porcelain 2>/dev/null",
        workspace_path
    );

    let output = execute_in_terminal(state.clone(), terminal_id, commands).await?;

    eprintln!("[grove-cmd] get_remote_git_status: got output {} bytes", output.len());

    // Parse the output
    parse_git_status_output(&workspace_id, &output)
}

fn parse_git_status_output(workspace_id: &str, output: &str) -> Result<git_status::GitStatus, String> {
    // Filter out command lines and empty lines
    let lines: Vec<&str> = output.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() &&
            !trimmed.contains("git rev-parse") &&
            !trimmed.contains("git status") &&
            !trimmed.contains("git rev-list") &&
            !trimmed.contains("2>/dev/null") &&
            !trimmed.contains("&& cd") &&
            !trimmed.starts_with("cd ")
        })
        .collect();

    eprintln!("[grove-cmd] parse_git_status_output: {} lines after filtering", lines.len());

    // Debug: print first few lines
    for (i, line) in lines.iter().take(5).enumerate() {
        eprintln!("[grove-cmd] Line {}: {}", i, line);
    }

    let branch = lines.get(0).map(|s| s.trim().to_string());
    let remote_branch = lines.get(1).map(|s| s.trim().to_string());

    // Parse ahead/behind counts
    let (ahead, behind) = if let Some(counts_line) = lines.get(2) {
        let parts: Vec<&str> = counts_line.split_whitespace().collect();
        let ahead = parts.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let behind = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    };

    // Parse git status --porcelain output (starts from line 3)
    let mut untracked_count = 0;
    let mut staged_count = 0;
    let mut modified_count = 0;

    for line in lines.iter().skip(3) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Porcelain format: XY filename
        // X = index status, Y = working tree status
        if line.starts_with("??") {
            untracked_count += 1;
        } else if let Some(first_char) = line.chars().next() {
            if first_char != ' ' && first_char != '?' {
                staged_count += 1;
            }
            if line.len() >= 2 {
                if let Some(second_char) = line.chars().nth(1) {
                    if second_char != ' ' && second_char != '?' {
                        modified_count += 1;
                    }
                }
            }
        }
    }

    let dirty = untracked_count > 0 || staged_count > 0 || modified_count > 0;

    eprintln!(
        "[grove-cmd] parse_git_status_output: branch={:?}, dirty={}, staged={}, modified={}, untracked={}",
        branch, dirty, staged_count, modified_count, untracked_count
    );

    Ok(git_status::GitStatus {
        workspace_id: workspace_id.to_string(),
        branch,
        remote_branch,
        ahead,
        behind,
        dirty,
        untracked_count,
        staged_count,
        modified_count,
    })
}

#[tauri::command]
pub fn start_git_watching(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let workspaces = state.workspaces.lock();
    restart_git_watcher(&app, &state, &workspaces);
    Ok(())
}

// ── Claude Code commands ──

#[tauri::command]
pub fn start_claude_monitoring(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let workspaces = state.workspaces.lock();
    restart_claude_monitor(&app, &state, &workspaces);
    Ok(())
}

// ── Helpers ──

fn restart_claude_monitor(
    app: &AppHandle,
    state: &tauri::State<'_, Arc<AppState>>,
    workspaces: &crate::workspace::manager::WorkspaceManager,
) {
    let mut monitor_slot = state.claude_monitor.lock();

    if let Some(old_monitor) = monitor_slot.take() {
        old_monitor.stop();
    }

    let paths = workspaces.workspace_paths();
    if let Ok(handle) = claude_monitor::start_monitoring(app.clone(), paths) {
        *monitor_slot = Some(handle);
    }
}

fn restart_git_watcher(
    app: &AppHandle,
    state: &tauri::State<'_, Arc<AppState>>,
    workspaces: &crate::workspace::manager::WorkspaceManager,
) {
    let mut watcher_slot = state.git_watcher.lock();

    // Stop existing watcher
    if let Some(old_watcher) = watcher_slot.take() {
        old_watcher.stop();
    }

    // Start new watcher with current workspace paths
    let paths = workspaces.workspace_paths();
    if !paths.is_empty() {
        if let Ok(handle) = watcher::start_watching(app.clone(), paths) {
            *watcher_slot = Some(handle);
        }
    }
}
