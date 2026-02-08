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

  // Load workspaces on mount
  useEffect(() => {
    listWorkspaces().then(setWorkspaces).catch(console.error);
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
    async (repoPath: string) => {
      const discovered = await discoverWorkspaces(repoPath);
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

  return {
    workspaces,
    activeWorkspaceId,
    gitStatuses,
    setActiveWorkspace,
    addWorkspace: handleAddWorkspace,
    discoverWorkspaces: handleDiscoverWorkspaces,
    removeWorkspace: handleRemoveWorkspace,
  };
}
