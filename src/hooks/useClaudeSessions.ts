import { useEffect, useState } from "react";
import {
  startClaudeMonitoring,
  onClaudeSessionsUpdated,
} from "@/services/tauri";
import type { ClaudeSession } from "@/types";

export function useClaudeSessions() {
  const [sessions, setSessions] = useState<ClaudeSession[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      unlisten = await onClaudeSessionsUpdated((newSessions) => {
        setSessions(newSessions);
      });

      // Start monitoring
      startClaudeMonitoring().catch(console.error);
    };

    setup();

    return () => {
      unlisten?.();
    };
  }, []);

  const activeSessions = sessions.filter((s) => s.active);
  const totalInProgress = sessions.reduce(
    (sum, s) => sum + s.taskCount.in_progress,
    0,
  );

  return {
    sessions,
    activeSessions,
    totalInProgress,
  };
}
