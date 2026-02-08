import { create } from "zustand";
import type { TerminalTab, Workspace, GitStatus } from "@/types";

interface AppStore {
  // Terminal tabs
  tabs: TerminalTab[];
  activeTabId: string | null;

  // Workspaces
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  gitStatuses: Record<string, GitStatus>;

  // Terminal actions
  addTab: (tab: TerminalTab) => void;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabTitle: (id: string, title: string) => void;

  // Workspace actions
  setWorkspaces: (workspaces: Workspace[]) => void;
  addWorkspaces: (workspaces: Workspace[]) => void;
  removeWorkspaceFromStore: (id: string) => void;
  setActiveWorkspace: (id: string | null) => void;
  updateGitStatus: (workspaceId: string, status: GitStatus) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  // Initial state
  tabs: [],
  activeTabId: null,
  workspaces: [],
  activeWorkspaceId: null,
  gitStatuses: {},

  // Terminal actions
  addTab: (tab) =>
    set((state) => ({
      tabs: [...state.tabs, tab],
      activeTabId: tab.id,
    })),

  removeTab: (id) =>
    set((state) => {
      const newTabs = state.tabs.filter((t) => t.id !== id);
      const wasActive = state.activeTabId === id;
      return {
        tabs: newTabs,
        activeTabId: wasActive
          ? (newTabs[newTabs.length - 1]?.id ?? null)
          : state.activeTabId,
      };
    }),

  setActiveTab: (id) => set({ activeTabId: id }),

  updateTabTitle: (id, title) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === id ? { ...t, title } : t)),
    })),

  // Workspace actions
  setWorkspaces: (workspaces) =>
    set({
      workspaces,
      activeWorkspaceId: workspaces[0]?.id ?? null,
    }),

  addWorkspaces: (newWorkspaces) =>
    set((state) => {
      const existingIds = new Set(state.workspaces.map((w) => w.id));
      const toAdd = newWorkspaces.filter((w) => !existingIds.has(w.id));
      const all = [...state.workspaces, ...toAdd];
      return {
        workspaces: all,
        activeWorkspaceId: state.activeWorkspaceId ?? all[0]?.id ?? null,
      };
    }),

  removeWorkspaceFromStore: (id) =>
    set((state) => {
      const remaining = state.workspaces.filter((w) => w.id !== id);
      const { [id]: _removed, ...otherStatuses } = state.gitStatuses;
      return {
        workspaces: remaining,
        gitStatuses: otherStatuses,
        activeWorkspaceId:
          state.activeWorkspaceId === id
            ? (remaining[0]?.id ?? null)
            : state.activeWorkspaceId,
      };
    }),

  setActiveWorkspace: (id) => set({ activeWorkspaceId: id }),

  updateGitStatus: (workspaceId, status) =>
    set((state) => ({
      gitStatuses: { ...state.gitStatuses, [workspaceId]: status },
    })),
}));
