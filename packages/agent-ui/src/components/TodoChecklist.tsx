import {
  CancelCircleIcon,
  CheckmarkCircle01Icon,
  Loading03Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ComponentProps } from "react";
import { cn } from "../lib/cn.js";
import type { TodoItem } from "../lib/todoParse.js";

export type { TodoItem, TodoItemStatus } from "../lib/todoParse.js";
export { parseTodoItems, summarizeTodos } from "../lib/todoParse.js";

export type TodoChecklistProps = ComponentProps<"ul"> & {
  items: TodoItem[];
  /** Smaller / denser variant — used inside the inline tool card. */
  dense?: boolean;
};

/**
 * Shared todo checklist renderer. Used both by the inline `todo_write` tool
 * card (dense) and the standalone todo summary.
 */
export function TodoChecklist({
  items,
  dense = false,
  className,
  ...props
}: TodoChecklistProps) {
  return (
    <ul
      className={cn(
        "flex flex-col gap-0.5",
        dense ? "text-[11.5px]" : "text-[12px]",
        className,
      )}
      {...props}
    >
      {items.map((item, i) => (
        <TodoChecklistRow key={item.id ?? i} item={item} dense={dense} />
      ))}
    </ul>
  );
}

function TodoChecklistRow({
  item,
  dense,
}: {
  item: TodoItem;
  dense: boolean;
}) {
  const isInProgress = item.status === "in_progress";
  const isDone = item.status === "completed";
  return (
    <li
      className={cn(
        "flex items-start gap-2 rounded-sm",
        dense ? "px-1 py-0.5" : "px-1.5 py-1",
        isInProgress && "bg-muted/40",
      )}
    >
      <span className="mt-[2px] inline-flex size-3.5 shrink-0 items-center justify-center">
        {isInProgress ? (
          <HugeiconsIcon
            icon={Loading03Icon}
            size={13}
            strokeWidth={1.75}
            className="animate-spin text-foreground"
          />
        ) : isDone ? (
          <HugeiconsIcon
            icon={CheckmarkCircle01Icon}
            size={13}
            strokeWidth={1.75}
            className="text-emerald-600 dark:text-emerald-400"
          />
        ) : (
          <HugeiconsIcon
            icon={CancelCircleIcon}
            size={13}
            strokeWidth={1.75}
            className="text-muted-foreground/40"
          />
        )}
      </span>
      <span
        className={cn(
          "min-w-0 flex-1 leading-snug",
          isDone
            ? "text-muted-foreground/70 line-through"
            : isInProgress
              ? "text-foreground"
              : "text-muted-foreground",
        )}
      >
        {item.title}
        {item.description ? (
          <span className="block text-[10.5px] text-muted-foreground/70">
            {item.description}
          </span>
        ) : null}
      </span>
    </li>
  );
}
