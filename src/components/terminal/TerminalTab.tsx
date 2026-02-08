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
      <div ref={containerRef} className="h-full w-full" />
    </div>
  );
}
