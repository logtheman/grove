use crate::git::watcher::GitWatcherHandle;
use crate::pty::manager::PtySession;
use crate::workspace::manager::WorkspaceManager;
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct AppState {
    pub terminals: Mutex<HashMap<String, PtySession>>,
    pub workspaces: Mutex<WorkspaceManager>,
    pub git_watcher: Mutex<Option<GitWatcherHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut wm = WorkspaceManager::new();

        // Load persisted workspaces
        let saved = crate::workspace::state::load_workspaces();
        wm.load_from(saved);

        Self {
            terminals: Mutex::new(HashMap::new()),
            workspaces: Mutex::new(wm),
            git_watcher: Mutex::new(None),
        }
    }
}
