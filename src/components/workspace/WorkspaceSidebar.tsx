import { useState } from "react";
import {
  GitBranch,
  Circle,
  FolderGit2,
  Plus,
  Trash2,
  Search,
} from "lucide-react";
import type { Workspace, GitStatus } from "@/types";

interface WorkspaceSidebarProps {
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  gitStatuses: Record<string, GitStatus>;
  onSelectWorkspace: (id: string) => void;
  onDiscoverWorkspaces: (path: string) => void;
  onRemoveWorkspace: (id: string) => void;
  onScanForWorkspaces: () => void;
}

export function WorkspaceSidebar({
  workspaces,
  activeWorkspaceId,
  gitStatuses,
  onSelectWorkspace,
  onDiscoverWorkspaces,
  onRemoveWorkspace,
  onScanForWorkspaces,
}: WorkspaceSidebarProps) {
  const [showAddInput, setShowAddInput] = useState(false);
  const [inputValue, setInputValue] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputValue.trim()) return;

    // If path looks like a repo, discover worktrees; otherwise add single workspace
    onDiscoverWorkspaces(inputValue.trim());
    setInputValue("");
    setShowAddInput(false);
  };

  return (
    <div className="flex flex-col h-full bg-grove-bg border-r border-grove-border w-56 select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-3 h-9 border-b border-grove-border">
        <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
          Workspaces
        </span>
        <div className="flex items-center gap-1">
          <button
            className="p-1 hover:bg-grove-surface rounded text-grove-text-muted hover:text-grove-text"
            onClick={async () => {
              try {
                await onScanForWorkspaces();
              } catch (error) {
                console.error("[WorkspaceSidebar] Scan failed:", error);
              }
            }}
            title="Scan for workspaces in ~/projects, ~/dev, etc."
          >
            <Search size={12} />
          </button>
          <button
            className="p-1 hover:bg-grove-surface rounded text-grove-text-muted hover:text-grove-text"
            onClick={() => setShowAddInput(!showAddInput)}
            title="Add workspace by path"
          >
            <Plus size={14} />
          </button>
        </div>
      </div>

      {/* Add workspace input */}
      {showAddInput && (
        <form onSubmit={handleSubmit} className="px-2 py-2 border-b border-grove-border">
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder="/path/to/repo"
            className="w-full px-2 py-1 text-xs bg-grove-surface border border-grove-border rounded text-grove-text placeholder-grove-text-muted focus:outline-none focus:border-grove-accent"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setShowAddInput(false);
                setInputValue("");
              }
            }}
          />
        </form>
      )}

      {/* Workspace list */}
      <div className="flex-1 overflow-y-auto py-1">
        {workspaces.length === 0 && (
          <div className="px-3 py-4 text-xs text-grove-text-muted text-center">
            No workspaces yet.
            <br />
            Click + to add a repo path.
          </div>
        )}

        {workspaces.map((ws) => {
          const status = gitStatuses[ws.id];
          const isActive = ws.id === activeWorkspaceId;

          return (
            <WorkspaceItem
              key={ws.id}
              workspace={ws}
              status={status}
              isActive={isActive}
              onSelect={() => onSelectWorkspace(ws.id)}
              onRemove={() => onRemoveWorkspace(ws.id)}
            />
          );
        })}
      </div>
    </div>
  );
}

function WorkspaceItem({
  workspace,
  status,
  isActive,
  onSelect,
  onRemove,
}: {
  workspace: Workspace;
  status?: GitStatus;
  isActive: boolean;
  onSelect: () => void;
  onRemove: () => void;
}) {
  const dirty = status?.dirty ?? false;
  const branch = status?.branch ?? workspace.gitBranch ?? "—";
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;

  return (
    <div
      className={`
        group flex items-start gap-2 px-3 py-1.5 cursor-pointer
        ${isActive ? "bg-grove-surface" : "hover:bg-grove-surface/50"}
      `}
      onClick={onSelect}
    >
      {/* Status indicator */}
      <div className="mt-0.5">
        {dirty ? (
          <Circle size={8} className="fill-grove-warning text-grove-warning" />
        ) : (
          <Circle size={8} className="fill-grove-success text-grove-success" />
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1">
          <FolderGit2 size={12} className="text-grove-text-muted flex-shrink-0" />
          <span className="text-sm truncate text-grove-text">{workspace.name}</span>
        </div>
        <div className="flex items-center gap-1 mt-0.5">
          <GitBranch size={10} className="text-grove-text-muted flex-shrink-0" />
          <span className="text-xs text-grove-text-muted truncate">{branch}</span>
          {ahead > 0 && (
            <span className="text-xs text-grove-success">+{ahead}</span>
          )}
          {behind > 0 && (
            <span className="text-xs text-grove-error">-{behind}</span>
          )}
        </div>
      </div>

      {/* Remove button */}
      <button
        className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-grove-border rounded text-grove-text-muted"
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        title="Remove workspace"
      >
        <Trash2 size={10} />
      </button>
    </div>
  );
}
