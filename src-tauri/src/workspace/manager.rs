use crate::git::status as git_status;
use crate::workspace::discovery;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub git_branch: Option<String>,
    pub is_main_worktree: bool,
}

pub struct WorkspaceManager {
    workspaces: HashMap<String, Workspace>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            workspaces: HashMap::new(),
        }
    }

    /// Add a workspace by path. Auto-detects git info.
    pub fn add_workspace(&mut self, path: &str) -> Result<Workspace> {
        let path_buf = PathBuf::from(path);

        // Check if already tracked
        for ws in self.workspaces.values() {
            if ws.path == path {
                return Ok(ws.clone());
            }
        }

        let id = uuid::Uuid::new_v4().to_string();

        let name = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        let git_branch = git2::Repository::open(&path_buf)
            .ok()
            .and_then(|repo| {
                repo.head().ok().and_then(|head| {
                    head.shorthand().map(|s| s.to_string())
                })
            });

        let workspace = Workspace {
            id: id.clone(),
            name,
            path: path.to_string(),
            git_branch,
            is_main_worktree: true, // Will be updated by discovery
        };

        self.workspaces.insert(id, workspace.clone());
        Ok(workspace)
    }

    /// Discover and add all worktrees from a repo path.
    pub fn add_from_discovery(&mut self, repo_path: &str) -> Result<Vec<Workspace>> {
        let worktrees = discovery::discover_worktrees(&PathBuf::from(repo_path))?;
        let mut added = Vec::new();

        for wt in worktrees {
            let path_str = wt.path.to_string_lossy().to_string();

            // Skip if already tracked
            if self.workspaces.values().any(|ws| ws.path == path_str) {
                if let Some(existing) = self.workspaces.values().find(|ws| ws.path == path_str) {
                    added.push(existing.clone());
                }
                continue;
            }

            let id = uuid::Uuid::new_v4().to_string();
            let workspace = Workspace {
                id: id.clone(),
                name: wt.name,
                path: path_str,
                git_branch: wt.branch,
                is_main_worktree: wt.is_main,
            };

            self.workspaces.insert(id, workspace.clone());
            added.push(workspace);
        }

        Ok(added)
    }

    /// Remove a workspace by ID.
    pub fn remove_workspace(&mut self, id: &str) -> Option<Workspace> {
        self.workspaces.remove(id)
    }

    /// List all workspaces.
    pub fn list_workspaces(&self) -> Vec<Workspace> {
        let mut workspaces: Vec<_> = self.workspaces.values().cloned().collect();
        workspaces.sort_by(|a, b| a.name.cmp(&b.name));
        workspaces
    }

    /// Get a workspace by ID.
    pub fn get_workspace(&self, id: &str) -> Option<&Workspace> {
        self.workspaces.get(id)
    }

    /// Update the git branch for a workspace.
    pub fn update_branch(&mut self, id: &str, branch: Option<String>) {
        if let Some(ws) = self.workspaces.get_mut(id) {
            ws.git_branch = branch;
        }
    }

    /// Get all workspace paths with IDs (for git watcher).
    pub fn workspace_paths(&self) -> Vec<(String, PathBuf)> {
        self.workspaces
            .values()
            .map(|ws| (ws.id.clone(), PathBuf::from(&ws.path)))
            .collect()
    }

    /// Load workspaces from persisted state.
    pub fn load_from(&mut self, workspaces: Vec<Workspace>) {
        for ws in workspaces {
            self.workspaces.insert(ws.id.clone(), ws);
        }
    }
}
