import { useEffect, useRef, useCallback } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
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

  const initTerminal = useCallback(() => {
    if (!containerRef.current || terminalRef.current) return;

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

    // Try WebGL, fall back to canvas
    try {
      const webglAddon = new WebglAddon();
      terminal.loadAddon(webglAddon);
    } catch {
      console.warn("WebGL addon failed to load, using canvas renderer");
    }

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

    // Send initial size to backend
    resizeTerminal(terminalId, terminal.rows, terminal.cols);
  }, [terminalId]);

  // Set up data listener
  useEffect(() => {
    let unlistenData: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;

    const setup = async () => {
      unlistenData = await onTerminalData(terminalId, (data) => {
        if (terminalRef.current) {
          terminalRef.current.write(new Uint8Array(data));
        }
      });

      unlistenExit = await onTerminalExit(terminalId, () => {
        onExit?.();
      });
    };

    setup();

    return () => {
      unlistenData?.();
      unlistenExit?.();
    };
  }, [terminalId, onExit]);

  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      fitAddonRef.current?.fit();
    };

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  // Cleanup
  useEffect(() => {
    return () => {
      terminalRef.current?.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, []);

  const focus = useCallback(() => {
    terminalRef.current?.focus();
  }, []);

  const fit = useCallback(() => {
    fitAddonRef.current?.fit();
  }, []);

  return {
    containerRef,
    initTerminal,
    focus,
    fit,
  };
}
