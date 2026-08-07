import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AgentChatLayoutDensity = "sidebar" | "desktop";

export type AgentChatLayoutProps = {
  /** History column or stacked sessions surface. */
  history?: ReactNode;
  /** Primary chat column (messages + composer). */
  main: ReactNode;
  /**
   * `sidebar` = column stack for narrow VS Code Activity Bar.
   * `desktop` = history rail beside main (Desktop / wide secondary sidebar).
   */
  density?: AgentChatLayoutDensity;
  className?: string;
};

/**
 * Shared outer split for chat: history + main. Hosts supply content slots only.
 * VS Code uses density="sidebar"; Desktop uses density="desktop".
 */
export function AgentChatLayout({
  history,
  main,
  density = "sidebar",
  className,
}: AgentChatLayoutProps) {
  const hasHistory = history != null && history !== false;
  return (
    <div
      className={cn(
        "altai-agent-chat-layout flex min-h-0 flex-1",
        density === "desktop" ? "flex-row" : "flex-col",
        className,
      )}
      data-density={density}
    >
      {hasHistory ? (
        <div
          className={cn(
            "altai-agent-chat-history flex min-h-0 min-w-0",
            density === "desktop"
              ? "w-[11.5rem] max-w-56 shrink-0 flex-col border-r border-border-subtle"
              : "w-full shrink-0 flex-col border-b border-border-subtle",
          )}
        >
          {history}
        </div>
      ) : null}
      <div className="altai-agent-chat-main flex min-h-0 min-w-0 flex-1 flex-col">
        {main}
      </div>
    </div>
  );
}
