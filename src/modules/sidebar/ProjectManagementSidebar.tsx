import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useAgentRunsStore } from "@/modules/ai/store/agentRunsStore";
import type {
  Assignment,
  AssignmentStatus,
} from "@/modules/github/lib/assignments";
import {
  ACTIVE_ASSIGNMENT_STATES,
  useAssignmentsStore,
} from "@/modules/github/store/assignmentsStore";
import { ProjectIntelligencePanel } from "./ProjectIntelligencePanel";
import {
  ArrowRight01Icon,
  Cancel01Icon,
  CheckListIcon,
  PlayIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useState } from "react";

type WorkFilter = "all" | "active" | "approval" | "review" | "failed";

type Props = {
  workspaceName: string;
  onOpenBoard: () => void;
  onCreateWork: () => void;
};

const FILTERS: Array<{
  id: WorkFilter;
  label: string;
  statuses: AssignmentStatus[];
}> = [
  {
    id: "active",
    label: "Active",
    statuses: ["dispatching", "running"],
  },
  {
    id: "approval",
    label: "Approval",
    statuses: ["awaiting-approval"],
  },
  { id: "review", label: "Review", statuses: ["done"] },
  { id: "failed", label: "Failed", statuses: ["failed"] },
];

const STATUS_META: Record<
  AssignmentStatus,
  { label: string; dot: string; text: string }
> = {
  dispatching: {
    label: "Dispatching",
    dot: "bg-amber-500",
    text: "text-amber-500",
  },
  running: {
    label: "Running",
    dot: "bg-emerald-500",
    text: "text-emerald-500",
  },
  "awaiting-approval": {
    label: "Awaiting approval",
    dot: "bg-amber-500",
    text: "text-amber-500",
  },
  done: { label: "Ready for review", dot: "bg-sky-500", text: "text-sky-500" },
  failed: { label: "Failed", dot: "bg-red-500", text: "text-red-500" },
  cancelled: {
    label: "Cancelled",
    dot: "bg-muted-foreground/45",
    text: "text-muted-foreground",
  },
};

