import { useCallback, useEffect, useRef, useState } from "react";
import { formatRelativeTime, WorkDetail } from "@altai/agent-ui";
import type {
  WorkAttempt,
  WorkInboxItem,
  WorkItem,
} from "@altai/host-contract";
import { native } from "@/modules/ai/lib/native";
import {
  toWorkDetailModel,
  toWorkGraphModel,
  type WorkGraphModel,
} from "./lib/workDetailProjection";
import { WorkGraphSection } from "./WorkGraphSection";

type LoadStatus = "loading" | "ready" | "error" | "not_found";

type Props = {
  workspacePath: string;
  workspaceName: string;
  workId: string;
  onBack: () => void;
  /** Navigate to another Work in the graph. */
  onOpenWork: (workId: string) => void;
  className?: string;
};

/**
 * The Work detail surface (package 062, PR 2): one Work from the server
 * projections — `work_get`, `work_attempts`, `work_inbox_list`,
 * `work_children` — composed through the detail/graph projections and
 * rendered by the shared `@altai/agent-ui` detail screen. Status, execution
 * phase, and attention stay distinct axes end to end.
 */
export function WorkDetailPanel({
  workspacePath,
  workspaceName,
  workId,
  onBack,
  onOpenWork,
  className,
}: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [work, setWork] = useState<WorkItem | null>(null);
  const [parent, setParent] = useState<WorkItem | null>(null);
  const [children, setChildren] = useState<WorkItem[]>([]);
  const [attempts, setAttempts] = useState<WorkAttempt[]>([]);
  const [inbox, setInbox] = useState<WorkInboxItem[]>([]);
  const generation = useRef(0);

  const refresh = useCallback(async () => {
    const current = ++generation.current;
    setStatus((previous) => (previous === "ready" ? "ready" : "loading"));
    try {
      const nextWork = await native.workGet(workId, workspacePath);
      if (current !== generation.current) return;
      if (!nextWork) {
        setWork(null);
        setStatus("not_found");
        return;
      }
      const [nextAttempts, nextInbox, nextChildren, nextParent] =
        await Promise.all([
          native.workAttempts(workId, workspacePath).catch(() => [] as WorkAttempt[]),
          native
            .workInboxList(workspacePath)
            .catch(() => [] as WorkInboxItem[]),
          native
            .workChildren(workId, workspacePath)
            .catch(() => [] as WorkItem[]),
          nextWork.parentWorkId
            ? native
                .workGet(nextWork.parentWorkId, workspacePath)
                .catch(() => null)
            : Promise.resolve(null),
        ]);
      if (current !== generation.current) return;
      setWork(nextWork);
      setAttempts(nextAttempts);
      setInbox(nextInbox);
      setChildren(nextChildren);
      setParent(nextParent);
      setError(null);
      setStatus("ready");
    } catch (loadError) {
      if (current !== generation.current) return;
      setError(loadError instanceof Error ? loadError.message : String(loadError));
      setStatus("error");
    }
  }, [workId, workspacePath]);

  useEffect(() => {
    void refresh();
    return () => {
      generation.current += 1;
    };
  }, [refresh]);

  const model = work
    ? toWorkDetailModel({ work, attempts, inbox })
    : null;
  const graph: WorkGraphModel | null = work
    ? toWorkGraphModel({ work, parent, children })
    : null;

  return (
    <div
      className={`flex h-full min-h-0 flex-col overflow-hidden bg-card ${className ?? ""}`}
    >
      {model ? (
        <WorkDetail
          className="min-h-0 flex-1"
          status={status === "not_found" ? "not_found" : status}
          title={model.title}
          stateLabel={model.statusLabel}
          attentionLabel={model.attentionLabel}
          projectLabel={workspaceName}
          updatedLabel={formatRelativeTime(model.updatedAtMs)}
          description={model.description}
          acceptanceCriteria={model.acceptanceCriteria}
          blocker={model.blocker}
          attempts={model.attemptRows.map((row) => ({
            id: row.id,
            label: `Attempt ${row.number}`,
            phaseLabel: row.phaseLabel,
          }))}
          onBack={onBack}
          onRetry={() => void refresh()}
          errorMessage={error ?? undefined}
        />
      ) : (
        <WorkDetail
          className="min-h-0 flex-1"
          status={status}
          onBack={onBack}
          onRetry={() => void refresh()}
          errorMessage={error ?? undefined}
        />
      )}
      {graph ? <WorkGraphSection graph={graph} onOpenWork={onOpenWork} /> : null}
    </div>
  );
}
