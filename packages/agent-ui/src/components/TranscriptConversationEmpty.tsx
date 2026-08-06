import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type TranscriptConversationEmptyProps = {
  title?: string;
  description?: string;
  icon?: ReactNode;
  /** Footer slot (e.g. host `AgentStatusPill`). */
  children?: ReactNode;
  className?: string;
};

/**
 * Empty transcript body chrome (no messages yet).
 * Wave 4 / A6.7 — host owns the scroll container / status pill slot.
 */
export function TranscriptConversationEmpty({
  title = "Ask ALTAI anything",
  description = "Explain command output, fix errors, generate snippets, or run a task.",
  icon,
  children,
  className,
}: TranscriptConversationEmptyProps) {
  return (
    <div
      className={cn(
        "flex size-full flex-col items-center justify-center gap-3 p-8 text-center",
        className,
      )}
    >
      {icon ? <div className="text-muted-foreground">{icon}</div> : null}
      <div className="space-y-1">
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        {description ? (
          <p className="text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {children}
    </div>
  );
}
