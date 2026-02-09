import { useState, useEffect } from "react";
import { ChevronLeft, ChevronRight, GitBranch, FolderGit2, MapPin, Terminal, RefreshCw } from "lucide-react";
import { ClaudeSessionWidget } from "./ClaudeSessionWidget";
import { writeToTerminal, getRemoteGitStatus } from "@/services/tauri";
import type { GitStatus, ClaudeSession, Workspace } from "@/types";

interface DashboardPanelProps {
  workspace: Workspace | undefined;
  gitStatus: GitStatus | undefined;
  claudeSessions: ClaudeSession[];
  activeTerminalId: string | null;
  onGitStatusUpdate: (workspaceId: string, status: GitStatus) => void;
}

export function DashboardPanel({
  workspace,
  gitStatus,
  claudeSessions,
  activeTerminalId,
  onGitStatusUpdate,
}: DashboardPanelProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [isRemote, setIsRemote] = useState(false);
  const [loadingStatus, setLoadingStatus] = useState(false);

  // Detect if workspace is remote (path starts with / but likely doesn't exist locally)
  // A better heuristic: if path doesn't start with /Users/ or /home/<username>/ on mac, it's likely remote
  useEffect(() => {
    if (!workspace) {
      setIsRemote(false);
      return;
    }

    const path = workspace.path;
    const isLikelyRemote = path.startsWith("/") && !path.startsWith("/Users/");
    setIsRemote(isLikelyRemote);

    // Auto-fetch git status for remote workspaces when selected
    if (isLikelyRemote && activeTerminalId && !gitStatus) {
      handleFetchRemoteGitStatus();
    }
  }, [workspace?.id, activeTerminalId]);

  const handleFetchRemoteGitStatus = async () => {
    if (!workspace || !activeTerminalId) return;

    setLoadingStatus(true);
    try {
      const status = await getRemoteGitStatus(workspace.id, activeTerminalId);
      onGitStatusUpdate(workspace.id, status);
      console.log(`[DashboardPanel] Fetched remote git status for: ${workspace.path}`);
    } catch (error) {
      console.error("[DashboardPanel] Failed to fetch remote git status:", error);
    } finally {
      setLoadingStatus(false);
    }
  };

  const handleSwitchToWorkspace = async () => {
    if (!workspace || !activeTerminalId) return;

    try {
      const command = `cd ${workspace.path}\n`;
      const encoder = new TextEncoder();
      const data = encoder.encode(command);
      await writeToTerminal(activeTerminalId, data);
      console.log(`[DashboardPanel] Switched terminal to: ${workspace.path}`);
    } catch (error) {
      console.error("[DashboardPanel] Failed to switch workspace:", error);
    }
  };

  if (collapsed) {
    return (
      <div className="flex flex-col items-center w-8 bg-grove-bg border-l border-grove-border">
        <button
          className="p-1 mt-2 hover:bg-grove-surface rounded text-grove-text-muted"
          onClick={() => setCollapsed(false)}
          title="Expand dashboard"
        >
          <ChevronLeft size={14} />
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col w-56 bg-grove-bg border-l border-grove-border overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-3 h-9 border-b border-grove-border">
        <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
          Dashboard
        </span>
        <button
          className="p-1 hover:bg-grove-surface rounded text-grove-text-muted"
          onClick={() => setCollapsed(true)}
          title="Collapse dashboard"
        >
          <ChevronRight size={12} />
        </button>
      </div>

      {/* Workspace Info */}
      {workspace ? (
        <div className="px-3 py-2 border-b border-grove-border">
          <div className="flex items-center gap-2 mb-2">
            <FolderGit2 size={14} className="text-grove-text-muted" />
            <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
              Workspace
            </span>
          </div>

          <div className="space-y-1 text-xs">
            <div>
              <div className="text-grove-text-muted mb-0.5">Name</div>
              <div className="text-grove-text font-medium">{workspace.name}</div>
            </div>

            <div>
              <div className="text-grove-text-muted mb-0.5 flex items-center gap-1">
                <MapPin size={10} />
                <span>Path</span>
              </div>
              <div className="text-grove-text text-[10px] font-mono break-all">
                {workspace.path}
              </div>
            </div>

            {workspace.gitBranch && (
              <div className="flex justify-between items-center">
                <span className="text-grove-text-muted">Branch</span>
                <span className="text-grove-accent">{workspace.gitBranch}</span>
              </div>
            )}

            <div className="flex justify-between items-center">
              <span className="text-grove-text-muted">Type</span>
              <span className="text-grove-text-muted text-[10px]">
                {workspace.isMainWorktree ? "Main worktree" : "Linked worktree"}
              </span>
            </div>

            {/* Switch to workspace button */}
            <div className="mt-3 pt-2 border-t border-grove-border">
              <button
                onClick={handleSwitchToWorkspace}
                disabled={!activeTerminalId}
                className={`
                  w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded text-xs
                  ${activeTerminalId
                    ? "bg-grove-accent text-white hover:bg-grove-accent/80"
                    : "bg-grove-surface text-grove-text-muted cursor-not-allowed"
                  }
                `}
                title={activeTerminalId ? "Switch terminal to this workspace" : "No active terminal"}
              >
                <Terminal size={12} />
                <span>Switch to Workspace</span>
              </button>
            </div>
          </div>
        </div>
      ) : (
        <div className="px-3 py-4 text-xs text-grove-text-muted text-center">
          Select a workspace to view details
        </div>
      )}

      {/* Git Status */}
      {workspace && gitStatus && (
        <div className="px-3 py-2 border-b border-grove-border">
          <div className="flex items-center gap-2 mb-2">
            <GitBranch size={14} className="text-grove-text-muted" />
            <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
              Git Status
            </span>
          </div>

          <div className="space-y-1 text-xs">
            <div className="flex justify-between">
              <span className="text-grove-text-muted">Branch</span>
              <span className="text-grove-text">{gitStatus.branch ?? "—"}</span>
            </div>
            {gitStatus.remoteBranch && (
              <div className="flex justify-between">
                <span className="text-grove-text-muted">Remote</span>
                <span className="text-grove-text">{gitStatus.remoteBranch}</span>
              </div>
            )}
            <div className="flex justify-between">
              <span className="text-grove-text-muted">Status</span>
              <span
                className={gitStatus.dirty ? "text-grove-warning" : "text-grove-success"}
              >
                {gitStatus.dirty ? "dirty" : "clean"}
              </span>
            </div>
            {(gitStatus.ahead > 0 || gitStatus.behind > 0) && (
              <div className="flex justify-between">
                <span className="text-grove-text-muted">Sync</span>
                <span className="text-grove-text">
                  {gitStatus.ahead > 0 && (
                    <span className="text-grove-success">+{gitStatus.ahead}</span>
                  )}
                  {gitStatus.ahead > 0 && gitStatus.behind > 0 && " "}
                  {gitStatus.behind > 0 && (
                    <span className="text-grove-error">-{gitStatus.behind}</span>
                  )}
                </span>
              </div>
            )}
            {gitStatus.stagedCount > 0 && (
              <div className="flex justify-between">
                <span className="text-grove-text-muted">Staged</span>
                <span className="text-grove-accent">{gitStatus.stagedCount}</span>
              </div>
            )}
            {gitStatus.modifiedCount > 0 && (
              <div className="flex justify-between">
                <span className="text-grove-text-muted">Modified</span>
                <span className="text-grove-warning">{gitStatus.modifiedCount}</span>
              </div>
            )}
            {gitStatus.untrackedCount > 0 && (
              <div className="flex justify-between">
                <span className="text-grove-text-muted">Untracked</span>
                <span className="text-grove-text-muted">
                  {gitStatus.untrackedCount}
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Git Status Not Available or Remote Fetch */}
      {workspace && !gitStatus && (
        <div className="px-3 py-2 border-b border-grove-border">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <GitBranch size={14} className="text-grove-text-muted" />
              <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
                Git Status
              </span>
            </div>
            {isRemote && activeTerminalId && (
              <button
                onClick={handleFetchRemoteGitStatus}
                disabled={loadingStatus}
                className="p-1 hover:bg-grove-surface rounded text-grove-text-muted hover:text-grove-text disabled:opacity-50"
                title="Fetch git status from remote"
              >
                <RefreshCw size={12} className={loadingStatus ? "animate-spin" : ""} />
              </button>
            )}
          </div>
          <div className="text-xs">
            {loadingStatus ? (
              <div className="text-grove-text-muted">Loading git status...</div>
            ) : isRemote && activeTerminalId ? (
              <div className="text-grove-text-muted">
                Click refresh to fetch git status from remote
              </div>
            ) : (
              <div className="text-grove-text-muted">
                {isRemote ? "Connect to a terminal to fetch git status" : "Git status not available"}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Claude Sessions */}
      <div className="border-b border-grove-border">
        <ClaudeSessionWidget sessions={claudeSessions} />
      </div>
    </div>
  );
}
