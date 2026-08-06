/**
 * Pure todo_write parsing for chat tools and checklist chrome.
 * Wave 4 / A6.18 — host-neutral; no React.
 */

export type TodoItemStatus = "pending" | "in_progress" | "completed";

export type TodoItem = {
  id?: string;
  title: string;
  description?: string;
  status: TodoItemStatus;
};

/** True for tool names that update the agent todo list. */
export function isTodoToolName(name: string): boolean {
  const n = name.toLowerCase().replace(/[\s-]+/g, "_");
  return (
    n === "todo_write" ||
    n === "todowrite" ||
    n === "update_todos" ||
    n === "todo" ||
    n === "todos"
  );
}

/**
 * Parse free-form `todo_write` input items into the strict TodoItem shape.
 * Field names vary by model — content/title/task/text are all observed.
 */
export function parseTodoItems(input: unknown): TodoItem[] {
  if (!input || typeof input !== "object") return [];
  const items = (input as { items?: unknown }).items;
  if (!Array.isArray(items)) return [];
  return items.map((raw, i) => {
    const it = (raw ?? {}) as Record<string, unknown>;
    const title =
      (typeof it.content === "string" && it.content) ||
      (typeof it.title === "string" && it.title) ||
      (typeof it.task === "string" && it.task) ||
      (typeof it.text === "string" && it.text) ||
      "Untitled task";
    const id = typeof it.id === "string" ? it.id : `item-${i}`;
    const description =
      typeof it.description === "string" ? it.description : undefined;
    return {
      id,
      title,
      ...(description ? { description } : {}),
      status: normalizeStatus(it.status),
    };
  });
}

function normalizeStatus(value: unknown): TodoItemStatus {
  const v =
    typeof value === "string"
      ? value.trim().toLowerCase().replace(/[\s-]+/g, "_")
      : "";
  if (["completed", "complete", "done", "finished"].includes(v)) {
    return "completed";
  }
  if (
    ["in_progress", "active", "running", "doing", "started", "wip"].includes(v)
  ) {
    return "in_progress";
  }
  return "pending";
}

export function summarizeTodos(items: readonly TodoItem[]): {
  total: number;
  done: number;
  inProgress: number;
  pct: number;
} {
  const total = items.length;
  const done = items.filter((i) => i.status === "completed").length;
  const inProgress = items.filter((i) => i.status === "in_progress").length;
  const pct = total === 0 ? 0 : Math.round((done / total) * 100);
  return { total, done, inProgress, pct };
}

/** VS Code host alias — same implementation as `parseTodoItems`. */
export const parseTodoItemsFromInput = parseTodoItems;

/** VS Code host alias — same implementation as `summarizeTodos`. */
export const summarizeTodoItems = summarizeTodos;
