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

    let event_id = terminal_id.clone();
    let app_handle = app.clone();

    let session = PtySession::spawn(&working_dir, 24, 80, move |data| {
        let event_name = format!("{}:{}", events::TERMINAL_DATA, event_id);
        let _ = app_handle.emit(&event_name, data);
    })
    .map_err(|e| format!("Failed to create terminal: {}", e))?;

    let info = TerminalInfo {
        id: terminal_id.clone(),
        cwd: working_dir,
    };

    state.terminals.lock().insert(terminal_id, session);

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
            .map_err(|e| format!("Write failed: {}", e))
    } else {
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
