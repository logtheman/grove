export interface TerminalInfo {
  id: string;
  cwd: string;
}

export interface TerminalTab {
  id: string;
  cwd: string;
  title: string;
}

export interface Workspace {
  id: string;
  name: string;
  path: string;
  gitBranch: string | null;
  isMainWorktree: boolean;
}

export interface GitStatus {
  workspaceId: string;
  branch: string | null;
  remoteBranch: string | null;
  ahead: number;
  behind: number;
  dirty: boolean;
  untrackedCount: number;
  stagedCount: number;
  modifiedCount: number;
}

export interface ProcessInfo {
  pid: number;
  command: string;
  state: "running" | "stopped";
  terminalId: string;
}

export interface TaskCounts {
  pending: number;
  in_progress: number;
  completed: number;
}

export interface ClaudeSession {
  sessionId: string;
  workspaceId: string | null;
  workspaceName: string | null;
  cwd: string | null;
  gitBranch: string | null;
  model: string | null;
  lastMessageType: string | null;
  lastTimestamp: string | null;
  taskCount: TaskCounts;
  active: boolean;
}
