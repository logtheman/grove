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
  // Store onExit in a ref so it doesn't trigger useEffect re-runs
  const onExitRef = useRef(onExit);
  onExitRef.current = onExit;

  // Single useEffect that handles init, data listener, and cleanup together
  // NOTE: only depends on terminalId, NOT onExit (stored in ref above)
  useEffect(() => {
    console.log("[grove-term] useEffect: container=", !!containerRef.current, "initialized=", initializedRef.current, "terminalId=", terminalId);
    if (!containerRef.current || initializedRef.current) return;
    initializedRef.current = true;
    console.log("[grove-term] initializing terminal:", terminalId);

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
    console.log("[grove-term] xterm opened in DOM");

    // Skip WebGL - canvas renderer is more reliable cross-platform
    fitAddon.fit();
    console.log("[grove-term] fitAddon.fit() done, rows=", terminal.rows, "cols=", terminal.cols);

    // Send input to PTY
    terminal.onData((data) => {
      console.log("[grove-term] onData from xterm:", JSON.stringify(data));
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

    // Set up data listener IMMEDIATELY (before terminal renders) to avoid race condition
    // where PTY data arrives before frontend is listening
    let unlistenData: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;

    // Use IIFE to await listener setup while still allowing synchronous useEffect return
    (async () => {
      console.log("[grove-term] setting up listeners for:", terminalId);
      console.error("[grove-term] SETTING UP LISTENERS FOR:", terminalId); // More visible

      // @ts-ignore - Add to window for debugging
      if (!window.groveDebug) window.groveDebug = { dataCount: 0, listenerSetup: false };
      window.groveDebug.listenerSetup = true;
      window.groveDebug.terminalId = terminalId;

      let dataCount = 0;
      try {
        unlistenData = await onTerminalData(terminalId, (data) => {
          const bytes = new Uint8Array(data);
          dataCount++;
          window.groveDebug.dataCount = dataCount;
          window.groveDebug.lastByteCount = bytes.length;

          console.log("[grove-term] DATA RECEIVED!", dataCount, "chunks,", bytes.length, "bytes", "data:", data);
          console.error("[grove-term] DATA RECEIVED!", dataCount, "chunks,", bytes.length, "bytes"); // More visible

          // Update debug indicator if it exists
          const indicator = document.querySelector('.debug-indicator');
          if (indicator) {
            indicator.textContent = `DATA RECEIVED: ${dataCount} chunks, ${bytes.length} bytes (last)`;
            (indicator as HTMLElement).style.background = "green";
          }

          console.log("[grove-term] terminalRef exists?", !!terminalRef.current, "bytes:", bytes);
          if (terminalRef.current) {
            console.log("[grove-term] Calling terminal.write()...");
            try {
              terminalRef.current.write(bytes);
              console.log("[grove-term] terminal.write() completed successfully");
            } catch (error) {
              console.error("[grove-term] terminal.write() FAILED:", error);
            }
          } else {
            console.log("[grove-term] Buffering - terminal not ready");
            pendingDataRef.current.push(bytes);
          }
        });

        unlistenExit = await onTerminalExit(terminalId, () => {
          onExitRef.current?.();
        });

        console.log("[grove-term] listeners successfully set up for:", terminalId);
        console.error("[grove-term] LISTENERS READY!", terminalId);

        // WORKAROUND: Send Ctrl+L to clear screen and redraw prompt
        // This fixes the race condition where initial prompt was lost
        setTimeout(() => {
          console.log("[grove-term] Sending Ctrl+L to redraw prompt");
          const ctrlL = new Uint8Array([12]); // ASCII 12 = Ctrl+L
          writeToTerminal(terminalId, ctrlL);
        }, 100);

        // Auto-capture debug state for Claude debugging (after terminal setup)
        setTimeout(async () => {
          try {
            const { writeDebugLog } = await import("@/services/tauri");
            const termContainer = document.querySelector('.xterm');
            const screen = document.querySelector('.xterm-screen');
            const viewport = document.querySelector('.xterm-viewport');
            const rows = document.querySelector('.xterm-rows');
            const canvas = document.querySelector('canvas');

            const debugState = {
              timestamp: new Date().toISOString(),
              terminalId,
              groveDebug: (window as any).groveDebug,
              terminal: {
                exists: !!termContainer,
                visible: termContainer?.checkVisibility(),
                dimensions: termContainer ? {
                  width: (termContainer as HTMLElement).clientWidth,
                  height: (termContainer as HTMLElement).clientHeight,
                  scrollHeight: (termContainer as HTMLElement).scrollHeight,
                } : null,
                viewport: viewport ? {
                  scrollTop: (viewport as HTMLElement).scrollTop,
                  scrollHeight: (viewport as HTMLElement).scrollHeight,
                  clientHeight: (viewport as HTMLElement).clientHeight,
                } : null,
                screen: screen ? {
                  childCount: screen.childElementCount,
                  innerHTML: screen.innerHTML.slice(0, 2000),
                  textContent: screen.textContent?.slice(0, 1000),
                } : null,
                rows: rows ? {
                  exists: true,
                  childCount: rows.childElementCount,
                  innerHTML: rows.innerHTML.slice(0, 500),
                } : { exists: false },
                canvas: canvas ? {
                  exists: true,
                  width: (canvas as HTMLCanvasElement).width,
                  height: (canvas as HTMLCanvasElement).height,
                } : { exists: false },
                fullStructure: termContainer?.innerHTML.slice(0, 5000),
                rowsParent: rows?.parentElement?.className,
                rowsNextSibling: rows?.nextSibling?.nodeName,
                screenChildren: Array.from(screen?.children || []).map((el: any) => ({
                  tag: el.tagName,
                  className: el.className,
                  id: el.id,
                })),
              },
            };

            await writeDebugLog('/tmp/grove-debug.json', JSON.stringify(debugState, null, 2));
            console.log("[grove-term] Debug state written to /tmp/grove-debug.json");
          } catch (error) {
            console.error("[grove-term] Failed to capture debug state:", error);
          }
        }, 2000);
      } catch (error) {
        console.error("[grove-term] Error setting up listeners:", error);
        alert("Failed to set up terminal listeners: " + error);
      }
    })();

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
  }, [terminalId]);

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
