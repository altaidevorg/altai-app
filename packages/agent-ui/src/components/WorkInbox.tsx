import { cn } from "../lib/cn.js";
import { SurfaceEmptyState } from "./AuxiliarySurface.js";
import { SurfaceLoadingState } from "./SurfaceLoadingState.js";
import { SurfaceInlineError } from "./SurfaceInlineError.js";

export type WorkInboxKind =
  | "review_required"
  | "approval"
  | "question"
  | "failed_attempt"
  | "blocked";

export type WorkInboxRow = {
  id: string;
  workId: string;
  kind: WorkInboxKind;
  title: string;
  why: string;
  ageLabel: string;
};

export type WorkInboxProps = {
  status: "loading" | "ready" | "error";
  rows: WorkInboxRow[];
  onOpenWork: (workId: string) => void;
  onGoToWork?: () => void;
  errorMessage?: string;
  onRetry?: () => void;
  className?: string;
};

const KIND_LABEL: Record<WorkInboxKind, string> = {
  review_required: "Review",
  approval: "Approval",
  question: "Question",
  failed_attempt: "Failed",
  blocked: "Blocked",
};

/**
 * Work OS Inbox (SCREENS.md). Source-backed projection — not a notification store.
 */
export function WorkInbox({
  status,
  rows,
  onOpenWork,
  onGoToWork,
  errorMessage,
  onRetry,
  className,
}: WorkInboxProps) {
  return (
    <div
      className={cn(
        "altai-work-inbox flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-center border-b border-border-subtle px-3 py-2">
        <h2 className="text-[13px] font-semibold text-foreground">Inbox</h2>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {status === "loading" ? (
          <SurfaceLoadingState>Loading Inbox…</SurfaceLoadingState>
        ) : null}
        {status === "error" ? (
          <SurfaceInlineError
            className="m-3"
            message={errorMessage ?? "Inbox failed to load."}
            onDismiss={onRetry}
          />
        ) : null}
        {status === "ready" && rows.length === 0 ? (
          <SurfaceEmptyState
            title="Nothing needs you"
            description="Approvals, questions, and review-ready Work will appear here."
            action={
              onGoToWork ? (
                <button
                  type="button"
                  onClick={onGoToWork}
                  className="inline-flex h-7 items-center rounded-md border border-border px-2.5 text-[11px] font-medium"
                >
                  Go to Work
                </button>
              ) : undefined
            }
            className="border-0 bg-transparent"
          />
        ) : null}
        {status === "ready" && rows.length > 0 ? (
          <ul aria-label="Inbox" className="divide-y divide-border-subtle">
            {rows.map((row) => (
              <li key={row.id}>
                <button
                  type="button"
                  onClick={() => onOpenWork(row.workId)}
                  className="flex w-full flex-col gap-0.5 px-3 py-2.5 text-left hover:bg-muted/50"
                >
                  <span className="flex items-center gap-2 text-[10.5px] text-muted-foreground">
                    <span className="font-medium text-foreground">
                      {KIND_LABEL[row.kind]}
                    </span>
                    <span className="ml-auto tabular-nums">{row.ageLabel}</span>
                  </span>
                  <span className="truncate text-[12px] font-medium text-foreground">
                    {row.title}
                  </span>
                  <span className="truncate text-[11px] text-muted-foreground">
                    {row.why}
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
