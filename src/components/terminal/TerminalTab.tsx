import { useEffect } from "react";
import { useTerminal } from "@/hooks/useTerminal";
import "@xterm/xterm/css/xterm.css";

interface TerminalTabProps {
  terminalId: string;
  isActive: boolean;
  onExit: () => void;
}

export function TerminalTab({ terminalId, isActive, onExit }: TerminalTabProps) {
  const { containerRef, focus, fit } = useTerminal({
    terminalId,
    onExit,
  });

  useEffect(() => {
    if (isActive) {
      requestAnimationFrame(() => {
        fit();
        focus();
      });
    }
  }, [isActive, focus, fit]);

  return (
    <div
      className="xterm-container"
      style={{
        visibility: isActive ? "visible" : "hidden",
        zIndex: isActive ? 1 : 0,
      }}
    >
      {/* DEBUG indicator */}
      <div
        className="debug-indicator"
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          background: "red",
          color: "white",
          padding: "8px",
          zIndex: 9999,
          fontSize: "12px",
        }}
      >
        Waiting for data...
      </div>
      <div ref={containerRef} className="h-full w-full" />
    </div>
  );
}
