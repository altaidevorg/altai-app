import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useTodosStore } from "@/modules/ai/store/todoStore";
import { buildTodoSeed } from "@/modules/github/lib/assignments";
import type { GitHubRepoState } from "@/modules/github/lib/capabilities";
import {
  BOARD_COLUMNS,
  type BoardItem,
  type BoardStatus,
  type BoardSource,
  issueToBoardItem,
  pullToBoardItem,
  todoToBoardItem,
} from "@/modules/github/lib/boardModel";
import type { Assignment } from "@/modules/github/lib/assignments";
import { listItems, type GHItem, type RepoSlug } from "@/modules/github/lib/items";
import {
  assignGitHubItem,
  useAssignmentsStore,
} from "@/modules/github/store/assignmentsStore";
import {
  OrchestrationBar,
  useOrchestrationStore,
} from "@/modules/orchestration";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import {
  ArrowReloadHorizontalIcon,
  PlusSignIcon,
  Robot01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { AssignmentsRail } from "./AssignmentsRail";
import {
  BoardCardDetailsSheet,
  type BoardCardDetail,
} from "./BoardCardDetailsSheet";
import { StateBadge } from "./itemBits";

type Props = {
  repoRoot: string;
  slug: RepoSlug | null;
  githubConnected: boolean;
  githubConnectionReady: boolean;
  repoState: GitHubRepoState;
};

type AssignableSource = Exclude<BoardSource, "agent">;

const SOURCE_META: Record<AssignableSource, { label: string; cls: string }> = {
  issue: { label: "Issue", cls: "bg-sky-500/15 text-sky-500" },
  pr: { label: "PR", cls: "bg-violet-500/15 text-violet-400" },
  todo: { label: "Todo", cls: "bg-amber-500/15 text-amber-500" },
};

const ALL_SOURCES: AssignableSource[] = ["issue", "pr", "todo"];
const ACTIVE_ASSIGNMENT_STATUSES = new Set([
  "dispatching",
  "running",
  "awaiting-approval",
]);

function overrideStorageKey(scope: string): string {
  return `altai.project-board.status.${scope}`;
}

function readStatusOverrides(scope: string): Record<string, BoardStatus> {
  try {
    const parsed = JSON.parse(
      window.localStorage.getItem(overrideStorageKey(scope)) ?? "{}",
    ) as Record<string, string>;
    return Object.fromEntries(
      Object.entries(parsed).filter((entry): entry is [string, BoardStatus] =>
        BOARD_COLUMNS.some((column) => column.id === entry[1]),
      ),
    );
  } catch {
    return {};
  }
}

function assignmentKey(
  assignment: Assignment,
  slug: RepoSlug | null,
): string | null {
  if (
    assignment.source.kind === "issue" ||
    assignment.source.kind === "pr"
  ) {
    if (!slug) return null;
    if (
      assignment.source.owner !== slug.owner ||
      assignment.source.repo !== slug.repo
    ) {
      return null;
    }
    return `${assignment.source.kind}-${assignment.source.number}`;
  }
  return assignment.source.kind === "todo"
    ? `todo-${assignment.source.todoId}`
    : null;
}

function assignmentStatus(assignment: Assignment): BoardStatus | null {
  if (ACTIVE_ASSIGNMENT_STATUSES.has(assignment.status)) return "in_progress";
  if (assignment.status === "done") return "review";
  return null;
}

export function OverviewBoard({
  repoRoot,
  slug,
  githubConnected,
  githubConnectionReady,
  repoState,
}: Props) {
  const remoteAvailable = githubConnected && !!slug;
  const storageScope = slug ? `${slug.owner}/${slug.repo}` : repoRoot;
  const [issues, setIssues] = useState<GHItem[]>([]);
  const [pulls, setPulls] = useState<GHItem[]>([]);
  const [loading, setLoading] = useState(remoteAvailable);
  const [error, setError] = useState<string | null>(null);
  const [assignError, setAssignError] = useState<string | null>(null);
  const [reloadTick, setReloadTick] = useState(0);
  const [enabled, setEnabled] = useState<Set<AssignableSource>>(
    () => new Set(ALL_SOURCES),
  );
  const [creatingTodo, setCreatingTodo] = useState(false);
  const [todoDraft, setTodoDraft] = useState("");
  const [dragItem, setDragItem] = useState<string | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [statusOverrides, setStatusOverrides] = useState<
    Record<string, BoardStatus>
  >(() => readStatusOverrides(storageScope));

  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const orchestrationSnapshot = useOrchestrationStore(
    (state) => state.snapshots[repoRoot],
  );
  const todoSessionId =
    orchestrationSnapshot?.status !== "stopped"
      ? orchestrationSnapshot?.taskSessionId ?? activeSessionId
      : activeSessionId;
  const todos = useTodosStore((s) =>
    todoSessionId ? s.bySession[todoSessionId] : undefined,
  );
  const hydrateTodos = useTodosStore((s) => s.hydrate);
  const addTodo = useTodosStore((s) => s.addTodo);
  const updateTodoStatus = useTodosStore((s) => s.updateTodoStatus);

  const assignments = useAssignmentsStore((s) => s.assignments);
  const assign = useAssignmentsStore((s) => s.assign);

  useEffect(() => {
    setStatusOverrides(readStatusOverrides(storageScope));
    setSelectedKey(null);
  }, [storageScope]);

  useEffect(() => {
    if (todoSessionId) void hydrateTodos(todoSessionId);
  }, [todoSessionId, hydrateTodos]);

  useEffect(() => {
    if (!remoteAvailable || !slug) {
      setIssues([]);
      setPulls([]);
      setLoading(false);
      setError(null);
      return;
    }
    let alive = true;
    setLoading(true);
    setError(null);
    Promise.all([
      listItems(slug, "issues", "all"),
      listItems(slug, "pulls", "all"),
    ])
      .then(([is, ps]) => {
        if (!alive) return;
        setIssues(is);
        setPulls(ps);
      })
      .catch((e: unknown) => {
        if (alive) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [remoteAvailable, slug, reloadTick]);

  const assignmentByKey = useMemo(() => {
    const map = new Map<string, Assignment>();
    for (const assignment of assignments) {
      const key = assignmentKey(assignment, slug);
      // Assignments are newest-first. Preserve the latest attempt so a failed
      // orchestration retry cannot be hidden by an older run.
      if (key && !map.has(key)) map.set(key, assignment);
    }
    return map;
  }, [assignments, slug]);

  const items = useMemo<BoardItem[]>(() => {
    const out: BoardItem[] = [];
    if (enabled.has("issue")) out.push(...issues.map(issueToBoardItem));
    if (enabled.has("pr")) out.push(...pulls.map(pullToBoardItem));
    if (enabled.has("todo")) {
      (todos ?? []).forEach((t, i) => out.push(todoToBoardItem(t, i)));
    }
    return out.map((item) => {
      if (item.status === "done") return item;
      const assignment = assignmentByKey.get(item.key);
      const runStatus = assignment ? assignmentStatus(assignment) : null;
      return {
        ...item,
        status:
          runStatus === "in_progress"
            ? "in_progress"
            : statusOverrides[item.key] ?? runStatus ?? item.status,
      };
    });
  }, [
    issues,
    pulls,
    todos,
    enabled,
    assignmentByKey,
    statusOverrides,
  ]);

  const byColumn = useMemo(() => {
    const map = new Map<string, BoardItem[]>();
    for (const col of BOARD_COLUMNS) map.set(col.id, []);
    for (const item of items) map.get(item.status)?.push(item);
    return map;
  }, [items]);
  const sourceCounts: Record<AssignableSource, number> = {
    issue: issues.length,
    pr: pulls.length,
    todo: todos?.length ?? 0,
  };

  // Board-item keys that already have an assignment, so we don't offer a
  // duplicate "Assign agent".
  const assignedKeys = useMemo(
    () => new Set(assignmentByKey.keys()),
    [assignmentByKey],
  );

  const toggleSource = (s: AssignableSource) =>
    setEnabled((prev) => {
      const next = new Set(prev);
      if (next.has(s)) next.delete(s);
      else next.add(s);
      return next;
    });

  const createTodo = () => {
    const title = todoDraft.trim();
    if (!title || !todoSessionId) return;
    addTodo(todoSessionId, { title });
    setTodoDraft("");
    setCreatingTodo(false);
    setEnabled((current) => new Set(current).add("todo"));
  };

  const onAssign = async (card: BoardItem) => {
    setAssignError(null);
    try {
      if (
        (card.source === "issue" || card.source === "pr") &&
        card.number != null
      ) {
        if (!slug) {
          openSettingsWindow("github");
          return;
        }
        const arr = card.source === "issue" ? issues : pulls;
        const gh = arr.find((x) => x.number === card.number);
        await assignGitHubItem({
          kind: card.source,
          slug,
          number: card.number,
          title: card.title,
          body: gh?.body ?? null,
          url: card.url ?? "",
        });
      } else if (card.source === "todo") {
        const todoId = card.key.replace(/^todo-/, "");
        await assign({
          source: { kind: "todo", todoId },
          title: `🤖 ${card.title}`,
          seed: buildTodoSeed(card.title),
        });
      }
    } catch (e) {
      setAssignError(e instanceof Error ? e.message : String(e));
    }
  };

  const moveCard = useCallback(
    (card: BoardItem, status: BoardStatus) => {
      setStatusOverrides((current) => {
        const next = { ...current, [card.key]: status };
        try {
          window.localStorage.setItem(
            overrideStorageKey(storageScope),
            JSON.stringify(next),
          );
        } catch {
          // Local Overview status is best-effort; linked Projects sync remotely.
        }
        return next;
      });
      if (
        card.source === "todo" &&
        todoSessionId &&
        card.key.startsWith("todo-")
      ) {
        const todoId = card.key.replace(/^todo-/, "");
        updateTodoStatus(
          todoSessionId,
          todoId,
          status === "done"
            ? "completed"
            : status === "todo"
              ? "pending"
              : "in_progress",
        );
      }
    },
    [todoSessionId, storageScope, updateTodoStatus],
  );

  const onDrop = (status: BoardStatus) => {
    const card = items.find((item) => item.key === dragItem);
    setDragItem(null);
    if (card && card.status !== status) moveCard(card, status);
  };

  const selectedCard = items.find((item) => item.key === selectedKey) ?? null;
  const selectedAssignment = selectedCard
    ? assignmentByKey.get(selectedCard.key)
    : undefined;
  const selectedDetail: BoardCardDetail | null = selectedCard
    ? {
        title: selectedCard.title,
        source:
          selectedCard.source === "pr"
            ? "pr"
            : selectedCard.source === "issue"
              ? "issue"
              : "todo",
        status: selectedCard.status,
        statusLabel:
          BOARD_COLUMNS.find((column) => column.id === selectedCard.status)
            ?.name ?? selectedCard.status,
        number: selectedCard.number,
        url: selectedCard.url,
        body: selectedCard.body,
        meta: selectedCard.meta,
      }
    : null;

  return (
    <div className="flex h-full w-full flex-col">
      <OrchestrationBar
        workspaceKey={repoRoot}
        taskSessionId={todoSessionId}
      />
      <AssignmentsRail />

      {/* Source filters */}
      <div className="flex shrink-0 items-center gap-1.5 border-b border-border/50 px-4 py-2">
        {ALL_SOURCES.map((s) => {
          const on = enabled.has(s);
          const remoteSource = s === "issue" || s === "pr";
          const disabled = remoteSource && !remoteAvailable;
          return (
            <button
              key={s}
              type="button"
              disabled={disabled}
              onClick={() => toggleSource(s)}
              title={
                disabled
                  ? "Connect GitHub and add a GitHub origin to load this source."
                  : undefined
              }
              className={cn(
                "flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors",
                disabled
                  ? "cursor-not-allowed text-muted-foreground/30"
                  : on
                  ? SOURCE_META[s].cls
                  : "text-muted-foreground/50 hover:text-muted-foreground",
              )}
            >
              {SOURCE_META[s].label}
              <span className="opacity-70">{sourceCounts[s]}</span>
            </button>
          );
        })}
        {todoSessionId ? (
          <Button
            size="xs"
            variant="ghost"
            className="h-7 gap-1 text-[11px]"
            onClick={() => setCreatingTodo(true)}
          >
            <HugeiconsIcon icon={PlusSignIcon} size={12} strokeWidth={2} />
            New todo
          </Button>
        ) : null}
        <span className="ml-auto text-[10px] text-muted-foreground/45">
          Local workflow · drag cards to update
        </span>
        <Button
          size="xs"
          variant="ghost"
          className="h-7 w-7 p-0"
          aria-label="Refresh board"
          onClick={() => setReloadTick((t) => t + 1)}
          disabled={loading || !remoteAvailable}
          title={
            remoteAvailable
              ? "Refresh GitHub items"
              : "GitHub items are not available."
          }
        >
          <HugeiconsIcon
            icon={ArrowReloadHorizontalIcon}
            size={13}
            strokeWidth={2}
            className={cn(loading && "animate-spin")}
          />
        </Button>
      </div>

      {!githubConnectionReady ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-border/50 px-4 py-2 text-[10.5px] text-muted-foreground">
          <Spinner className="size-3.5" />
          Checking the optional GitHub connection while local work remains
          available…
        </div>
      ) : !githubConnected ? (
        <div className="flex shrink-0 items-center gap-3 border-b border-border/50 bg-sky-500/[0.045] px-4 py-2">
          <div className="min-w-0 flex-1">
            <p className="text-[11.5px] font-medium text-foreground">
              Local board is ready
            </p>
            <p className="text-[10.5px] text-muted-foreground">
              Connect GitHub only when you want to load issues, pull requests,
              and linked Projects.
            </p>
          </div>
          <Button
            size="xs"
            variant="outline"
            className="h-7 shrink-0 gap-1.5 text-[10.5px]"
            onClick={() => openSettingsWindow("github")}
          >
            Connect GitHub
          </Button>
        </div>
      ) : repoState === "loading" ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-border/50 px-4 py-2 text-[10.5px] text-muted-foreground">
          <Spinner className="size-3.5" />
          Resolving the GitHub repository while local work remains available…
        </div>
      ) : !slug ? (
        <div className="shrink-0 border-b border-border/50 bg-amber-500/[0.045] px-4 py-2">
          <p className="text-[11.5px] font-medium text-foreground">
            No GitHub origin found
          </p>
          <p className="text-[10.5px] text-muted-foreground">
            Todos and agent runs remain available. Add a GitHub origin to load
            remote issues, pull requests, and Projects.
          </p>
        </div>
      ) : null}

      {creatingTodo ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-border/50 bg-muted/15 px-4 py-2">
          <Input
            value={todoDraft}
            onChange={(event) => setTodoDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") createTodo();
              if (event.key === "Escape") {
                setTodoDraft("");
                setCreatingTodo(false);
              }
            }}
            autoFocus
            aria-label="Todo title"
            placeholder="What needs to be done?"
            className="h-8 max-w-md text-[11.5px]"
          />
          <Button
            size="xs"
            className="h-7 text-[11px]"
            onClick={createTodo}
            disabled={!todoDraft.trim()}
          >
            Add todo
          </Button>
          <Button
            size="xs"
            variant="ghost"
            className="h-7 text-[11px]"
            onClick={() => {
              setTodoDraft("");
              setCreatingTodo(false);
            }}
          >
            Cancel
          </Button>
        </div>
      ) : null}

      {error || assignError ? (
        <div
          role="alert"
          className="mx-4 mt-2 rounded-lg border border-destructive/20 bg-destructive/10 px-2.5 py-2 text-[11.5px] text-destructive"
        >
          {assignError ?? error}
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto p-4">
        {BOARD_COLUMNS.map((col) => {
          const cards = byColumn.get(col.id) ?? [];
          return (
            <div
              key={col.id}
              onDragOver={(event) => {
                if (dragItem) event.preventDefault();
              }}
              onDrop={() => onDrop(col.id)}
              className={cn(
                "flex w-72 shrink-0 flex-col rounded-xl border border-border/50 bg-card/30 transition-colors",
                dragItem && "hover:border-primary/40 hover:bg-primary/[0.025]",
              )}
            >
              <div className="flex items-center gap-2 border-b border-border/40 px-3 py-2">
                <span className="text-[12px] font-semibold text-foreground">
                  {col.name}
                </span>
                <span className="ml-auto rounded-full bg-foreground/10 px-1.5 text-[10px] font-semibold text-muted-foreground">
                  {cards.length}
                </span>
              </div>
              <ul className="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto p-2">
                {cards.map((card) => (
                  <li key={card.key}>
                    <OverviewCardView
                      card={card}
                      assigned={assignedKeys.has(card.key)}
                      onAssign={() => void onAssign(card)}
                      onOpen={() => setSelectedKey(card.key)}
                      draggable
                      onDragStart={() => setDragItem(card.key)}
                      onDragEnd={() => setDragItem(null)}
                    />
                  </li>
                ))}
                {cards.length === 0 ? (
                  <li className="px-1 py-3 text-center text-[11px] text-muted-foreground/40">
                    —
                  </li>
                ) : null}
              </ul>
            </div>
          );
        })}
      </div>

      <BoardCardDetailsSheet
        open={!!selectedCard}
        onOpenChange={(open) => !open && setSelectedKey(null)}
        card={selectedDetail}
        assignment={selectedAssignment}
        statusOptions={BOARD_COLUMNS.map((column) => ({
          id: column.id,
          label: column.name,
        }))}
        onStatusChange={(status) => {
          if (selectedCard) moveCard(selectedCard, status as BoardStatus);
        }}
        onAssign={
          selectedCard && !selectedAssignment
            ? () => void onAssign(selectedCard)
            : undefined
        }
      />
    </div>
  );
}

function OverviewCardView({
  card,
  assigned,
  onAssign,
  onOpen,
  draggable,
  onDragStart,
  onDragEnd,
}: {
  card: BoardItem;
  assigned: boolean;
  onAssign: () => void;
  onOpen: () => void;
  draggable: boolean;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const source = card.source === "agent" ? null : SOURCE_META[card.source];
  return (
    <div
      draggable={draggable}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      className="flex flex-col gap-1.5 rounded-lg border border-border/50 bg-background/70 px-2.5 py-2 active:cursor-grabbing"
    >
      <button
        type="button"
        onClick={onOpen}
        className="flex cursor-pointer flex-col gap-1.5 text-left"
      >
        <p className="line-clamp-2 text-[12px] font-medium leading-snug text-foreground">
          {card.title}
        </p>
        <div className="flex items-center gap-2 text-[10.5px] text-muted-foreground/60">
          {source ? (
            <span
              className={cn(
                "rounded px-1.5 py-px text-[9.5px] font-semibold uppercase",
                source.cls,
              )}
            >
              {source.label}
            </span>
          ) : null}
          {card.number ? <span className="font-mono">#{card.number}</span> : null}
          {card.meta ? <span className="truncate">{card.meta}</span> : null}
          {card.badge ? (
            <span className="ml-auto">
              <StateBadge state={card.badge} />
            </span>
          ) : null}
        </div>
      </button>

      {assigned ? (
        <span className="flex items-center gap-1 text-[10px] font-medium text-emerald-500">
          <HugeiconsIcon icon={Robot01Icon} size={11} strokeWidth={1.9} />
          Agent assigned
        </span>
      ) : (
        <button
          type="button"
          onClick={onAssign}
          className="flex items-center gap-1 self-start rounded-md px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground/70 transition-colors hover:bg-muted/60 hover:text-foreground"
        >
          <HugeiconsIcon icon={Robot01Icon} size={11} strokeWidth={1.9} />
          Assign agent
        </button>
      )}
    </div>
  );
}
