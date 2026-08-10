import { describe, expect, it } from "vitest";
import {
  runRecoveryDetail,
  runRecoveryPresentation,
  runRecoverySteerPrompt,
  runRecoveryTitle,
} from "../lib/runRecoveryChrome.js";

describe("runRecoveryChrome", () => {
  it("builds warning title and detail", () => {
    expect(
      runRecoveryTitle({ hasWarning: true, canRetry: false }),
    ).toBe("Possible repeated failure");
    expect(
      runRecoveryDetail({ warningDescription: "Same failure twice" }),
    ).toContain("still working");
  });

  it("builds stuck, budget, and retry presentations", () => {
    expect(
      runRecoveryTitle({
        hasWarning: false,
        canRetry: false,
        outcome: { kind: "stuck", reason: "doom_loop" },
      }),
    ).toBe("Run paused");
    expect(
      runRecoveryDetail({
        outcome: { kind: "stuck", reason: "doom_loop" },
      }),
    ).toBe("The run paused because it was doom loop.");
    expect(
      runRecoveryPresentation({
        canRetry: false,
        outcome: {
          kind: "budget_exhausted",
          budget: { iterations_used: 40 },
        },
      }).title,
    ).toBe("Turn limit reached");
    expect(
      runRecoveryTitle({ hasWarning: false, canRetry: true }),
    ).toBe("Retry available");
    expect(runRecoveryDetail({ outcome: { kind: "failed" } })).toContain(
      "retry policy",
    );
  });

  it("builds steer prefill", () => {
    expect(runRecoverySteerPrompt(true)).toContain("Adjust the active run");
    expect(runRecoverySteerPrompt(false)).toContain("Continue the previous");
  });
});
