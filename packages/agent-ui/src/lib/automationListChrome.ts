/**
 * Pure automation list chrome (A6.206).
 */

/** Stable ordered automation list by id. */
export function sortAutomationItemsById<T extends { id: string }>(
  items: readonly T[],
): T[] {
  return [...items].sort((left, right) => left.id.localeCompare(right.id));
}

export type BackgroundJobLike = {
  id: string;
  updatedAtMs: number;
};

/**
 * Index background jobs with `cron:<automationId>` ids to the latest job
 * per automation (highest updatedAtMs wins).
 */
export function indexLatestCronJobsByAutomationId<T extends BackgroundJobLike>(
  jobs: readonly T[],
): Record<string, T> {
  return jobs.reduce<Record<string, T>>((result, job) => {
    if (!job.id.startsWith("cron:")) return result;
    const automationId = job.id.slice("cron:".length);
    if (!automationId) return result;
    const existing = result[automationId];
    if (!existing || existing.updatedAtMs < job.updatedAtMs) {
      result[automationId] = job;
    }
    return result;
  }, {});
}
