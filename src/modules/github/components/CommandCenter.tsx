import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/modules/ai/store/chatStore";
import {
  useAgentRunsStore,
  type RunState,
} from "@/modules/ai/store/agentRunsStore";
import { describeTerminalOutcomeAttention } from "@/modules/ai/lib/agentEventBridge";
import { useTodosStore } from "@/modules/ai/store/todoStore";
import type { Todo } from "@/modules/ai/lib/todos";
import type { Assignment, AssignmentStatus } from "@/modules/github/lib/assignments";
import {
  ACTIVE_ASSIGNMENT_STATES,
  useAssignmentsStore,
} from "@/modules/github/store/assignmentsStore";
import { OrchestrationControlCenter } from "@/modules/orchestration/OrchestrationControlCenter";
import { useOrchestrationStore } from "@/modules/orchestration";
import {
  OperationsOverview,
  type OperationsOverviewRow,
} from "@altai/agent-ui";
import {
  ArrowRight01Icon,
  CheckmarkCircle01Icon,
  PlayIcon,
  Robot01Icon,
  StopCircleIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo } from "react";
import { useState, type ReactElement } from "react";
import { NewWorkComposer } from "./NewWorkComposer";

type Props = {
  repoRoot: string;
  workspaceName: string;
  onCreateWork: () => void;
  newWorkRequestKey?: number;
};

type ResolvedAssignment = {
  assignment: Assignment;
  status: AssignmentStatus;
  run?: RunState;
};

type AttentionItem = {
  key: string;
  title: string;
  detail: string;
  tone: "warning" | "error" | "success";
  action: string;
  assignment: ResolvedAssignment;
};

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
  done: {
    label: "Ready for delivery",
    dot: "bg-sky-500",
    text: "text-sky-500",
  },
  failed: { label: "Failed", dot: "bg-red-500", text: "text-red-500" },
  cancelled: {
    label: "Cancelled",
    dot: "bg-muted-foreground/45",
    text: "text-muted-foreground",
  },
};

/**
 * Desktop Operations Overview host. Aggregates assignments + runs into
 * shared `OperationsOverview` props and keeps host-only chrome:
 * orchestration strip, new-work composer, delivery actions, local todos.
 */
