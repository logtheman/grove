use crate::git::status::{self, GitStatus};
use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const DEBOUNCE_MS: u64 = 500;

pub struct GitWatcherHandle {
    _watcher: RecommendedWatcher,
    shutdown_tx: std::sync::mpsc::Sender<()>,
}

impl GitWatcherHandle {
    pub fn stop(self) {
        let _ = self.shutdown_tx.send(());
    }
}

struct WatchedRepo {
    workspace_id: String,
    repo_path: PathBuf,
    last_check: Instant,
    pending: bool,
}

/// Start watching git status for multiple workspaces.
/// Emits "git-status-updated:{workspace_id}" events when status changes.
pub fn start_watching(
    app: AppHandle,
    workspaces: Vec<(String, PathBuf)>, // (workspace_id, repo_path)
) -> Result<GitWatcherHandle> {
    let (fs_tx, fs_rx) = mpsc::channel::<Event>();
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = fs_tx.send(event);
            }
        },
        Config::default(),
    )?;

    // Map git dirs to workspace info
    let mut watched: HashMap<PathBuf, WatchedRepo> = HashMap::new();

    for (workspace_id, repo_path) in &workspaces {
        let git_dir = repo_path.join(".git");
        let watch_path = if git_dir.is_dir() {
            git_dir.clone()
        } else if git_dir.is_file() {
            // Worktree: .git is a file pointing to the actual git dir
            // Watch the repo path instead
            repo_path.clone()
        } else {
            continue;
        };

        if watcher.watch(&watch_path, RecursiveMode::Recursive).is_ok() {
            watched.insert(
                watch_path,
                WatchedRepo {
                    workspace_id: workspace_id.clone(),
                    repo_path: repo_path.clone(),
                    last_check: Instant::now() - Duration::from_secs(60), // Force initial check
                    pending: true, // Check immediately on start
                },
            );
        }
    }

    // Debounce + emit thread
    std::thread::spawn(move || {
        loop {
            // Check for shutdown
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            // Drain filesystem events and mark repos as pending
            while let Ok(event) = fs_rx.try_recv() {
                for path in &event.paths {
                    // Find which watched repo this path belongs to
                    for (watch_path, repo) in watched.iter_mut() {
                        if path.starts_with(watch_path) {
                            repo.pending = true;
                            break;
                        }
                    }
                }
            }

            // Process pending repos that have passed the debounce window
            let now = Instant::now();
            for repo in watched.values_mut() {
                if repo.pending
                    && now.duration_since(repo.last_check) >= Duration::from_millis(DEBOUNCE_MS)
                {
                    repo.pending = false;
                    repo.last_check = now;

                    if let Ok(git_status) =
                        status::query_status(&repo.workspace_id, &repo.repo_path)
                    {
                        let event_name =
                            format!("git-status-updated:{}", repo.workspace_id);
                        let _ = app.emit(&event_name, &git_status);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    });

    Ok(GitWatcherHandle {
        _watcher: watcher,
        shutdown_tx,
    })
}
