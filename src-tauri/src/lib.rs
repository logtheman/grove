mod claude;
mod git;
mod ipc;
mod pty;
mod state;
mod workspace;

use state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Terminal
            ipc::commands::create_terminal,
            ipc::commands::close_terminal,
            ipc::commands::write_to_terminal,
            ipc::commands::resize_terminal,
            ipc::commands::list_terminals,
            // Debug
            ipc::commands::write_debug_log,
            // Workspace
            ipc::commands::add_workspace,
            ipc::commands::discover_workspaces,
            ipc::commands::remove_workspace,
            ipc::commands::list_workspaces,
            ipc::commands::scan_for_workspaces,
            // Git
            ipc::commands::get_git_status,
            ipc::commands::start_git_watching,
            // Claude Code
            ipc::commands::start_claude_monitoring,
        ])
        .run(tauri::generate_context!())
        .expect("error while running grove");
}