export function ProjectManagementSidebar({
  workspaceName,
  onOpenBoard,
  onCreateWork,
}: Props) {
  const assignments = useAssignmentsStore((state) => state.assignments);
  const hydrate = useAssignmentsStore((state) => state.hydrate);
  const cancel = useAssignmentsStore((state) => state.cancel);
  const remove = useAssignmentsStore((state) => state.remove);
  const publishDraftPullRequest = useAssignmentsStore(
    (state) => state.publishDraftPullRequest,
  );
  const applyLocalChanges = useAssignmentsStore(
    (state) => state.applyLocalChanges,
  );
  const runs = useAgentRunsStore((state) => state.runs);
  const switchSession = useChatStore((state) => state.switchSession);
  const openMini = useChatStore((state) => state.openMini);
  const [filter, setFilter] = useState<WorkFilter>("all");

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  const resolvedAssignments = useMemo(
    () =>
      assignments.map((assignment) => ({
        assignment,
        status: resolvedStatus(
          assignment.status,
          runs[assignment.sessionId],
        ),
      })),
    [assignments, runs],
  );

  const filteredAssignments = useMemo(() => {
    const statuses = FILTERS.find((item) => item.id === filter)?.statuses;
    if (!statuses) return resolvedAssignments;
    return resolvedAssignments.filter((item) =>
      statuses.includes(item.status),
    );
  }, [filter, resolvedAssignments]);

  const openAssignment = (assignment: Assignment) => {
    switchSession(assignment.sessionId);
    openMini();
  };

  return (
    <aside className="flex h-full min-h-0 flex-col bg-card/80">
      <header className="shrink-0 border-b border-border/50 px-3 pb-3 pt-3">
        <div className="flex items-center gap-2">
          <HugeiconsIcon
            icon={CheckListIcon}
            size={15}
            strokeWidth={1.8}
            className="shrink-0 text-muted-foreground"
          />
          <span className="min-w-0 flex-1 truncate text-[12px] font-semibold">
            {workspaceName}
          </span>
        </div>
        <div className="mt-2 grid grid-cols-2 gap-1.5">
          <button
            type="button"
            onClick={onOpenBoard}
            className="flex h-8 items-center justify-center gap-1.5 rounded-lg border border-border/55 bg-background/45 text-[10.5px] font-medium transition-colors hover:bg-muted/60"
          >
            Open operations
            <HugeiconsIcon
              icon={ArrowRight01Icon}
              size={11}
              strokeWidth={2}
            />
          </button>
          <button
            type="button"
            onClick={onCreateWork}
            className="flex h-8 items-center justify-center gap-1.5 rounded-lg bg-primary text-[10.5px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
          >
            <HugeiconsIcon icon={PlayIcon} size={11} strokeWidth={2} />
            New task
          </button>
        </div>
      </header>

      <section
        aria-label="Work queues"
        className="shrink-0 border-b border-border/45 px-2.5 py-2.5"
      >
        <div className="grid grid-cols-2 gap-1">
          {FILTERS.map((item) => {
            const count = resolvedAssignments.filter((assignment) =>
              item.statuses.includes(assignment.status),
            ).length;
            const active = filter === item.id;
            return (
              <button
                key={item.id}
                type="button"
                aria-pressed={active}
                onClick={() =>
                  setFilter((current) =>
                    current === item.id ? "all" : item.id,
                  )
                }
                className={cn(
                  "flex h-8 items-center gap-2 rounded-lg px-2 text-[10.5px] font-medium transition-colors",
                  active
                    ? "bg-accent text-foreground"
                    : "bg-foreground/[0.035] text-muted-foreground hover:bg-muted/70 hover:text-foreground",
                )}
              >
                <span className="truncate">{item.label}</span>
                <span className="ml-auto rounded-full bg-foreground/[0.07] px-1.5 text-[9.5px] tabular-nums">
                  {count}
                </span>
              </button>
            );
          })}
        </div>
      </section>

      <ProjectIntelligencePanel />

      <section
        aria-label="Agent assignments"
        className="flex min-h-0 flex-1 flex-col"
      >
        <div className="flex shrink-0 items-center gap-2 px-3 py-2">
          <span className="text-[9.5px] font-semibold uppercase tracking-wide text-muted-foreground/65">
            {filter === "all"
              ? "All work"
              : FILTERS.find((item) => item.id === filter)?.label}
          </span>
          <span className="ml-auto text-[9.5px] tabular-nums text-muted-foreground/55">
            {filteredAssignments.length}
          </span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
          {filteredAssignments.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border/60 px-3 py-5 text-center text-[10.5px] leading-relaxed text-muted-foreground">
              {assignments.length === 0
                ? "No work has been assigned yet. Create a task to start an agent run."
                : "No assignments match this queue."}
            </div>
          ) : (
            <ul className="space-y-1.5">
              {filteredAssignments.map(({ assignment, status }) => (
                <li key={assignment.id}>
                  <AssignmentRow
                    assignment={assignment}
                    status={status}
                    onOpen={() => openAssignment(assignment)}
                    onCancel={() => void cancel(assignment.id)}
                    onRemove={() => void remove(assignment.id)}
                    onPublish={() =>
                      void publishDraftPullRequest(assignment.id)
                    }
                    onApply={() => void applyLocalChanges(assignment.id)}
                  />
                </li>
              ))}
            </ul>
          )}
        </div>
      </section>
    </aside>
  );
}

function resolvedStatus(
  fallback: AssignmentStatus,
  run:
    | {
        completed: boolean;
        outcome?: { kind: string } | null;
        status: string;
      }
    | undefined,
): AssignmentStatus {
  if (
    fallback === "done" ||
    fallback === "failed" ||
    fallback === "cancelled" ||
    !run
  ) {
    return fallback;
  }
  if (run.completed) {
    if (run.outcome?.kind === "completed") return "done";
    if (run.outcome?.kind === "cancelled") return "cancelled";
    return "failed";
  }
  if (run.status === "awaiting-approval") return "awaiting-approval";
  if (run.status === "error") return "failed";
  if (run.status === "thinking" || run.status === "streaming") return "running";
  return fallback;
}

