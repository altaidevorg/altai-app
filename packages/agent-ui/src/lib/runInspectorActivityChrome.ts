/**
 * Pure Run Inspector activity / todo metrics (A6.239).
 */

export type RunActivityItemLike = {
  label?: string | null;
  detail?: string | null;
  kind?: string | null;
  tone?: string | null;
};

/** Count todos marked completed. */
export function countCompletedTodos(
  todos: readonly { status: string }[],
): number {
  return todos.filter((todo) => todo.status === "completed").length;
}

/** Free-text filter over activity label/detail/kind/tone. */
export function filterActivityByQuery<T extends RunActivityItemLike>(
  items: readonly T[],
  query: string,
): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return [...items];
  return items.filter((item) =>
    [item.label, item.detail, item.kind, item.tone]
      .filter(Boolean)
      .join("\n")
      .toLowerCase()
      .includes(q),
  );
}

/** Keep activity rows of a single kind (research, mcp, …). */
export function filterActivityByKind<T extends { kind: string }>(
  items: readonly T[],
  kind: string,
): T[] {
  return items.filter((item) => item.kind === kind);
}

/** True while the agent run is thinking or streaming. */
export function isAgentRunBusy(status: string): boolean {
  return status === "thinking" || status === "streaming";
}
