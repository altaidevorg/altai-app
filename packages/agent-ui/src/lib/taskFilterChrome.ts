/**
 * Pure Task Runs list filter/count/group chrome (A6.214).
 */

export type TaskListFilterId = "all" | "active" | "attention" | "finished";

export type TaskStatusRowLike = {
  status: string;
};

export type TaskFilterCounts = {
  all: number;
  active: number;
  attention: number;
  finished: number;
};

export function isTaskAttentionStatus(status: string): boolean {
  return status === "awaiting-approval" || status === "failed";
}

export function taskFilterCounts(
  rows: readonly TaskStatusRowLike[],
  activeStatuses: readonly string[],
  terminalStatuses: readonly string[],
): TaskFilterCounts {
  return {
    all: rows.length,
    active: rows.filter((row) => activeStatuses.includes(row.status)).length,
    attention: rows.filter((row) => isTaskAttentionStatus(row.status)).length,
    finished: rows.filter((row) => terminalStatuses.includes(row.status)).length,
  };
}

export function taskMatchesListFilter(
  status: string,
  filter: TaskListFilterId,
  activeStatuses: readonly string[],
  terminalStatuses: readonly string[],
): boolean {
  if (filter === "all") return true;
  if (filter === "active") return activeStatuses.includes(status);
  if (filter === "attention") return isTaskAttentionStatus(status);
  if (filter === "finished") return terminalStatuses.includes(status);
  return true;
}

/** Search over free-form fields (title, prompt, step, lastResult…). */
export function taskMatchesQuery(
  fields: readonly string[],
  query: string,
): boolean {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return true;
  return fields.join("\n").toLowerCase().includes(normalizedQuery);
}

export type TaskGroupBucketId =
  | "attention"
  | "active"
  | "ready"
  | "stopped";

export type TaskGroupDefinition = {
  id: TaskGroupBucketId;
  title: string;
  description: string;
  match: (status: string) => boolean;
};

export const TASK_GROUP_DEFINITIONS: readonly TaskGroupDefinition[] = [
  {
    id: "attention",
    title: "Needs attention",
    description: "Runs waiting on you or blocked by an error",
    match: isTaskAttentionStatus,
  },
  {
    id: "active",
    title: "In progress",
    description: "Agents currently working in isolated chats",
    match: (status) => status === "dispatching" || status === "running",
  },
  {
    id: "ready",
    title: "Ready to review",
    description: "Completed runs with transcripts and outcomes",
    match: (status) => status === "done",
  },
  {
    id: "stopped",
    title: "Stopped",
    description: "Cancelled background work",
    match: (status) => status === "cancelled",
  },
];

/** Build list groups for non-empty buckets, preserving definition order. */
export function partitionTasksByGroupStatus<T extends TaskStatusRowLike>(
  rows: readonly T[],
  definitions: readonly TaskGroupDefinition[] = TASK_GROUP_DEFINITIONS,
): Array<{
  id: TaskGroupBucketId;
  title: string;
  description: string;
  items: T[];
}> {
  return definitions
    .map((definition) => ({
      id: definition.id,
      title: definition.title,
      description: definition.description,
      items: rows.filter((row) => definition.match(row.status)),
    }))
    .filter((group) => group.items.length > 0);
}
