import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalInfo } from "@/types";

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
