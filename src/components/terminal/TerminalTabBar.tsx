import { X, Plus } from "lucide-react";
import type { TerminalTab as Tab } from "@/types";

interface TerminalTabBarProps {
  tabs: Tab[];
  activeTabId: string | null;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onNewTab: () => void;
}

export function TerminalTabBar({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onNewTab,
}: TerminalTabBarProps) {
  return (
    <div className="flex items-center bg-grove-bg border-b border-grove-border h-9 select-none">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`
            flex items-center gap-2 px-3 h-full cursor-pointer
            border-r border-grove-border text-sm
            ${
              tab.id === activeTabId
                ? "bg-grove-surface text-grove-text"
                : "text-grove-text-muted hover:bg-grove-surface/50"
            }
          `}
          onClick={() => onSelectTab(tab.id)}
        >
          <span className="truncate max-w-[120px]">{tab.title}</span>
          <button
            className="hover:bg-grove-border rounded p-0.5"
            onClick={(e) => {
              e.stopPropagation();
              onCloseTab(tab.id);
            }}
          >
            <X size={12} />
          </button>
        </div>
      ))}
      <button
        className="flex items-center justify-center w-8 h-full hover:bg-grove-surface/50 text-grove-text-muted"
        onClick={onNewTab}
        title="New terminal (Ctrl+Shift+T)"
      >
        <Plus size={14} />
      </button>
    </div>
  );
}
