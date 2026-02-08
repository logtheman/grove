import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalInfo, Workspace, GitStatus, ClaudeSession } from "@/types";

// ── Terminal ──

export async function createTerminal(cwd?: string): Promise<TerminalInfo> {
  return invoke<TerminalInfo>("create_terminal", { cwd });
}

export async function closeTerminal(terminalId: string): Promise<void> {
  return invoke("close_terminal", { terminalId });
}

export async function writeToTerminal(
  terminalId: string,
  data: Uint8Array,
): Promise<void> {
  return invoke("write_to_terminal", {
    terminalId,
    data: Array.from(data),
  });
}

export async function resizeTerminal(
  terminalId: string,
  rows: number,
  cols: number,
): Promise<void> {
  return invoke("resize_terminal", { terminalId, rows, cols });
}

export async function listTerminals(): Promise<string[]> {
  return invoke<string[]>("list_terminals");
}

export function onTerminalData(
  terminalId: string,
  callback: (data: number[]) => void,
): Promise<UnlistenFn> {
  return listen<number[]>(`terminal-data:${terminalId}`, (event) => {
    callback(event.payload);
  });
}

export function onTerminalExit(
  terminalId: string,
  callback: () => void,
): Promise<UnlistenFn> {
  return listen(`terminal-exit:${terminalId}`, () => {
    callback();
  });
}

// ── Workspace ──

export async function addWorkspace(path: string): Promise<Workspace> {
  return invoke<Workspace>("add_workspace", { path });
}

export async function discoverWorkspaces(
  repoPath: string,
): Promise<Workspace[]> {
  return invoke<Workspace[]>("discover_workspaces", { repoPath });
}

export async function removeWorkspace(workspaceId: string): Promise<void> {
  return invoke("remove_workspace", { workspaceId });
}

export async function listWorkspaces(): Promise<Workspace[]> {
  return invoke<Workspace[]>("list_workspaces");
}

// ── Git ──

export async function getGitStatus(workspaceId: string): Promise<GitStatus> {
  return invoke<GitStatus>("get_git_status", { workspaceId });
}

export async function startGitWatching(): Promise<void> {
  return invoke("start_git_watching");
}

export function onGitStatusUpdated(
  workspaceId: string,
  callback: (status: GitStatus) => void,
): Promise<UnlistenFn> {
  return listen<GitStatus>(
    `git-status-updated:${workspaceId}`,
    (event) => {
      callback(event.payload);
    },
  );
}

// ── Claude Code ──

export async function startClaudeMonitoring(): Promise<void> {
  return invoke("start_claude_monitoring");
}

export function onClaudeSessionsUpdated(
  callback: (sessions: ClaudeSession[]) => void,
): Promise<UnlistenFn> {
  return listen<ClaudeSession[]>("claude-sessions-updated", (event) => {
    callback(event.payload);
  });
}
