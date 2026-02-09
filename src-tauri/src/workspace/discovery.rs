use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Discover git repos starting from a given path.
/// If the path is inside a git repo, discovers all worktrees for that repo.
/// Also scans sibling directories in the parent.
pub fn discover_from_path(cwd: &Path) -> Vec<WorktreeInfo> {
    eprintln!("[grove-discovery] discover_from_path: {}", cwd.display());

    let mut all_worktrees = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    // First, check if cwd is inside a git repo
    if let Ok(repo) = git2::Repository::discover(cwd) {
        if let Some(workdir) = repo.workdir() {
            eprintln!("[grove-discovery] Found current repo at: {}", workdir.display());
            if let Ok(worktrees) = discover_worktrees(workdir) {
                eprintln!("[grove-discovery] Discovered {} worktree(s) for current repo", worktrees.len());
                for wt in worktrees {
                    if seen_paths.insert(wt.path.clone()) {
                        all_worktrees.push(wt);
                    }
                }
            }
        }
    }

    // Also scan sibling directories in the parent
    let scan_dir = if cwd.is_dir() {
        cwd.parent().unwrap_or(cwd)
    } else {
        cwd.parent().unwrap_or(cwd).parent().unwrap_or(cwd)
    };

    eprintln!("[grove-discovery] Also scanning parent directory: {}", scan_dir.display());

    if scan_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(scan_dir) {
            for entry in entries.flatten() {
                if let Ok(path) = entry.path().canonicalize() {
                    // Check if it's a git repo
                    if path.join(".git").exists() {
                        eprintln!("[grove-discovery] Found sibling git repo: {}", path.display());
                        if let Ok(worktrees) = discover_worktrees(&path) {
                            eprintln!("[grove-discovery] Discovered {} worktree(s)", worktrees.len());
                            for wt in worktrees {
                                if seen_paths.insert(wt.path.clone()) {
                                    all_worktrees.push(wt);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    eprintln!("[grove-discovery] Total unique worktrees found: {}", all_worktrees.len());
    all_worktrees
}

/// Discover all git worktrees associated with a repository using `git worktree list`.
/// This is much faster than using git2-rs API and matches what tools like `cw` do.
pub fn discover_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    use std::process::Command;

    eprintln!("[grove-discovery] Running 'git worktree list' in {}", repo_path.display());

    let output = Command::new("git")
        .args(&["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("Failed to run git worktree list in {}", repo_path.display()))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("git worktree list failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_worktree: Option<WorktreeInfo> = None;

    for line in stdout.lines() {
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

            current_worktree = Some(WorktreeInfo {
                name,
                path,
                branch: None,
                is_main: worktrees.is_empty(), // First worktree is the main one
            });
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current_worktree {
                let branch_ref = line.trim_start_matches("branch ");
                // Extract branch name from refs/heads/branch-name
                let branch = branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string();
                wt.branch = Some(branch);
            }
        } else if line.starts_with("HEAD ") {
            // Detached HEAD - get short SHA
            if let Some(ref mut wt) = current_worktree {
                if wt.branch.is_none() {
                    let sha = line.trim_start_matches("HEAD ");
                    wt.branch = Some(format!("{:.7}", sha));
                }
            }
        }
    }

    // Add last worktree
    if let Some(wt) = current_worktree {
        worktrees.push(wt);
    }

    eprintln!("[grove-discovery] git worktree list found {} worktree(s)", worktrees.len());
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

/// Recursively scan a directory for git repos (up to max_depth levels)
fn scan_recursive(dir: &Path, max_depth: usize, current_depth: usize, seen_paths: &mut std::collections::HashSet<PathBuf>) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();

    if current_depth > max_depth {
        return worktrees;
    }

    if !dir.exists() {
        return worktrees;
    }

    // Check if this directory itself is a git repo
    if dir.join(".git").exists() {
        if let Ok(path) = dir.canonicalize() {
            eprintln!("[grove-discovery] Found git repo: {}", path.display());
            if let Ok(discovered) = discover_worktrees(&path) {
                eprintln!("[grove-discovery] Discovered {} worktree(s)", discovered.len());
                for wt in discovered {
                    if seen_paths.insert(wt.path.clone()) {
                        worktrees.push(wt);
                    }
                }
            }
        }
        // Don't recurse into git repos
        return worktrees;
    }

    // Scan subdirectories
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir = entry.path();
                    worktrees.extend(scan_recursive(&subdir, max_depth, current_depth + 1, seen_paths));
                }
            }
        }
    }

    worktrees
}

/// Discover git repos in common project directories
pub fn discover_common_repos() -> Vec<WorktreeInfo> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("[grove-discovery] No home directory found");
            return Vec::new();
        }
    };

    let common_dirs = vec![
        (home.join("projects"), 1),  // (path, max_depth)
        (home.join("dev"), 1),
        (home.join("Development"), 1),
        (home.join("Code"), 1),
        (home.join("src"), 1),
        (home.join("workspace"), 1),
        (home.join("carrot"), 2),  // Scan carrot deeper to find customers/*/
    ];

    eprintln!("[grove-discovery] Scanning {} common directories", common_dirs.len());

    let mut all_worktrees = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for (parent_dir, max_depth) in common_dirs {
        if !parent_dir.exists() {
            eprintln!("[grove-discovery] Skipping (doesn't exist): {}", parent_dir.display());
            continue;
        }

        eprintln!("[grove-discovery] Scanning: {} (depth={})", parent_dir.display(), max_depth);
        all_worktrees.extend(scan_recursive(&parent_dir, max_depth, 0, &mut seen_paths));
    }

    eprintln!("[grove-discovery] Total unique worktrees found: {}", all_worktrees.len());
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
