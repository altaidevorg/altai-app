/**
 * Pure Task Runs list ordering / agent enable filter (A6.224).
 */

export function sortByCreatedAtDesc<T extends { createdAt: number }>(
  items: readonly T[],
): T[] {
  return [...items].sort((left, right) => right.createdAt - left.createdAt);
}

/** Drop disabled agents from a picker list. */
export function filterEnabledAgents<T extends { id: string }>(
  agents: readonly T[],
  isDisabled: (id: string) => boolean,
): T[] {
  return agents.filter((agent) => !isDisabled(agent.id));
}

/** Keep only task-sourced assignments. */
export function filterTaskSourceAssignments<
  T extends { source: { kind: string } },
>(assignments: readonly T[]): T[] {
  return assignments.filter((assignment) => assignment.source.kind === "task");
}
