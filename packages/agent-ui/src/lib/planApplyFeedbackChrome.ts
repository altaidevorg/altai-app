/**
 * Pure Plan Diff Review apply feedback copy (A6.245).
 */

export type PlanApplyResultLike = {
  ok: boolean;
  error?: string | null;
};

export type PlanApplyFeedback = {
  feedback: string;
  activityLabel: string;
  activityDetail?: string | null;
  tone: "success" | "error";
};

/** Results that failed apply (kept pure for bulk filters). */
export function failedPlanApplyResults<T extends { ok: boolean }>(
  results: readonly T[],
): T[] {
  return results.filter((r) => !r.ok);
}

/** User feedback + activity chrome after bulk apply. */
export function bulkPlanApplyFeedback(
  results: readonly PlanApplyResultLike[],
): PlanApplyFeedback {
  const failed = failedPlanApplyResults(results);
  if (failed.length) {
    const n = failed.length;
    const noun = n === 1 ? "change" : "changes";
    return {
      feedback: `${n} ${noun} could not be applied. They remain in review.`,
      activityLabel: "Some reviewed changes could not be applied",
      activityDetail: `${n} ${noun} remain queued`,
      tone: "error",
    };
  }
  const n = results.length;
  const noun = n === 1 ? "change" : "changes";
  return {
    feedback: `${n} ${noun} applied. A restore point is available in Undo.`,
    activityLabel: `Applied ${n} reviewed ${noun}`,
    activityDetail: "Restore points are available in Undo",
    tone: "success",
  };
}

/** User feedback + activity chrome after single-item apply. */
export function singlePlanApplyFeedback(
  result: PlanApplyResultLike,
): PlanApplyFeedback {
  if (result.ok) {
    return {
      feedback: "Change applied. A restore point is available in Undo.",
      activityLabel: "Applied a reviewed change",
      activityDetail: "Restore point available in Undo",
      tone: "success",
    };
  }
  return {
    feedback: `Could not apply change: ${result.error ?? "Unknown error"}`,
    activityLabel: "Reviewed change could not be applied",
    activityDetail: result.error,
    tone: "error",
  };
}
