import {
  CheckListIcon,
  CheckmarkCircle01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import {
  parseTodoItems,
  summarizeTodos,
  type TodoItem,
  TodoChecklist,
} from "./TodoChecklist.js";

export type TodoSummaryChipProps = {
  /** Live plan items for the active session (host owns hydration/storage). */
  todos: TodoItem[];
};

/**
 * Compact header chip with done/total + progress. Opens a host-neutral
 * details panel with the full checklist (secondary to the inline tool card).
 */
export function TodoSummaryChip({ todos }: TodoSummaryChipProps) {
  if (todos.length === 0) return null;

  const { done, total, pct } = summarizeTodos(todos);
  const allDone = done === total && total > 0;

  return (
    <details className="altai-ai-todo-summary relative">
      <summary
        title={
          allDone
            ? `Plan complete · ${done}/${total} tasks`
            : `Plan in progress · ${done}/${total} tasks`
        }
        aria-label={`Plan: ${done} of ${total} tasks done`}
        className={cn(
          "inline-flex h-6 shrink-0 cursor-pointer list-none items-center gap-1.5 rounded-md border px-1.5",
          "text-[11px] transition-colors",
          "hover:bg-foreground/[0.06]",
          "[&::-webkit-details-marker]:hidden",
          allDone
            ? "border-success/30 bg-success/[0.10] text-success"
            : "border-border bg-card text-muted-foreground hover:text-foreground",
        )}
      >
        <HugeiconsIcon
          icon={allDone ? CheckmarkCircle01Icon : CheckListIcon}
          size={12}
          strokeWidth={1.75}
          className="shrink-0"
        />
        <span className="tabular-nums font-medium">
          {done}/{total}
        </span>
        {!allDone ? (
          <span className="relative h-1 w-10 overflow-hidden rounded-full bg-muted">
            <span
              className="absolute inset-y-0 left-0 rounded-full bg-primary transition-[width] duration-300"
              style={{ width: `${pct}%` }}
            />
          </span>
        ) : null}
      </summary>
      <div
        aria-label="Plan checklist"
        className="absolute right-0 top-full z-50 mt-1 w-72 overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-md"
      >
        <TodoSummaryPanel todos={todos} done={done} total={total} pct={pct} />
      </div>
    </details>
  );
}

function TodoSummaryPanel({
  todos,
  done,
  total,
  pct,
}: {
  todos: TodoItem[];
  done: number;
  total: number;
  pct: number;
}) {
  const items = parseTodoItems({ items: todos });
  return (
    <div className="flex max-h-80 min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b border-border/40 px-3 py-2">
        <HugeiconsIcon
          icon={CheckListIcon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
        />
        <span className="text-[12px] font-medium text-foreground">Plan</span>
        <span className="text-[11px] tabular-nums text-muted-foreground">
          {done}/{total}
        </span>
        <span className="relative h-1 flex-1 overflow-hidden rounded-full bg-muted">
          <span
            className="absolute inset-y-0 left-0 rounded-full bg-primary transition-[width] duration-300"
            style={{ width: `${pct}%` }}
          />
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <div className="px-2 py-1.5">
          <TodoChecklist items={items} />
        </div>
      </div>
    </div>
  );
}
