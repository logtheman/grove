use crate::ipc::events;
use crate::pty::manager::PtySession;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

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
