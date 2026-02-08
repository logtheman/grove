import { create } from "zustand";
import type { TerminalTab } from "@/types";

interface AppStore {
  // Terminal tabs
  tabs: TerminalTab[];
  activeTabId: string | null;

  // Actions
  addTab: (tab: TerminalTab) => void;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabTitle: (id: string, title: string) => void;
}

export const useAppStore = create<AppStore>((set, get) => ({
  tabs: [],
  activeTabId: null,

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
}));
