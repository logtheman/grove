use crate::workspace::manager::Workspace;
use anyhow::Result;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".grove")
}

fn workspaces_file() -> PathBuf {
    config_dir().join("workspaces.json")
}

/// Save workspaces to ~/.grove/workspaces.json
pub fn save_workspaces(workspaces: &[Workspace]) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;

    let json = serde_json::to_string_pretty(workspaces)?;
    std::fs::write(workspaces_file(), json)?;

    Ok(())
}

/// Load workspaces from ~/.grove/workspaces.json
pub fn load_workspaces() -> Vec<Workspace> {
    let path = workspaces_file();
    if !path.exists() {
        return Vec::new();
    }

    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<Workspace>>(&s).ok())
        .unwrap_or_default()
}
