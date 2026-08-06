import {
  Notebook01Icon,
  Notification01Icon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactElement, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type WorkspaceTopbarActionsProps = {
  variant: "workspace" | "sidebar";
  workOpen: boolean;
  inboxOpen: boolean;
  inboxAttentionCount: number;
  inspectorOpen: boolean;
  inspectorAvailable: boolean;
  onToggleWork: () => void;
  onToggleInbox: () => void;
  onToggleInspector: () => void;
  /**
   * Host wraps compact icon controls (Desktop uses Radix tooltip). Defaults to
   * the bare control.
   */
  renderTooltip?: (label: string, children: ReactElement) => ReactNode;
};

function defaultTooltip(_label: string, children: ReactElement): ReactNode {
  return children;
}

/**
 * Durable Work / Inbox / Run-details action cluster for the AI topbar.
 * Purely presentational; the host owns open state and transport.
 */
export function WorkspaceTopbarActions({
  variant,
  workOpen,
  inboxOpen,
  inboxAttentionCount,
  inspectorOpen,
  inspectorAvailable,
  onToggleWork,
  onToggleInbox,
  onToggleInspector,
  renderTooltip = defaultTooltip,
}: WorkspaceTopbarActionsProps) {
  const workLabel = "Open work in Operations";
  const inboxLabel = inboxAttentionCount
    ? `Open Operations inbox, ${inboxAttentionCount} need attention`
    : "Open Operations inbox";
  const inspectorLabel = inspectorOpen
    ? "Close run details"
    : "Open run details";

  return (
    <div className="altai-ai-topbar-actions flex shrink-0 items-center gap-0.5 rounded-lg border border-border/60 bg-muted/35 p-0.5">
      {renderTooltip(
        workLabel,
        <button
          type="button"
          onClick={onToggleWork}
          aria-label={workLabel}
          aria-pressed={workOpen}
          title={workLabel}
          className={cn(
            "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
            workOpen && "bg-foreground/[0.09] text-foreground",
          )}
        >
          <HugeiconsIcon icon={Notebook01Icon} size={14} strokeWidth={1.75} />
          {variant === "workspace" ? (
            <span className="hidden pr-0.5 text-[10px] font-medium @[40rem]:inline">
              Work
            </span>
          ) : null}
        </button>,
      )}
      {renderTooltip(
        inboxLabel,
        <button
          type="button"
          onClick={onToggleInbox}
          aria-label={inboxLabel}
          aria-pressed={inboxOpen}
          title={inboxLabel}
          className={cn(
            "relative inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
            inboxOpen && "bg-foreground/[0.09] text-foreground",
          )}
        >
          <HugeiconsIcon
            icon={Notification01Icon}
            size={14}
            strokeWidth={1.75}
          />
          {variant === "workspace" ? (
            <span className="hidden pr-0.5 text-[10px] font-medium @[40rem]:inline">
              Inbox
            </span>
          ) : null}
          {inboxAttentionCount ? (
            <span className="absolute -right-1 -top-1 flex min-w-3.5 items-center justify-center rounded-full bg-warning px-1 text-[8px] font-semibold leading-3 text-warning-foreground">
              {inboxAttentionCount > 99 ? "99+" : inboxAttentionCount}
            </span>
          ) : null}
        </button>,
      )}
      {inspectorAvailable
        ? renderTooltip(
            inspectorLabel,
            <button
              type="button"
              onClick={onToggleInspector}
              aria-label={inspectorLabel}
              aria-pressed={inspectorOpen}
              title={inspectorLabel}
              className={cn(
                "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
                inspectorOpen ? "bg-foreground/[0.09] text-foreground" : "",
              )}
            >
              <HugeiconsIcon icon={SparklesIcon} size={14} strokeWidth={1.75} />
            </button>,
          )
        : null}
    </div>
  );
}
