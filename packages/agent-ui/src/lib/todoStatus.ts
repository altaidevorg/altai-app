/**
 * Pure todo_write status normalize for plan/todo chrome (A6.144).
 * Case-insensitive and tolerant of common LLM variants.
 */

export type SharedTodoStatus = "pending" | "in_progress" | "completed";

export function normalizeTodoStatus(value: unknown): SharedTodoStatus {
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
