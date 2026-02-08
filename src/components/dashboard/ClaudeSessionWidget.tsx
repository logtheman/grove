import { Bot, CheckCircle2, Circle, Clock, Loader2 } from "lucide-react";
import type { ClaudeSession } from "@/types";

interface ClaudeSessionWidgetProps {
  sessions: ClaudeSession[];
}

export function ClaudeSessionWidget({ sessions }: ClaudeSessionWidgetProps) {
  if (sessions.length === 0) {
    return (
      <div className="px-3 py-2">
        <div className="flex items-center gap-2 mb-2">
          <Bot size={14} className="text-grove-text-muted" />
          <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
            Claude Code
          </span>
        </div>
        <p className="text-xs text-grove-text-muted">No active sessions</p>
      </div>
    );
  }

  return (
    <div className="px-3 py-2">
      <div className="flex items-center gap-2 mb-2">
        <Bot size={14} className="text-grove-accent" />
        <span className="text-xs font-medium text-grove-text-muted uppercase tracking-wider">
          Claude Code
        </span>
        <span className="text-xs text-grove-accent ml-auto">
          {sessions.filter((s) => s.active).length} active
        </span>
      </div>

      <div className="space-y-2">
        {sessions.map((session) => (
          <SessionCard key={session.sessionId} session={session} />
        ))}
      </div>
    </div>
  );
}

function SessionCard({ session }: { session: ClaudeSession }) {
  const totalTasks =
    session.taskCount.pending +
    session.taskCount.in_progress +
    session.taskCount.completed;

  const timeAgo = session.lastTimestamp
    ? formatTimeAgo(session.lastTimestamp)
    : null;

  return (
    <div className="bg-grove-bg rounded px-2 py-1.5 border border-grove-border">
      {/* Session header */}
      <div className="flex items-center gap-1.5">
        {session.active ? (
          <Loader2 size={10} className="text-grove-success animate-spin" />
        ) : (
          <Clock size={10} className="text-grove-text-muted" />
        )}
        <span className="text-xs text-grove-text truncate">
          {session.gitBranch ?? session.sessionId.slice(0, 8)}
        </span>
      </div>

      {/* Model + cwd */}
      <div className="flex items-center gap-1 mt-0.5">
        {session.model && (
          <span className="text-[10px] text-grove-text-muted">
            {formatModel(session.model)}
          </span>
        )}
        {timeAgo && (
          <span className="text-[10px] text-grove-text-muted ml-auto">
            {timeAgo}
          </span>
        )}
      </div>

      {/* Tasks */}
      {totalTasks > 0 && (
        <div className="flex items-center gap-2 mt-1">
          {session.taskCount.in_progress > 0 && (
            <span className="flex items-center gap-0.5 text-[10px] text-grove-accent">
              <Loader2 size={8} className="animate-spin" />
              {session.taskCount.in_progress}
            </span>
          )}
          {session.taskCount.pending > 0 && (
            <span className="flex items-center gap-0.5 text-[10px] text-grove-text-muted">
              <Circle size={8} />
              {session.taskCount.pending}
            </span>
          )}
          {session.taskCount.completed > 0 && (
            <span className="flex items-center gap-0.5 text-[10px] text-grove-success">
              <CheckCircle2 size={8} />
              {session.taskCount.completed}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function formatModel(model: string): string {
  if (model.includes("opus")) return "Opus";
  if (model.includes("sonnet")) return "Sonnet";
  if (model.includes("haiku")) return "Haiku";
  return model.split("-").slice(0, 2).join(" ");
}

function formatTimeAgo(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);

  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  return `${Math.floor(diffHr / 24)}d ago`;
}
