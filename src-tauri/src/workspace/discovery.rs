use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Discover all git worktrees associated with a repository.
/// Given any path inside a git repo, finds the main worktree and all linked worktrees.
pub fn discover_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let repo = git2::Repository::discover(repo_path)
        .with_context(|| format!("No git repo found at {}", repo_path.display()))?;

    let mut worktrees = Vec::new();

    // Add the main worktree
    if let Some(workdir) = repo.workdir() {
        let head_branch = get_head_branch(&repo);
        worktrees.push(WorktreeInfo {
            name: workdir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "main".to_string()),
            path: workdir.to_path_buf(),
            branch: head_branch,
            is_main: true,
        });
    }

    // Discover linked worktrees
    if let Ok(wt_names) = repo.worktrees() {
        for i in 0..wt_names.len() {
            if let Some(name) = wt_names.get(i) {
                if let Ok(wt) = repo.find_worktree(name) {
                    if wt.validate().is_ok() {
                        let wt_path = wt.path().to_path_buf();
                        // Open the worktree as a repo to get its branch
                        let branch = git2::Repository::open(&wt_path)
                            .ok()
                            .and_then(|r| get_head_branch(&r));

                        worktrees.push(WorktreeInfo {
                            name: name.to_string(),
                            path: wt_path,
                            branch,
                            is_main: false,
                        });
                    }
                }
            }
        }
    }

    Ok(worktrees)
}

/// Scan a list of directories for git repositories and their worktrees.
pub fn scan_directories(dirs: &[PathBuf]) -> Vec<WorktreeInfo> {
    let mut all_worktrees = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        if let Ok(worktrees) = discover_worktrees(dir) {
            for wt in worktrees {
                if seen_paths.insert(wt.path.clone()) {
                    all_worktrees.push(wt);
                }
            }
        }
    }

    all_worktrees
}

fn get_head_branch(repo: &git2::Repository) -> Option<String> {
    repo.head().ok().and_then(|head| {
        if head.is_branch() {
            head.shorthand().map(|s| s.to_string())
        } else {
            // Detached HEAD - show short hash
            head.target()
                .map(|oid| format!("{:.7}", oid.to_string()))
        }
    })
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_main: bool,
}
