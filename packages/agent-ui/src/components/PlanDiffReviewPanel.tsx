import { Cancel01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { AuxiliarySurface } from "./AuxiliarySurface.js";
import { PlanRow } from "./PlanRow.js";

export type PlanDiffReviewQueueItem = {
  id: string;
  path: string;
  kind: string;
  isNewFile: boolean;
  description?: string;
  originalContent: string;
  proposedContent: string;
};

export type PlanDiffReviewPanelProps = {
  queue: PlanDiffReviewQueueItem[];
  /** Restorable history count used for subtitle/empty copy. */
  historyCount?: number;
  feedback?: string | null;
  busy?: boolean;
  applyingId?: string | null;
  onClose?: () => void;
  onDiscardAll?: () => void;
  onApplyAll?: () => void;
  onApplyOne: (id: string) => void;
  onRejectOne: (id: string) => void;
  onOpenDiff: (id: string) => void;
  /** Host renders restore-history bridge (e.g. ReviewHistory + native restore). */
  history?: ReactNode;
};

/**
 * Coarse line-level added/removed counts for plan review rows. Not a true
 * LCS diff — matches Desktop's previous helper.
 */
export function planDiffStats(
  original: string,
  proposed: string,
): { added: number; removed: number } {
  const a = original.split("\n");
  const b = proposed.split("\n");
  const setA = new Set(a);
  const setB = new Set(b);
  let added = 0;
  let removed = 0;
  for (const line of b) if (!setA.has(line)) added++;
  for (const line of a) if (!setB.has(line)) removed++;
  return { added, removed };
}

function subtitleFor(queueLen: number, historyCount: number): string {
  if (queueLen) {
    return `${queueLen} pending change${queueLen === 1 ? "" : "s"}`;
  }
  if (historyCount) {
    return `${historyCount} restorable change${historyCount === 1 ? "" : "s"}`;
  }
  return "No changes to review";
}

/**
 * Change-review centre body: pending plan rows, optional history slot, and
 * empty state. Store/native restore wiring stays on the host.
 */
export function PlanDiffReviewPanel({
  queue,
  historyCount = 0,
  feedback = null,
  busy = false,
  applyingId = null,
  onClose,
  onDiscardAll,
  onApplyAll,
  onApplyOne,
  onRejectOne,
  onOpenDiff,
  history,
}: PlanDiffReviewPanelProps) {
  const actions =
    queue.length > 0 ? (
      <div className="flex items-center gap-1.5">
        <button
          type="button"
          className="inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive disabled:opacity-50"
          onClick={onDiscardAll}
          disabled={busy || !onDiscardAll}
        >
          <HugeiconsIcon icon={Cancel01Icon} size={12} strokeWidth={2} />
          Discard all
        </button>
        <button
          type="button"
          className="inline-flex h-7 items-center gap-1.5 rounded-md bg-primary px-2 text-[11px] font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
          onClick={onApplyAll}
          disabled={busy || !onApplyAll}
        >
          <HugeiconsIcon icon={Tick02Icon} size={12} strokeWidth={2} />
          Apply {queue.length}
        </button>
      </div>
    ) : undefined;

  return (
    <AuxiliarySurface
      title="Change review"
      subtitle={subtitleFor(queue.length, historyCount)}
      onClose={onClose}
      actions={actions}
    >
      {feedback ? (
        <div className="border-b border-border-subtle bg-muted/25 px-3 py-1.5 text-[10.5px] text-muted-foreground">
          {feedback}
        </div>
      ) : null}
      <div className="flex flex-1 flex-col gap-3 overflow-auto p-3">
        {queue.length ? (
          <section>
            <div className="mb-1.5 px-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Awaiting your decision
            </div>
            <ul className="flex flex-col gap-1.5">
              {queue.map((q) => (
                <PlanRow
                  key={q.id}
                  path={q.path}
                  kind={q.kind}
                  isNewFile={q.isNewFile}
                  description={q.description}
                  originalContent={q.originalContent}
                  proposedContent={q.proposedContent}
                  stats={
                    q.kind === "create_directory"
                      ? null
                      : planDiffStats(q.originalContent, q.proposedContent)
                  }
                  busy={busy || applyingId === q.id}
                  onOpenDiff={() => onOpenDiff(q.id)}
                  onApply={() => onApplyOne(q.id)}
                  onReject={() => onRejectOne(q.id)}
                />
              ))}
            </ul>
          </section>
        ) : null}
        {history}
        {!queue.length && !historyCount ? (
          <div className="rounded-md border border-dashed border-border/60 px-4 py-8 text-center text-[11px] leading-relaxed text-muted-foreground">
            When the agent proposes a plan or edits a file, it will appear here
            with a safe restore option.
          </div>
        ) : null}
      </div>
    </AuxiliarySurface>
  );
}
