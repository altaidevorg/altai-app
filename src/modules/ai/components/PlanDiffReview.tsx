import { Button } from "@/components/ui/button";
import {
  Cancel01Icon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { native, type CheckpointInfo } from "../lib/native";
import { usePlanStore, type AppliedPlanEdit } from "../store/planStore";
import { useChatStore } from "../store/chatStore";
import {
  AuxiliarySurface,
  PlanRow,
  ReviewHistory,
  type ReviewHistoryItem,
} from "@altai/agent-ui";

function diffStats(
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

export function PlanDiffReview({
  open = false,
  autoOpen = true,
  onClose,
}: {
  /** Opens the review centre even when no plan edits are pending. */
  open?: boolean;
  /** Pending plan edits normally interrupt the chat for a deliberate review. */
  autoOpen?: boolean;
  onClose?: () => void;
}) {
  const queue = usePlanStore((s) => s.queue);
  const applied = usePlanStore((s) => s.applied);
  const removeOne = usePlanStore((s) => s.removeOne);
  const clear = usePlanStore((s) => s.clear);
  const applyOne = usePlanStore((s) => s.applyOne);
  const applyAll = usePlanStore((s) => s.applyAll);
  const addActivity = useChatStore((s) => s.addActivity);
  const [busy, setBusy] = useState(false);
  const [applyingId, setApplyingId] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [checkpoints, setCheckpoints] = useState<CheckpointInfo[]>([]);

  useEffect(() => {
    if (!open && queue.length === 0) return;
    let mounted = true;
    void native.checkpointList().then((items) => {
      if (mounted) setCheckpoints(items);
    });
    return () => {
      mounted = false;
    };
  }, [open, queue.length]);

  if (!open && (!autoOpen || queue.length === 0)) return null;
  const historyCount = applied.length + checkpoints.length;

  const onApply = async () => {
    setBusy(true);
    try {
      const results = await applyAll();
      const failed = results.filter((r) => !r.ok);
      if (failed.length) {
        console.error("plan apply failures:", failed);
        setFeedback(`${failed.length} change${failed.length === 1 ? "" : "s"} could not be applied. They remain in review.`);
        addActivity({
          label: "Some reviewed changes could not be applied",
          detail: `${failed.length} change${failed.length === 1 ? "" : "s"} remain queued`,
          tone: "error",
        });
      } else {
        setFeedback(`${results.length} change${results.length === 1 ? "" : "s"} applied. A restore point is available in Undo.`);
        addActivity({
          label: `Applied ${results.length} reviewed change${results.length === 1 ? "" : "s"}`,
          detail: "Restore points are available in Undo",
          tone: "success",
        });
      }
    } finally {
      setBusy(false);
    }
  };

  const onApplyOne = async (id: string) => {
    setApplyingId(id);
    setFeedback(null);
    try {
      const result = await applyOne(id);
      if (!result) return;
      if (result.ok) {
        setFeedback("Change applied. A restore point is available in Undo.");
        addActivity({
          label: "Applied a reviewed change",
          detail: "Restore point available in Undo",
          tone: "success",
        });
      } else {
        setFeedback(`Could not apply change: ${result.error ?? "Unknown error"}`);
        addActivity({
          label: "Reviewed change could not be applied",
          detail: result.error,
          tone: "error",
        });
      }
    } finally {
      setApplyingId(null);
    }
  };

  return (
    <AuxiliarySurface
      title="Change review"
      subtitle={
        queue.length
          ? `${queue.length} pending change${queue.length === 1 ? "" : "s"}`
          : historyCount
            ? `${historyCount} restorable change${historyCount === 1 ? "" : "s"}`
            : "No changes to review"
      }
      onClose={onClose}
      actions={
        queue.length ? (
          <div className="flex items-center gap-1.5">
            <Button
              type="button"
              size="sm"
              variant="ghost"
              className="h-7 gap-1.5 text-[11px] hover:bg-destructive/10 hover:text-destructive"
              onClick={() => clear()}
              disabled={busy}
            >
              <HugeiconsIcon icon={Cancel01Icon} size={12} strokeWidth={2} />
              Discard all
            </Button>
            <Button
              type="button"
              size="sm"
              className="h-7 gap-1.5 text-[11px]"
              onClick={() => void onApply()}
              disabled={busy}
            >
              <HugeiconsIcon icon={Tick02Icon} size={12} strokeWidth={2} />
              Apply {queue.length}
            </Button>
          </div>
        ) : undefined
      }
    >
      {feedback ? (
        <div className="border-b border-border-subtle bg-muted/25 px-3 py-1.5 text-[10.5px] text-muted-foreground">
          {feedback}
        </div>
      ) : null}
      <div className="flex flex-1 flex-col gap-3 overflow-auto p-3">
        {queue.length ? <section>
          <div className="mb-1.5 px-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">Awaiting your decision</div>
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
            stats={q.kind === "create_directory" ? null : diffStats(q.originalContent, q.proposedContent)}
            busy={busy || applyingId === q.id}
            onOpenDiff={() => {
              if (q.kind === "create_directory") return;
              window.dispatchEvent(
                new CustomEvent("altai:plan-review-diff", { detail: q }),
              );
            }}
            onApply={() => void onApplyOne(q.id)}
            onReject={() => removeOne(q.id)}
          />
          ))}
          </ul>
        </section> : null}
        <ReviewHistoryBridge
          items={checkpoints}
          applied={applied}
          onCheckpointsChange={setCheckpoints}
        />
        {!queue.length && !historyCount ? <div className="rounded-md border border-dashed border-border/60 px-4 py-8 text-center text-[11px] leading-relaxed text-muted-foreground">When the agent proposes a plan or edits a file, it will appear here with a safe restore option.</div> : null}
      </div>
    </AuxiliarySurface>
  );
}

function ReviewHistoryBridge({
  items,
  applied,
  onCheckpointsChange,
}: {
  items: CheckpointInfo[];
  applied: AppliedPlanEdit[];
  onCheckpointsChange: (items: CheckpointInfo[]) => void;
}) {
  const restoreApplied = usePlanStore((s) => s.restoreApplied);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!items.length && !applied.length) return null;

  const rows: ReviewHistoryItem[] = [
    ...[...applied].reverse().map((item) => ({
      id: `plan-${item.id}`,
      path: item.path,
      detail: `Accepted review · ${item.isNewFile ? "remove new file" : "restore prior content"}`,
    })),
    ...items.map((item) => ({
      id: item.id,
      path: item.path,
      detail: `${item.label} · ${new Date(item.createdMs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`,
    })),
  ];

  const onRestore = async (rowId: string) => {
    if (restoring) return;
    setError(null);
    setRestoring(rowId);
    try {
      if (rowId.startsWith("plan-")) {
        const id = rowId.slice("plan-".length);
        const result = await restoreApplied(id);
        if (result && !result.ok) {
          setError(result.error ?? "Could not restore change.");
        }
      } else {
        await native.checkpointRestore(rowId);
        onCheckpointsChange(await native.checkpointList());
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestoring(null);
    }
  };

  return (
    <ReviewHistory
      items={rows}
      restoringId={restoring}
      error={error}
      onRestore={(id) => void onRestore(id)}
    />
  );
}


