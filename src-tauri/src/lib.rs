mod ipc;
mod pty;
mod state;

use state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            ipc::commands::create_terminal,
            ipc::commands::close_terminal,
            ipc::commands::write_to_terminal,
            ipc::commands::resize_terminal,
            ipc::commands::list_terminals,
        ])
        .run(tauri::generate_context!())
        .expect("error while running grove");
}
