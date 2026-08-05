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

export type OperationsNavigationShellProps = {
  view: OperationsView;
  onViewChange: (view: OperationsView) => void;
  /** Hosts expose only completed domain slices; unavailable routes are inert. */
  availableViews: readonly OperationsView[];
  children?: ReactNode;
};

const LABELS: Record<OperationsView, string> = {
  overview: "Overview", work: "Work", runs: "Runs", inbox: "Inbox", agents: "Agents", governance: "Governance",
};

/** Shared product navigation chrome; hosts supply data and capability state. */
export function OperationsNavigationShell({ view, onViewChange, availableViews, children }: OperationsNavigationShellProps) {
  return (
    <section aria-label="Operations" className="flex h-full min-h-0 flex-col overflow-hidden bg-card">
      <nav aria-label="Operations navigation" className="shrink-0 border-b border-border/50 px-3 py-2">
        <div role="tablist" className="flex gap-1 overflow-x-auto">
          {OPERATIONS_VIEWS.map((item) => {
            const available = availableViews.includes(item);
            const selected = item === view;
            return <button key={item} type="button" role="tab" aria-selected={selected} disabled={!available} onClick={() => { if (available) onViewChange(item); }} className={cn("shrink-0 rounded-md px-2.5 py-1.5 text-[10.5px] font-medium transition-colors", selected ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-muted/70 hover:text-foreground", "disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent disabled:hover:text-muted-foreground")}>{LABELS[item]}</button>;
          })}
        </div>
      </nav>
      <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
    </section>
  );
}
