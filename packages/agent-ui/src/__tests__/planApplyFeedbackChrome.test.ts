import { describe, expect, it } from "vitest";
import {
  bulkPlanApplyFeedback,
  failedPlanApplyResults,
  singlePlanApplyFeedback,
} from "../lib/planApplyFeedbackChrome.js";

describe("planApplyFeedbackChrome", () => {
  it("filters failed apply results", () => {
    expect(
      failedPlanApplyResults([{ ok: true }, { ok: false }, { ok: true }]),
    ).toEqual([{ ok: false }]);
  });

  it("builds bulk failure feedback", () => {
    const fb = bulkPlanApplyFeedback([{ ok: false }, { ok: false }]);
    expect(fb.tone).toBe("error");
    expect(fb.feedback).toContain("2 changes could not be applied");
    expect(fb.activityDetail).toBe("2 changes remain queued");
  });

  it("builds bulk success feedback (singular)", () => {
    const fb = bulkPlanApplyFeedback([{ ok: true }]);
    expect(fb.tone).toBe("success");
    expect(fb.feedback).toBe(
      "1 change applied. A restore point is available in Undo.",
    );
    expect(fb.activityLabel).toBe("Applied 1 reviewed change");
  });

  it("builds single apply success and error feedback", () => {
    expect(singlePlanApplyFeedback({ ok: true }).tone).toBe("success");
    const err = singlePlanApplyFeedback({ ok: false, error: "disk full" });
    expect(err.tone).toBe("error");
    expect(err.feedback).toBe("Could not apply change: disk full");
    expect(err.activityDetail).toBe("disk full");
    expect(
      singlePlanApplyFeedback({ ok: false }).feedback,
    ).toContain("Unknown error");
  });
});
