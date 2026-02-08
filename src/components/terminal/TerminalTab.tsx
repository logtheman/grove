import { useEffect } from "react";
import { useTerminal } from "@/hooks/useTerminal";
import "@xterm/xterm/css/xterm.css";

interface TerminalTabProps {
  terminalId: string;
  isActive: boolean;
  onExit: () => void;
}

export function TerminalTab({ terminalId, isActive, onExit }: TerminalTabProps) {
  const { containerRef, initTerminal, focus, fit } = useTerminal({
    terminalId,
    onExit,
  });

  useEffect(() => {
    initTerminal();
  }, [initTerminal]);

  useEffect(() => {
    if (isActive) {
      // Small delay to ensure DOM is updated
      requestAnimationFrame(() => {
        fit();
        focus();
      });
    }
  }, [isActive, focus, fit]);

  return (
    <div
      className="xterm-container"
      style={{ display: isActive ? "block" : "none" }}
    >
      <div ref={containerRef} className="h-full w-full" />
    </div>
  );
}
