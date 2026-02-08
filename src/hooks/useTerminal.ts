import { useEffect, useRef, useCallback } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import {
  writeToTerminal,
  resizeTerminal,
  onTerminalData,
  onTerminalExit,
} from "@/services/tauri";

interface UseTerminalOptions {
  terminalId: string;
  onExit?: () => void;
}

export function useTerminal({ terminalId, onExit }: UseTerminalOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const initializedRef = useRef(false);
  // Buffer data that arrives before terminal is ready
  const pendingDataRef = useRef<Uint8Array[]>([]);

  // Single useEffect that handles init, data listener, and cleanup together
  useEffect(() => {
    if (!containerRef.current || initializedRef.current) return;
    initializedRef.current = true;

    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "block",
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', monospace",
      theme: {
        background: "#1a1b26",
        foreground: "#c0caf5",
        cursor: "#c0caf5",
        selectionBackground: "#33467c",
        black: "#15161e",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        blue: "#7aa2f7",
        magenta: "#bb9af7",
        cyan: "#7dcfff",
        white: "#a9b1d6",
        brightBlack: "#414868",
        brightRed: "#f7768e",
        brightGreen: "#9ece6a",
        brightYellow: "#e0af68",
        brightBlue: "#7aa2f7",
        brightMagenta: "#bb9af7",
        brightCyan: "#7dcfff",
        brightWhite: "#c0caf5",
      },
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(containerRef.current);

    // Skip WebGL - canvas renderer is more reliable cross-platform
    fitAddon.fit();

    // Send input to PTY
    terminal.onData((data) => {
      const encoded = new TextEncoder().encode(data);
      writeToTerminal(terminalId, encoded);
    });

    // Handle resize
    terminal.onResize(({ rows, cols }) => {
      resizeTerminal(terminalId, rows, cols);
    });

    terminalRef.current = terminal;
    fitAddonRef.current = fitAddon;

    // Flush any buffered data
    for (const chunk of pendingDataRef.current) {
      terminal.write(chunk);
    }
    pendingDataRef.current = [];

    // Send initial size to backend
    resizeTerminal(terminalId, terminal.rows, terminal.cols);

    // Focus the terminal
    terminal.focus();

    // Set up data listener
    let unlistenData: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;

    const setupListeners = async () => {
      unlistenData = await onTerminalData(terminalId, (data) => {
        const bytes = new Uint8Array(data);
        if (terminalRef.current) {
          terminalRef.current.write(bytes);
        } else {
          pendingDataRef.current.push(bytes);
        }
      });

      unlistenExit = await onTerminalExit(terminalId, () => {
        onExit?.();
      });
    };

    setupListeners();

    // Window resize handler
    const handleResize = () => {
      fitAddonRef.current?.fit();
    };
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      unlistenData?.();
      unlistenExit?.();
      terminal.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      initializedRef.current = false;
    };
  }, [terminalId, onExit]);

  const focus = useCallback(() => {
    terminalRef.current?.focus();
  }, []);

  const fit = useCallback(() => {
    fitAddonRef.current?.fit();
  }, []);

  return {
    containerRef,
    focus,
    fit,
  };
}
