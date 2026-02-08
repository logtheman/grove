import { useState } from "react";
import { ChevronLeft, ChevronRight, GitBranch } from "lucide-react";
import { ClaudeSessionWidget } from "./ClaudeSessionWidget";
import type { GitStatus, ClaudeSession } from "@/types";

interface DashboardPanelProps {
  gitStatus: GitStatus | undefined;
  claudeSessions: ClaudeSession[];
}

export function DashboardPanel({
  gitStatus,
  claudeSessions,
}: DashboardPanelProps) {
  const [collapsed, setCollapsed] = useState(false);

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

      {/* Git Status */}
      {gitStatus && (
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

      {/* Claude Sessions */}
      <div className="border-b border-grove-border">
        <ClaudeSessionWidget sessions={claudeSessions} />
      </div>
    </div>
  );
}