export function CommandCenter({
  repoRoot,
  workspaceName,
  onCreateWork,
  newWorkRequestKey,
}: Props) {
  const [composerOpen, setComposerOpen] = useState(!!newWorkRequestKey);
  const assignments = useAssignmentsStore((state) => state.assignments);
  const hydrateAssignments = useAssignmentsStore((state) => state.hydrate);
  const cancel = useAssignmentsStore((state) => state.cancel);
  const publishDraftPullRequest = useAssignmentsStore(
    (state) => state.publishDraftPullRequest,
  );
  const applyLocalChanges = useAssignmentsStore(
    (state) => state.applyLocalChanges,
  );
  const runs = useAgentRunsStore((state) => state.runs);
  const activeSessionId = useChatStore((state) => state.activeSessionId);
  const switchSession = useChatStore((state) => state.switchSession);
  const openMini = useChatStore((state) => state.openMini);
  const orchestrationSnapshot = useOrchestrationStore(
    (state) => state.snapshots[repoRoot],
  );
  const taskSessionId =
    orchestrationSnapshot?.status !== "stopped"
      ? (orchestrationSnapshot?.taskSessionId ?? activeSessionId)
      : activeSessionId;
  const todos = useTodosStore((state) =>
    taskSessionId ? state.bySession[taskSessionId] : undefined,
  );
  const hydrateTodos = useTodosStore((state) => state.hydrate);

  useEffect(() => {
    void hydrateAssignments();
  }, [hydrateAssignments]);

  useEffect(() => {
    if (taskSessionId) void hydrateTodos(taskSessionId);
  }, [hydrateTodos, taskSessionId]);

  useEffect(() => {
    if (newWorkRequestKey) setComposerOpen(true);
  }, [newWorkRequestKey]);

  const resolved = useMemo<ResolvedAssignment[]>(
    () =>
      assignments.map((assignment) => ({
        assignment,
        run: runs[assignment.sessionId],
        status: resolveStatus(assignment.status, runs[assignment.sessionId]),
      })),
    [assignments, runs],
  );

  const activeRuns = resolved.filter((item) =>
    ACTIVE_ASSIGNMENT_STATES.includes(item.status),
  );
  const attention = useMemo(() => buildAttention(resolved), [resolved]);
  const delivery = resolved.filter(isReadyForDelivery);
  const recent = [...resolved]
    .sort(
      (left, right) =>
        right.assignment.updatedAt - left.assignment.updatedAt,
    )
    .slice(0, 5);
  const pendingTodos = (todos ?? []).filter(
    (todo) => todo.status !== "completed",
  );
  const failedChecks = resolved.filter((item) =>
    item.run?.verifications.some((check) => check.status === "failed"),
  ).length;
  const queuedCount = Math.max(
    0,
    (orchestrationSnapshot?.retryingCount ?? 0) +
      (orchestrationSnapshot?.claimingCount ?? 0),
  );

  const openAssignment = (item: ResolvedAssignment) => {
    switchSession(item.assignment.sessionId);
    openMini();
  };

  const overviewAttention: OperationsOverviewRow[] = attention
    .slice(0, 8)
    .map((item) => ({
      id: item.key,
      title: item.title,
      detail: item.detail,
      statusLabel: item.action,
      tone: item.tone === "error" || item.tone === "warning" ? "attention" : "default",
      onOpen: () => openAssignment(item.assignment),
    }));

  const overviewProgressing: OperationsOverviewRow[] = activeRuns
    .slice(0, 8)
    .map((item) => {
      const run = item.run;
      const tokens = run ? run.tokens.input + run.tokens.output : 0;
      const meta = STATUS_META[item.status];
      return {
        id: item.assignment.id,
        title: item.assignment.title,
        detail: [
          sourceLabel(item.assignment),
          run?.step ?? meta.label,
          tokens > 0 ? `${formatTokens(tokens)} tok` : null,
          item.assignment.sessionId === activeSessionId ? "Focused" : null,
        ]
          .filter(Boolean)
          .join(" · "),
        statusLabel: meta.label,
        onOpen: () => openAssignment(item),
        actions: (
          <button
            type="button"
            aria-label={`Cancel ${item.assignment.title}`}
            title="Cancel run"
            onClick={() => void cancel(item.assignment.id)}
            className="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground/55 transition-colors hover:bg-destructive/10 hover:text-destructive"
          >
            <HugeiconsIcon icon={StopCircleIcon} size={13} strokeWidth={1.8} />
          </button>
        ),
      };
    });

  return (
    <div className="flex h-full min-h-0 w-full flex-col overflow-hidden">
      <div className="shrink-0 border-b border-border/50 px-4 pb-3 pt-3">
        <div className="flex items-center gap-2">
          <HugeiconsIcon
            icon={Robot01Icon}
            size={16}
            strokeWidth={1.75}
            className="shrink-0 text-primary"
          />
          <div className="min-w-0 flex-1">
            <h1 className="truncate text-[14px] font-semibold text-foreground">
              Project Operations
            </h1>
            <p className="truncate text-[10.5px] text-muted-foreground">
              {workspaceName} · command center
            </p>
          </div>
          <Button
            size="xs"
            className="h-7 gap-1.5 text-[10.5px]"
            onClick={onCreateWork}
          >
            <HugeiconsIcon icon={PlayIcon} size={11} strokeWidth={2} />
            New work
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3">
        <div className="mb-3">
          <OrchestrationControlCenter
            workspaceKey={repoRoot}
            taskSessionId={taskSessionId}
          />
        </div>
        {composerOpen ? (
          <NewWorkComposer
            taskSessionId={taskSessionId}
            onClose={() => setComposerOpen(false)}
          />
        ) : null}
        {!composerOpen ? (
          <div className="mb-3 flex items-center justify-between rounded-lg border border-dashed border-border/55 bg-background/30 px-3 py-2">
            <p className="text-[10px] text-muted-foreground/70">
              Turn an idea into a run or a queued work item.
            </p>
            <Button
              size="xs"
              variant="outline"
              className="h-7 text-[10px]"
              onClick={() => {
                setComposerOpen(true);
                onCreateWork();
              }}
            >
              Create work
            </Button>
          </div>
        ) : null}

        <OperationsOverview
          status="ready"
          className="px-0 py-0"
          metrics={[
            { label: "Attention", value: String(attention.length) },
            { label: "Running", value: String(activeRuns.length) },
            { label: "Queued", value: String(queuedCount) },
            { label: "Delivery", value: String(delivery.length) },
          ]}
          attention={overviewAttention}
          progressing={overviewProgressing}
          attentionEmptyLabel="Approvals, failed checks, and blocked runs will appear here."
          progressingEmptyLabel="Create a task and dispatch it directly to an agent."
        />

        <SectionHeader
          label="Ready for delivery"
          count={delivery.length}
          action={
            failedChecks > 0 ? `${failedChecks} checks failed` : undefined
          }
        />
        {delivery.length > 0 ? (
          <div className="space-y-1.5">
            {delivery.slice(0, 5).map((item) => (
              <DeliveryRow
                key={item.assignment.id}
                item={item}
                onOpen={() => openAssignment(item)}
                onPublish={() => void publishDraftPullRequest(item.assignment.id)}
                onApply={() => void applyLocalChanges(item.assignment.id)}
              />
            ))}
          </div>
        ) : (
          <EmptySection
            icon={ArrowRight01Icon}
            title="Nothing ready to deliver"
            description="Completed work with a worktree or draft changes will appear here."
          />
        )}

        <SectionHeader
          label="Up next"
          count={pendingTodos.length}
          action="Work list"
        />
        {pendingTodos.length > 0 ? (
          <div className="space-y-1.5">
            {pendingTodos.slice(0, 5).map((todo) => (
              <TodoRow key={todo.id} todo={todo} />
            ))}
          </div>
        ) : (
          <EmptySection
            icon={PlayIcon}
            title="No queued local work"
            description="Use New work to add a task to the local workflow."
            action={onCreateWork}
          />
        )}

        <SectionHeader label="Recent activity" count={recent.length} />
        <div className="divide-y divide-border/40 rounded-lg border border-border/45">
          {recent.length > 0 ? (
            recent.map((item) => (
              <button
                key={item.assignment.id}
                type="button"
                onClick={() => openAssignment(item)}
                className="flex w-full items-center gap-2 px-2.5 py-2 text-left transition-colors hover:bg-foreground/[0.035]"
              >
                <span
                  className={cn(
                    "size-1.5 shrink-0 rounded-full",
                    STATUS_META[item.status].dot,
                  )}
                />
                <span className="min-w-0 flex-1 truncate text-[10.5px] text-foreground/85">
                  {item.assignment.title}
                </span>
                <span className="shrink-0 text-[9.5px] text-muted-foreground/55">
                  {STATUS_META[item.status].label}
                </span>
              </button>
            ))
          ) : (
            <p className="px-3 py-3 text-[10.5px] text-muted-foreground">
              Activity will appear as work moves through the workflow.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function resolveStatus(
  fallback: AssignmentStatus,
  run?: RunState,
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

function buildAttention(items: ResolvedAssignment[]): AttentionItem[] {
  const out: AttentionItem[] = [];
  for (const item of items) {
    if (item.status === "awaiting-approval") {
      out.push({
        key: `${item.assignment.id}:approval`,
        title: item.assignment.title,
        detail: item.run?.step ?? "The agent is waiting for a decision.",
        tone: "warning",
        action: "Review request",
        assignment: item,
      });
    }
    const failed = item.run?.verifications.find(
      (check) => check.status === "failed",
    );
    if (failed) {
      out.push({
        key: `${item.assignment.id}:check:${failed.id}`,
        title: item.assignment.title,
        detail: failed.label,
        tone: "error",
        action: "Inspect checks",
        assignment: item,
      });
    }
    if (item.status === "failed" && !failed) {
      out.push({
        key: `${item.assignment.id}:failed`,
        title: item.assignment.title,
        detail:
          item.run?.failures[0] ??
          describeTerminalOutcomeAttention(item.run?.outcome) ??
          "The run failed.",
        tone: "error",
        action: "Open failure",
        assignment: item,
      });
    }
  }
  return out.sort((left, right) => toneRank(left.tone) - toneRank(right.tone));
}

function toneRank(tone: AttentionItem["tone"]): number {
  return tone === "error" ? 0 : tone === "warning" ? 1 : 2;
}

function isReadyForDelivery(item: ResolvedAssignment): boolean {
  if (item.status !== "done" || !item.assignment.delivery) return false;
  return (
    item.assignment.delivery.status !== "applied" &&
    item.assignment.delivery.status !== "draft-pr"
  );
}

function sourceLabel(assignment: Assignment): string {
  if (assignment.source.kind === "issue")
    return `Issue #${assignment.source.number}`;
  if (assignment.source.kind === "pr") return `PR #${assignment.source.number}`;
  if (assignment.source.kind === "task") return "Background task";
  return "Local todo";
}

function SectionHeader({
  label,
  count,
  action,
}: {
  label: string;
  count: number;
  action?: string;
}) {
  return (
    <div className="mt-4 flex items-center gap-2 pb-1.5">
      <h2 className="text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground/75">
        {label}
      </h2>
      <span className="rounded-full bg-foreground/[0.07] px-1.5 text-[9.5px] tabular-nums text-muted-foreground">
        {count}
      </span>
      {action ? (
        <span className="ml-auto text-[9.5px] text-muted-foreground/55">
          {action}
        </span>
      ) : null}
    </div>
  );
}

function DeliveryRow({
  item,
  onOpen,
  onPublish,
  onApply,
}: {
  item: ResolvedAssignment;
  onOpen: () => void;
  onPublish: () => void;
  onApply: () => void;
}) {
  const delivery = item.assignment.delivery;
  if (!delivery) return null;
  const isIssue = item.assignment.source.kind === "issue";
  const action = isIssue ? onPublish : onApply;
  const retry = delivery.status === "failed";
  const label = isIssue
    ? retry
      ? "Retry PR"
      : "Draft PR"
    : retry
      ? "Retry apply"
      : "Apply";
  const busy =
    delivery.status === "publishing" || delivery.status === "applying";
  return (
    <div className="flex items-center gap-2 rounded-lg border border-border/50 bg-card/35 px-2.5 py-2">
      <HugeiconsIcon
        icon={CheckmarkCircle01Icon}
        size={14}
        strokeWidth={1.8}
        className="shrink-0 text-emerald-500"
      />
      <button type="button" onClick={onOpen} className="min-w-0 flex-1 text-left">
        <span className="block truncate text-[11px] font-medium text-foreground">
          {item.assignment.title}
        </span>
        <span className="block truncate font-mono text-[9.5px] text-muted-foreground">
          {delivery.branchName} · {sourceLabel(item.assignment)}
        </span>
      </button>
      <Button
        size="xs"
        variant="outline"
        className="h-7 shrink-0 text-[9.5px]"
        disabled={busy}
        onClick={action}
      >
        {busy ? <Spinner className="size-3" /> : label}
      </Button>
    </div>
  );
}

function TodoRow({ todo }: { todo: Todo }) {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-border/45 bg-card/25 px-2.5 py-2">
      <span
        className={cn(
          "size-1.5 shrink-0 rounded-full",
          todo.status === "in_progress"
            ? "bg-amber-500"
            : "bg-muted-foreground/45",
        )}
      />
      <span className="min-w-0 flex-1 truncate text-[10.5px] text-foreground/85">
        {todo.title}
      </span>
      <span className="shrink-0 text-[9.5px] text-muted-foreground/55">
        {todo.status === "in_progress" ? "In progress" : "Backlog"}
      </span>
    </div>
  );
}

function EmptySection({
  icon,
  title,
  description,
  action,
}: {
  icon: typeof CheckmarkCircle01Icon;
  title: string;
  description: string;
  action?: () => void;
}): ReactElement {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-dashed border-border/55 px-2.5 py-2.5">
      <HugeiconsIcon
        icon={icon}
        size={14}
        strokeWidth={1.7}
        className="shrink-0 text-muted-foreground/60"
      />
      <div className="min-w-0 flex-1">
        <p className="text-[10.5px] font-medium text-foreground/80">{title}</p>
        <p className="truncate text-[9.5px] text-muted-foreground">
          {description}
        </p>
      </div>
      {action ? (
        <button
          type="button"
          onClick={action}
          className="shrink-0 text-[9.5px] font-medium text-primary hover:underline"
        >
          Open
        </button>
      ) : null}
    </div>
  );
}

function formatTokens(tokens: number): string {
  return tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : String(tokens);
}
