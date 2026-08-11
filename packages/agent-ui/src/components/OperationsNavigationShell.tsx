import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export const OPERATIONS_VIEWS = [
  "overview",
  "work",
  "runs",
  "inbox",
  "agents",
  "governance",
] as const;

export type OperationsView = (typeof OPERATIONS_VIEWS)[number];

/** Milestone 1 Work OS destinations (SCREENS.md). */
export const WORK_OS_VIEWS = ["work", "inbox"] as const;
export type WorkOsView = (typeof WORK_OS_VIEWS)[number];

export type OperationsNavigationShellProps = {
  view: OperationsView;
  onViewChange: (view: OperationsView) => void;
  /** Hosts expose only completed domain slices; unavailable routes are omitted. */
  availableViews: readonly OperationsView[];
  children?: ReactNode;
  /** Accessible name for the shell. Defaults to Work when only Work OS views. */
  ariaLabel?: string;
};

const LABELS: Record<OperationsView, string> = {
  overview: "Overview",
  work: "Work",
  runs: "Runs",
  inbox: "Inbox",
  agents: "Agents",
  governance: "Governance",
};

/**
 * Shared product navigation chrome; hosts supply data and capability state.
 * Only `availableViews` are rendered (no disabled peer destinations).
 */
export function OperationsNavigationShell({
  view,
  onViewChange,
  availableViews,
  children,
  ariaLabel,
}: OperationsNavigationShellProps) {
  const label =
    ariaLabel ??
    (availableViews.every((item) => item === "work" || item === "inbox")
      ? "Work"
      : "Operations");

  return (
    <section
      aria-label={label}
      className="flex h-full min-h-0 flex-col overflow-hidden bg-card"
    >
      <nav
        aria-label={`${label} navigation`}
        className="shrink-0 border-b border-border/50 px-3 py-2"
      >
        <div role="tablist" className="flex gap-1 overflow-x-auto">
          {availableViews.map((item) => {
            const selected = item === view;
            return (
              <button
                key={item}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => onViewChange(item)}
                className={cn(
                  "shrink-0 rounded-md px-2.5 py-1.5 text-[10.5px] font-medium transition-colors",
                  selected
                    ? "bg-accent text-foreground"
                    : "text-muted-foreground hover:bg-muted/70 hover:text-foreground",
                )}
              >
                {LABELS[item]}
              </button>
            );
          })}
        </div>
      </nav>
      <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
    </section>
  );
}
