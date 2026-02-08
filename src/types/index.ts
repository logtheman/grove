export interface TerminalInfo {
  id: string;
  cwd: string;
}

export interface TerminalTab {
  id: string;
  cwd: string;
  title: string;
}

export interface WorkspaceInfo {
  id: string;
  name: string;
  path: string;
  gitBranch?: string;
  dirty: boolean;
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

export interface ClaudeSession {
  workspaceId: string;
  sessionId: string;
  gitBranch: string | null;
  active: boolean;
  taskCount: number;
}
