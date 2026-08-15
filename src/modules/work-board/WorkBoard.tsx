import { cn } from "@/lib/utils";
import type { WorkAttemptPhase, WorkInboxKind, WorkState } from "@altai/host-contract";
import { BOARD_COLUMNS, type WorkBoardRow } from "./lib/workBoardProjection";

type Props = {
  status: "loading" | "ready" | "error";
  rows: WorkBoardRow[];
  onOpenWork: (id: string) => void;
  onNewWork: () => void;
  onOpenInbox?: () => void;
  errorMessage?: string;
  onRetry?: () => void;
  title?: string;
  className?: string;
};

const PHASE_DOT: Record<WorkAttemptPhase, string> = {
  queued: "bg-zinc-400",
  running: "bg-sky-500",
  waiting: "bg-amber-500",
  succeeded: "bg-emerald-500",
  failed: "bg-red-500",
  cancelled: "bg-zinc-400",
};

const ATTENTION_CHIP: Record<WorkInboxKind, string> = {
  approval: "bg-warning/15 text-warning",
  review_required: "bg-sky-500/15 text-sky-500",
  failed_attempt: "bg-red-500/15 text-red-500",
  blocked: "bg-red-500/15 text-red-500",
  question: "bg-violet-500/15 text-violet-500",
};

function columnsFor(rows: readonly WorkBoardRow[]): {
  state: WorkState;
  statusLabel: string;
  rows: WorkBoardRow[];
}[] {
  return BOARD_COLUMNS.map((state) => ({
    state,
    statusLabel: state.replace(/_/g, " "),
    rows: rows.filter((row) => row.status === state),
  })).filter((column) => column.rows.length > 0);
}

/**
 * Work OS board (package 062, PR 1): Work as columns by status, each card
 * carrying its execution phase and attention as distinct chips — the gate's
 * three axes stay separate at render, matching the projection's typed fields.
 */
export function WorkBoard({
  status,
  rows,
  onOpenWork,
  onNewWork,
  onOpenInbox,
  errorMessage,
  onRetry,
  title = "Work board",
  className,
}: Props) {
  return (
    <div
      className={cn(
        "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-center gap-2 border-b border-border-subtle px-3 py-2">
        <h2 className="min-w-0 flex-1 text-[13px] font-semibold text-foreground">
          {title}
        </h2>
        <button
          type="button"
          onClick={onNewWork}
          className="inline-flex h-7 shrink-0 items-center rounded-md bg-foreground px-2.5 text-[11px] font-medium text-background transition-opacity hover:opacity-90"
        >
          New Work
        </button>
      </header>

      <div className="min-h-0 flex-1 overflow-x-auto overflow-y-hidden">
        {status === "loading" ? (
          <p className="px-3 py-6 text-[11px] text-muted-foreground">
            Loading Work…
          </p>
        ) : null}
        {status === "error" ? (
          <div className="m-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-[11px] text-red-500">
            <p>{errorMessage ?? "Work board failed to load."}</p>
            {onRetry ? (
              <button
                type="button"
                onClick={onRetry}
                className="mt-1 underline underline-offset-2"
              >
                Retry
              </button>
            ) : null}
          </div>
        ) : null}
        {status === "ready" && rows.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
            <p className="text-[12px] font-medium text-foreground">
              Nothing on the board
            </p>
            <p className="text-[11px] text-muted-foreground">
              Create Work or check the Inbox for what needs you.
            </p>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={onNewWork}
                className="inline-flex h-7 items-center rounded-md border border-border px-2.5 text-[11px] font-medium text-foreground hover:bg-muted"
              >
                New Work
              </button>
              {onOpenInbox ? (
                <button
                  type="button"
                  onClick={onOpenInbox}
                  className="inline-flex h-7 items-center rounded-md px-2.5 text-[11px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
                >
                  Inbox
                </button>
              ) : null}
            </div>
          </div>
        ) : null}
        {status === "ready" && rows.length > 0 ? (
          <div
            role="list"
            aria-label="Work by status"
            className="flex h-full min-w-0 items-stretch gap-px bg-border-subtle p-px"
          >
            {columnsFor(rows).map((column) => (
              <section
                key={column.state}
                aria-label={column.statusLabel}
                className="flex min-w-[180px] flex-1 flex-col overflow-hidden bg-card"
              >
                <header className="flex shrink-0 items-baseline gap-1.5 border-b border-border-subtle px-3 py-2">
                  <h3 className="text-[11px] font-semibold text-foreground">
                    {column.statusLabel}
                  </h3>
                  <span className="text-[10px] tabular-nums text-muted-foreground">
                    {column.rows.length}
                  </span>
                </header>
                <ul className="min-h-0 flex-1 space-y-1.5 overflow-y-auto p-2">
                  {column.rows.map((row) => (
                    <li key={row.id}>
                      <button
                        type="button"
                        onClick={() => onOpenWork(row.id)}
                        className="flex w-full flex-col gap-1 rounded-md border border-border-subtle bg-background px-2.5 py-2 text-left transition-colors hover:bg-muted/50"
                      >
                        <span className="truncate text-[12px] font-medium text-foreground">
                          {row.title}
                        </span>
                        <span className="flex flex-wrap items-center gap-1">
                          {row.executionPhase ? (
                            <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground">
                              <span
                                aria-hidden="true"
                                className={cn(
                                  "size-1.5 rounded-full",
                                  PHASE_DOT[row.executionPhase],
                                )}
                              />
                              {row.phaseLabel}
                            </span>
                          ) : null}
                          {row.attention ? (
                            <span
                              className={cn(
                                "rounded-full px-1.5 py-px text-[9.5px] font-medium",
                                ATTENTION_CHIP[row.attention],
                              )}
                            >
                              {row.attentionLabel}
                            </span>
                          ) : null}
                        </span>
                        <span className="text-[10px] tabular-nums text-muted-foreground">
                          {row.updatedLabel}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