function AssignmentRow({
  assignment,
  status,
  onOpen,
  onCancel,
  onRemove,
  onPublish,
  onApply,
}: {
  assignment: Assignment;
  status: AssignmentStatus;
  onOpen: () => void;
  onCancel: () => void;
  onRemove: () => void;
  onPublish: () => void;
  onApply: () => void;
}) {
  const meta = STATUS_META[status];
  const deliveryBusy =
    assignment.delivery?.status === "publishing" ||
    assignment.delivery?.status === "applying";
  const canPublish =
    status === "done" &&
    assignment.source.kind === "issue" &&
    !!assignment.delivery &&
    assignment.delivery.status !== "draft-pr";
  const canApply =
    status === "done" &&
    assignment.source.kind === "todo" &&
    assignment.origin === "orchestrator" &&
    !!assignment.delivery &&
    assignment.delivery.status !== "applied";

  return (
    <div className="rounded-lg border border-border/50 bg-background/35 p-2">
      <button
        type="button"
        onClick={onOpen}
        className="group flex w-full items-start gap-2 text-left"
      >
        <span
          className={cn(
            "mt-1 size-2 shrink-0 rounded-full",
            meta.dot,
            ACTIVE_ASSIGNMENT_STATES.includes(status) && "animate-pulse",
          )}
        />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[11px] font-medium text-foreground">
            {assignment.title}
          </span>
          <span
            className={cn(
              "mt-0.5 block truncate text-[9.5px]",
              meta.text,
            )}
          >
            {meta.label}
            {assignment.runConfig?.branchName
              ? ` · ${assignment.runConfig.branchName}`
              : ""}
          </span>
        </span>
        <HugeiconsIcon
          icon={ArrowRight01Icon}
          size={11}
          strokeWidth={2}
          className="mt-0.5 shrink-0 text-muted-foreground/35 transition-transform group-hover:translate-x-0.5"
        />
      </button>
      <div className="mt-1.5 flex items-center gap-1">
        {ACTIVE_ASSIGNMENT_STATES.includes(status) ? (
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md px-1.5 py-1 text-[9.5px] font-medium text-red-500 transition-colors hover:bg-red-500/10"
          >
            Cancel
          </button>
        ) : canPublish ? (
          <button
            type="button"
            onClick={onPublish}
            disabled={deliveryBusy}
            className="inline-flex items-center gap-1 rounded-md px-1.5 py-1 text-[9.5px] font-medium text-violet-500 transition-colors hover:bg-violet-500/10 disabled:opacity-50"
          >
            {deliveryBusy ? <Spinner className="size-2.5" /> : null}
            {assignment.delivery?.status === "failed"
              ? "Retry draft PR"
              : "Create draft PR"}
          </button>
        ) : canApply ? (
          <button
            type="button"
            onClick={onApply}
            disabled={deliveryBusy}
            className="inline-flex items-center gap-1 rounded-md px-1.5 py-1 text-[9.5px] font-medium text-emerald-500 transition-colors hover:bg-emerald-500/10 disabled:opacity-50"
          >
            {deliveryBusy ? <Spinner className="size-2.5" /> : null}
            {assignment.delivery?.status === "failed"
              ? "Retry apply"
              : "Apply changes"}
          </button>
        ) : (
          <button
            type="button"
            onClick={onOpen}
            className="rounded-md px-1.5 py-1 text-[9.5px] font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            {status === "awaiting-approval" ? "Review request" : "Open run"}
          </button>
        )}
        {!ACTIVE_ASSIGNMENT_STATES.includes(status) ? (
          <button
            type="button"
            aria-label="Remove assignment"
            title="Remove assignment"
            onClick={onRemove}
            className="ml-auto flex size-5 items-center justify-center rounded text-muted-foreground/45 transition-colors hover:bg-muted hover:text-foreground"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={2} />
          </button>
        ) : null}
      </div>
    </div>
  );
}
