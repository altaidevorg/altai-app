/**
 * Pure automation Operations list filter/sort chrome (A6.210).
 */

export type AutomationFilterId = "all" | "once" | "repeat" | "issues";

export type AutomationFilterItemLike = {
  id: string;
  message: string;
  chatId: string;
  schedule: { kind: string };
};

export type AutomationJobLike = {
  lastError?: string | null;
};

export type AutomationFilterCounts = {
  all: number;
  once: number;
  repeat: number;
  issues: number;
};

export function automationFilterCounts(
  items: readonly AutomationFilterItemLike[],
  jobsByAutomationId: Readonly<Record<string, AutomationJobLike | undefined>>,
): AutomationFilterCounts {
  return {
    all: items.length,
    once: items.filter((item) => item.schedule.kind === "at").length,
    repeat: items.filter((item) => item.schedule.kind !== "at").length,
    issues: items.filter((item) => jobsByAutomationId[item.id]?.lastError)
      .length,
  };
}

export function automationMatchesFilter(
  item: AutomationFilterItemLike,
  filter: AutomationFilterId,
  jobsByAutomationId: Readonly<Record<string, AutomationJobLike | undefined>>,
): boolean {
  if (filter === "once" && item.schedule.kind !== "at") return false;
  if (filter === "repeat" && item.schedule.kind === "at") return false;
  if (filter === "issues" && !jobsByAutomationId[item.id]?.lastError) {
    return false;
  }
  return true;
}

export function automationMatchesQuery(
  item: AutomationFilterItemLike,
  query: string,
  title: string,
  scheduleLabel: string,
  lastError: string,
): boolean {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return true;
  return [item.message, title, scheduleLabel, lastError]
    .join("\n")
    .toLowerCase()
    .includes(normalizedQuery);
}

/** Failed jobs first; `compareNextRun` decides remaining order (asc next-run). */
export function compareAutomationsForList(
  left: AutomationFilterItemLike,
  right: AutomationFilterItemLike,
  jobsByAutomationId: Readonly<Record<string, AutomationJobLike | undefined>>,
  compareNextRun: (
    left: AutomationFilterItemLike,
    right: AutomationFilterItemLike,
  ) => number,
): number {
  const leftFailed = Boolean(jobsByAutomationId[left.id]?.lastError);
  const rightFailed = Boolean(jobsByAutomationId[right.id]?.lastError);
  if (leftFailed !== rightFailed) return leftFailed ? -1 : 1;
  return compareNextRun(left, right);
}
