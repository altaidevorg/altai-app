import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import { SurfaceEmptyState } from "./AuxiliarySurface.js";
import { SurfaceLoadingState } from "./SurfaceLoadingState.js";
import { SurfaceInlineError } from "./SurfaceInlineError.js";

export type WorkListFilterId = "my_active" | "review" | "backlog" | "done";

export type WorkListRow = {
  id: string;
  title: string;
  projectLabel: string;
  stateLabel: string;
  attemptLabel: string;
  updatedLabel: string;
};

export type WorkListProps = {
  status: "loading" | "ready" | "error";
  filter: WorkListFilterId;
  onFilterChange: (filter: WorkListFilterId) => void;
  rows: WorkListRow[];
  onOpenWork: (id: string) => void;
  onNewWork: () => void;
  onOpenInbox?: () => void;
  errorMessage?: string;
  onRetry?: () => void;
  className?: string;
};

const FILTERS: { id: WorkListFilterId; label: string }[] = [
  { id: "my_active", label: "My active" },
  { id: "review", label: "Review" },
  { id: "backlog", label: "Backlog" },
  { id: "done", label: "Done" },
];

const EMPTY: Record<
  WorkListFilterId,
  { title: string; description: string; showInbox?: boolean }
> = {
  my_active: {
    title: "Nothing active",
    description: "Create Work or check Inbox.",
    showInbox: true,
  },
  review: {
    title: "No Work waiting for review",
    description: "Completed attempts that need Accept or Return will show here.",
  },
  backlog: {
    title: "Backlog is empty",
    description: "Capture an outcome with New Work.",
  },
  done: {
    title: "No completed Work yet",
    description: "Accepted Work will appear here.",
  },
};

/**
 * Work OS list screen (SCREENS.md). Presentational — host owns data and routes.
 */
export function WorkList({
  status,
  filter,
  onFilterChange,
  rows,
  onOpenWork,
  onNewWork,
  onOpenInbox,
  errorMessage,
  onRetry,
  className,
}: WorkListProps) {
  return (
    <div
      className={cn(
        "altai-work-list flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-center gap-2 border-b border-border-subtle px-3 py-2">
        <h2 className="min-w-0 flex-1 text-[13px] font-semibold text-foreground">
          Work
        </h2>
        <button
          type="button"
          onClick={onNewWork}
          className="inline-flex h-7 shrink-0 items-center rounded-md bg-foreground px-2.5 text-[11px] font-medium text-background transition-opacity hover:opacity-90"
        >
          New Work
        </button>
      </header>

      <div
        role="tablist"
        aria-label="Work filters"
        className="flex shrink-0 gap-1 overflow-x-auto border-b border-border-subtle px-3 py-2"
      >
        {FILTERS.map((item) => {
          const selected = item.id === filter;
          return (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={selected}
              onClick={() => onFilterChange(item.id)}
              className={cn(
                "shrink-0 rounded-md px-2.5 py-1 text-[10.5px] font-medium transition-colors",
                selected
                  ? "bg-accent text-foreground"
                  : "text-muted-foreground hover:bg-muted/70 hover:text-foreground",
              )}
            >
              {item.label}
            </button>
          );
        })}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {status === "loading" ? (
          <SurfaceLoadingState>Loading Work…</SurfaceLoadingState>
        ) : null}
        {status === "error" ? (
          <SurfaceInlineError
            className="m-3"
            message={errorMessage ?? "Work list failed to load."}
            onDismiss={onRetry}
          />
        ) : null}
        {status === "ready" && rows.length === 0 ? (
          <EmptyWorkList
            filter={filter}
            onNewWork={onNewWork}
            onOpenInbox={onOpenInbox}
          />
        ) : null}
        {status === "ready" && rows.length > 0 ? (
          <ul aria-label="Work items" className="divide-y divide-border-subtle">
            {rows.map((row) => (
              <li key={row.id}>
                <button
                  type="button"
                  onClick={() => onOpenWork(row.id)}
                  className="flex w-full flex-col gap-0.5 px-3 py-2.5 text-left transition-colors hover:bg-muted/50"
                >
                  <span className="truncate text-[12px] font-medium text-foreground">
                    {row.title}
                  </span>
                  <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-[10.5px] text-muted-foreground">
                    <span className="truncate">{row.projectLabel}</span>
                    <span>{row.stateLabel}</span>
                    <span>{row.attemptLabel}</span>
                    <span className="ml-auto tabular-nums">{row.updatedLabel}</span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </div>
    </div>
  );
}

function EmptyWorkList({
  filter,
  onNewWork,
  onOpenInbox,
}: {
  filter: WorkListFilterId;
  onNewWork: () => void;
  onOpenInbox?: () => void;
}) {
  const copy = EMPTY[filter];
  let actions: ReactNode = (
    <button
      type="button"
      onClick={onNewWork}
      className="inline-flex h-7 items-center rounded-md border border-border px-2.5 text-[11px] font-medium text-foreground hover:bg-muted"
    >
      New Work
    </button>
  );
  if (copy.showInbox && onOpenInbox) {
    actions = (
      <div className="flex gap-2">
        {actions}
        <button
          type="button"
          onClick={onOpenInbox}
          className="inline-flex h-7 items-center rounded-md px-2.5 text-[11px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          Inbox
        </button>
      </div>
    );
  }
  return (
    <SurfaceEmptyState
      title={copy.title}
      description={copy.description}
      action={actions}
      className="border-0 bg-transparent"
    />
  );
}
