/**
 * Pure Task Runs outcome count projection (A6.228).
 */

export type TaskVerificationLike = {
  status: string;
};

export type TaskRunOutcomeCounts = {
  changesCount: number;
  checksPassed: number;
  checksFailed: number;
};

/** Count passed/failed checks and change listings for a completed run card. */
export function taskRunOutcomeCounts(input: {
  changesCount: number;
  verifications: readonly TaskVerificationLike[];
}): TaskRunOutcomeCounts {
  const checksPassed = input.verifications.filter(
    (v) => v.status === "passed",
  ).length;
  const checksFailed = input.verifications.filter(
    (v) => v.status === "failed",
  ).length;
  return {
    changesCount: input.changesCount,
    checksPassed,
    checksFailed,
  };
}
