use crate::pty::manager::PtySession;
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct AppState {
    pub terminals: Mutex<HashMap<String, PtySession>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            terminals: Mutex::new(HashMap::new()),
        }
    }
}
