import { useEffect, useState } from "react";
import {
  PlanDiffReviewPanel,
  ReviewHistory,
  useHostPorts,
  type ReviewHistoryItem,
  composePlanReviewHistoryRows,
  isPlanRestoreRowId,
  planIdFromRestoreRowId,
} from "@altai/agent-ui";
import { native, type CheckpointInfo } from "../lib/native";
import {
  editProposalInputFromQueued,
  usePlanStore,
  type AppliedPlanEdit,
  type PlanApplyResult,
  type QueuedEdit,
} from "../store/planStore";
import { useChatStore } from "../store/chatStore";

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
  const ports = useHostPorts();
  const queue = usePlanStore((s) => s.queue);
  const applied = usePlanStore((s) => s.applied);
  const removeOne = usePlanStore((s) => s.removeOne);
  const clear = usePlanStore((s) => s.clear);
  const recordApplied = usePlanStore((s) => s.recordApplied);
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

  const applyViaReviewPort = async (
    item: QueuedEdit,
  ): Promise<PlanApplyResult> => {
    try {
      await ports.review.applyEditProposal(
        item.id,
        editProposalInputFromQueued(item),
      );
      const recorded = recordApplied(item.id);
      return recorded ?? { id: item.id, ok: true };
    } catch (error) {
      return { id: item.id, ok: false, error: String(error) };
    }
  };

  const onApply = async () => {
    setBusy(true);
    try {
      const pending = usePlanStore.getState().queue.slice();
      const results: PlanApplyResult[] = [];
      for (const item of pending) {
        results.push(await applyViaReviewPort(item));
      }
      const failed = results.filter((r) => !r.ok);
      if (failed.length) {
        console.error("plan apply failures:", failed);
        setFeedback(
          `${failed.length} change${failed.length === 1 ? "" : "s"} could not be applied. They remain in review.`,
        );
        addActivity({
          label: "Some reviewed changes could not be applied",
          detail: `${failed.length} change${failed.length === 1 ? "" : "s"} remain queued`,
          tone: "error",
        });
      } else {
        setFeedback(
          `${results.length} change${results.length === 1 ? "" : "s"} applied. A restore point is available in Undo.`,
        );
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
      const item = usePlanStore.getState().queue.find((q) => q.id === id);
      if (!item) return;
      const result = await applyViaReviewPort(item);
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

  const onRejectOne = async (id: string) => {
    try {
      await ports.review.denyEditProposal(id);
    } catch {
      // Local discard still proceeds if host deny is a no-op failure.
    }
    removeOne(id);
  };

  const onDiscardAll = async () => {
    const ids = usePlanStore.getState().queue.map((q) => q.id);
    for (const id of ids) {
      try {
        await ports.review.denyEditProposal(id);
      } catch {
        /* continue */
      }
    }
    clear();
  };

  return (
    <PlanDiffReviewPanel
      queue={queue}
      historyCount={historyCount}
      feedback={feedback}
      busy={busy}
      applyingId={applyingId}
      onClose={onClose}
      onDiscardAll={() => void onDiscardAll()}
      onApplyAll={() => void onApply()}
      onApplyOne={(id) => void onApplyOne(id)}
      onRejectOne={(id) => void onRejectOne(id)}
      onOpenDiff={(id) => {
        const item = queue.find((q) => q.id === id);
        if (!item || item.kind === "create_directory") return;
        window.dispatchEvent(
          new CustomEvent("altai:plan-review-diff", { detail: item }),
        );
      }}
      history={
        <ReviewHistoryBridge
          items={checkpoints}
          applied={applied}
          onCheckpointsChange={setCheckpoints}
        />
      }
    />
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
  const ports = useHostPorts();
  const restoreApplied = usePlanStore((s) => s.restoreApplied);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!items.length && !applied.length) return null;

  const rows: ReviewHistoryItem[] = composePlanReviewHistoryRows(
    applied,
    items,
    (createdMs) =>
      new Date(createdMs).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      }),
  );

  const onRestore = async (rowId: string) => {
    if (restoring) return;
    setError(null);
    setRestoring(rowId);
    try {
      if (isPlanRestoreRowId(rowId)) {
        const id = planIdFromRestoreRowId(rowId);
        const result = await restoreApplied(id);
        if (result && !result.ok) {
          setError(result.error ?? "Could not restore change.");
        }
      } else {
        await ports.review.restoreCheckpoint(rowId);
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
