import { useEffect, useCallback } from "react";
import { useAppStore } from "@/stores/appStore";
import {
  listWorkspaces,
  addWorkspace,
  discoverWorkspaces,
  removeWorkspace,
  getGitStatus,
  startGitWatching,
  onGitStatusUpdated,
} from "@/services/tauri";
import type { GitStatus } from "@/types";

export function useWorkspaces() {
  const {
    workspaces,
    activeWorkspaceId,
    gitStatuses,
    setWorkspaces,
    addWorkspaces,
    removeWorkspaceFromStore,
    setActiveWorkspace,
    updateGitStatus,
  } = useAppStore();

  // Load workspaces on mount, auto-scan if empty
  useEffect(() => {
    const loadAndScan = async () => {
      try {
        const loaded = await listWorkspaces();
        setWorkspaces(loaded);

        // Auto-scan if no workspaces found
        if (loaded.length === 0) {
          console.log("[useWorkspaces] No workspaces found, running auto-scan...");
          const { scanForWorkspaces } = await import("@/services/tauri");
          const discovered = await scanForWorkspaces();
          if (discovered.length > 0) {
            console.log(`[useWorkspaces] Auto-scan found ${discovered.length} workspace(s)`);
            setWorkspaces(discovered);
          }
        }
      } catch (error) {
        console.error("[useWorkspaces] Failed to load workspaces:", error);
      }
    };

    loadAndScan();
  }, [setWorkspaces]);

  // Subscribe to git status events for all workspaces
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      for (const ws of workspaces) {
        const unlisten = await onGitStatusUpdated(ws.id, (status: GitStatus) => {
          updateGitStatus(ws.id, status);
        });
        unlisteners.push(unlisten);

        // Fetch initial status
        getGitStatus(ws.id)
          .then((status) => updateGitStatus(ws.id, status))
          .catch(() => {}); // Workspace may not be a git repo
      }

      // Start the filesystem watcher
      if (workspaces.length > 0) {
        startGitWatching().catch(console.error);
      }
    };

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [workspaces, updateGitStatus]);

  const handleAddWorkspace = useCallback(
    async (path: string) => {
      const ws = await addWorkspace(path);
      addWorkspaces([ws]);
      return ws;
    },
    [addWorkspaces],
  );

  const handleDiscoverWorkspaces = useCallback(
    async (input: string) => {
      // Check if input looks like git worktree list output (even without newlines)
      if (input.includes("worktree /") && input.includes("branch refs/heads/")) {
        console.log("[useWorkspaces] Detected git worktree list output, parsing...");
        const workspaces: any[] = [];

        // Split on "worktree " to get each worktree block
        const blocks = input.split(/worktree /).filter(Boolean);

        for (let i = 0; i < blocks.length; i++) {
          const block = blocks[i];

          // Extract path (everything up to the first space or "HEAD")
          const pathMatch = block.match(/^([^\s]+)/);
          if (!pathMatch) continue;

          const path = pathMatch[1];
          const name = path.split("/").pop() || "workspace";

          // Extract branch
          const branchMatch = block.match(/branch refs\/heads\/([^\s]+)/);
          const gitBranch = branchMatch ? branchMatch[1] : null;

          // If no branch, try to get short SHA from HEAD
          const headMatch = block.match(/HEAD ([a-f0-9]+)/);
          const finalBranch = gitBranch || (headMatch ? headMatch[1].substring(0, 7) : null);

          // Generate a stable ID from the path
          const id = path.replace(/[^a-zA-Z0-9]/g, '_');

          workspaces.push({
            id,
            path,
            name,
            gitBranch: finalBranch,
            isMainWorktree: i === 0, // First worktree is the main one
          });
        }

        if (workspaces.length > 0) {
          console.log(`[useWorkspaces] Parsed ${workspaces.length} worktrees:`, workspaces);
          addWorkspaces(workspaces);
          return workspaces;
        }
      }

      // Fall back to discovery
      const discovered = await discoverWorkspaces(input);
      addWorkspaces(discovered);
      return discovered;
    },
    [addWorkspaces],
  );

  const handleRemoveWorkspace = useCallback(
    async (id: string) => {
      await removeWorkspace(id);
      removeWorkspaceFromStore(id);
    },
    [removeWorkspaceFromStore],
  );

  const handleScanForWorkspaces = useCallback(async (cwd?: string) => {
    console.log("[useWorkspaces] Starting workspace scan from:", cwd || "default directories");
    const { scanForWorkspaces } = await import("@/services/tauri");
    const discovered = await scanForWorkspaces(cwd);
    addWorkspaces(discovered);
    console.log(`[useWorkspaces] Scan complete: found ${discovered.length} new workspace(s)`);
    if (discovered.length === 0) {
      console.log("[useWorkspaces] No new workspaces found (existing ones are already tracked)");
    }
    return discovered;
  }, [addWorkspaces]);

  return {
    workspaces,
    activeWorkspaceId,
    gitStatuses,
    setActiveWorkspace,
    addWorkspace: handleAddWorkspace,
    discoverWorkspaces: handleDiscoverWorkspaces,
    removeWorkspace: handleRemoveWorkspace,
    scanForWorkspaces: handleScanForWorkspaces,
  };
}
