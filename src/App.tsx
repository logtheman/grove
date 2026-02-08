import { useCallback, useEffect } from "react";
import { useAppStore } from "@/stores/appStore";
import { createTerminal, closeTerminal } from "@/services/tauri";
import { useWorkspaces } from "@/hooks/useWorkspaces";
import { TerminalTab } from "@/components/terminal/TerminalTab";
import { TerminalTabBar } from "@/components/terminal/TerminalTabBar";
import { WorkspaceSidebar } from "@/components/workspace/WorkspaceSidebar";

function App() {
  const { tabs, activeTabId, addTab, removeTab, setActiveTab } = useAppStore();

  const {
    workspaces,
    activeWorkspaceId,
    gitStatuses,
    setActiveWorkspace,
    discoverWorkspaces,
    removeWorkspace,
  } = useWorkspaces();

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId);

  const handleNewTab = useCallback(async () => {
    try {
      const cwd = activeWorkspace?.path;
      const info = await createTerminal(cwd);
      addTab({
        id: info.id,
        cwd: info.cwd,
        title: activeWorkspace?.name ?? `Terminal ${tabs.length + 1}`,
      });
    } catch (err) {
      console.error("Failed to create terminal:", err);
    }
  }, [addTab, tabs.length, activeWorkspace]);

  const handleCloseTab = useCallback(
    async (id: string) => {
      try {
        await closeTerminal(id);
      } catch {
        // Terminal may already be gone
      }
      removeTab(id);
    },
    [removeTab],
  );

  const handleTerminalExit = useCallback(
    (id: string) => {
      removeTab(id);
    },
    [removeTab],
  );

  const handleSelectWorkspace = useCallback(
    (id: string) => {
      setActiveWorkspace(id);
    },
    [setActiveWorkspace],
  );

  const handleDiscoverWorkspaces = useCallback(
    async (path: string) => {
      try {
        await discoverWorkspaces(path);
      } catch (err) {
        console.error("Failed to discover workspaces:", err);
      }
    },
    [discoverWorkspaces],
  );

  // Create first terminal on mount
  useEffect(() => {
    if (tabs.length === 0) {
      handleNewTab();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === "T") {
        e.preventDefault();
        handleNewTab();
      }
      if (e.ctrlKey && e.shiftKey && e.key === "W") {
        e.preventDefault();
        if (activeTabId) {
          handleCloseTab(activeTabId);
        }
      }
      if (e.ctrlKey && e.key === "Tab" && !e.shiftKey) {
        e.preventDefault();
        const idx = tabs.findIndex((t) => t.id === activeTabId);
        if (idx >= 0 && tabs.length > 1) {
          const nextIdx = (idx + 1) % tabs.length;
          setActiveTab(tabs[nextIdx].id);
        }
      }
      if (e.ctrlKey && e.shiftKey && e.key === "Tab") {
        e.preventDefault();
        const idx = tabs.findIndex((t) => t.id === activeTabId);
        if (idx >= 0 && tabs.length > 1) {
          const prevIdx = (idx - 1 + tabs.length) % tabs.length;
          setActiveTab(tabs[prevIdx].id);
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [tabs, activeTabId, handleNewTab, handleCloseTab, setActiveTab]);

  // Count dirty workspaces for status bar
  const dirtyCount = Object.values(gitStatuses).filter((s) => s.dirty).length;

  return (
    <div className="flex h-screen bg-grove-bg">
      {/* Sidebar */}
      <WorkspaceSidebar
        workspaces={workspaces}
        activeWorkspaceId={activeWorkspaceId}
        gitStatuses={gitStatuses}
        onSelectWorkspace={handleSelectWorkspace}
        onDiscoverWorkspaces={handleDiscoverWorkspaces}
        onRemoveWorkspace={removeWorkspace}
      />

      {/* Main area */}
      <div className="flex flex-col flex-1 min-w-0">
        {/* Tab bar */}
        <TerminalTabBar
          tabs={tabs}
          activeTabId={activeTabId}
          onSelectTab={setActiveTab}
          onCloseTab={handleCloseTab}
          onNewTab={handleNewTab}
        />

        {/* Terminal area */}
        <div className="flex-1 relative overflow-hidden">
          {tabs.length === 0 && (
            <div className="flex items-center justify-center h-full text-grove-text-muted">
              <div className="text-center">
                <p className="text-lg mb-2">No terminals open</p>
                <p className="text-sm">
                  Press{" "}
                  <kbd className="px-1 py-0.5 bg-grove-surface rounded text-xs">
                    Ctrl+Shift+T
                  </kbd>{" "}
                  to open a new terminal
                </p>
              </div>
            </div>
          )}

          {tabs.map((tab) => (
            <TerminalTab
              key={tab.id}
              terminalId={tab.id}
              isActive={tab.id === activeTabId}
              onExit={() => handleTerminalExit(tab.id)}
            />
          ))}
        </div>

        {/* Status bar */}
        <div className="flex items-center px-3 h-6 bg-grove-surface border-t border-grove-border text-xs text-grove-text-muted">
          <span>
            {workspaces.length} workspace{workspaces.length !== 1 ? "s" : ""}
          </span>
          <span className="mx-2">|</span>
          <span>
            {tabs.length} terminal{tabs.length !== 1 ? "s" : ""}
          </span>
          {dirtyCount > 0 && (
            <>
              <span className="mx-2">|</span>
              <span className="text-grove-warning">{dirtyCount} dirty</span>
            </>
          )}
          <span className="ml-auto">Grove v0.1.0</span>
        </div>
      </div>
    </div>
  );
}

export default App;
