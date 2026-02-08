use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub workspace_id: String,
    pub branch: Option<String>,
    pub remote_branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub untracked_count: u32,
    pub staged_count: u32,
    pub modified_count: u32,
}

/// Query the full git status for a repository at the given path.
pub fn query_status(workspace_id: &str, repo_path: &Path) -> Result<GitStatus> {
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("Failed to open repo at {}", repo_path.display()))?;

    let mut status = GitStatus {
        workspace_id: workspace_id.to_string(),
        ..Default::default()
    };

    // Branch info
    if let Ok(head) = repo.head() {
        if head.is_branch() {
            status.branch = head.shorthand().map(|s| s.to_string());

            // Find upstream and compute ahead/behind
            if let Some(branch_name) = status.branch.as_ref() {
                if let Ok(local_branch) =
                    repo.find_branch(branch_name, git2::BranchType::Local)
                {
                    if let Ok(upstream) = local_branch.upstream() {
                        status.remote_branch =
                            upstream.name().ok().flatten().map(|s| s.to_string());

                        if let (Some(local_oid), Some(upstream_oid)) =
                            (head.target(), upstream.get().target())
                        {
                            if let Ok((ahead, behind)) =
                                repo.graph_ahead_behind(local_oid, upstream_oid)
                            {
                                status.ahead = ahead as u32;
                                status.behind = behind as u32;
                            }
                        }
                    }
                }
            }
        } else {
            // Detached HEAD
            status.branch = head
                .target()
                .map(|oid| format!("{:.7}", oid.to_string()));
        }
    }

    // File status counts
    let opts = &mut git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false) // Don't recurse into untracked dirs (performance)
        .exclude_submodules(true);

    if let Ok(statuses) = repo.statuses(Some(opts)) {
        for entry in statuses.iter() {
            let s = entry.status();

            if s.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            ) {
                status.staged_count += 1;
            }

            if s.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_TYPECHANGE
                    | git2::Status::WT_RENAMED,
            ) {
                status.modified_count += 1;
            }

            if s.contains(git2::Status::WT_NEW) {
                status.untracked_count += 1;
            }
        }

        status.dirty = status.staged_count > 0
            || status.modified_count > 0
            || status.untracked_count > 0;
    }

    Ok(status)
}
