import { describe, expect, it } from "vitest";
import {
  continueBudgetSegmentPrompt,
  continueStuckPrompt,
  describeRunWarning,
  describeTerminalOutcomeAttention,
  isRecoverableRunOutcome,
} from "../lib/runContinueChrome.js";

describe("runContinueChrome", () => {
  it("classifies recoverable outcomes", () => {
    expect(isRecoverableRunOutcome({ kind: "stuck", reason: "x" })).toBe(true);
    expect(
      isRecoverableRunOutcome({
        kind: "budget_exhausted",
        budget: { iterations_used: 3 },
      }),
    ).toBe(true);
    expect(
      isRecoverableRunOutcome({
        kind: "failed",
        failure: "boom",
        retryable: true,
      }),
    ).toBe(false);
  });

  it("describes terminal attention for pauses", () => {
    expect(
      describeTerminalOutcomeAttention({
        kind: "stuck",
        reason: "Stopped: model loop",
      }),
    ).toBe("Run paused — model loop");
    expect(
      describeTerminalOutcomeAttention({
        kind: "budget_exhausted",
        budget: { iterations_used: 12 },
      }),
    ).toBe("Run paused — Hit the turn limit after 12 steps");
    expect(describeTerminalOutcomeAttention({ kind: "completed" })).toBe(null);
  });

  it("exposes continue prompts and warning copy", () => {
    expect(continueStuckPrompt()).toContain("Continue the previous task");
    expect(continueBudgetSegmentPrompt()).toContain("additional turns");
    expect(
      describeRunWarning({
        reason: { kind: "no_progress", turns: 4 },
      }),
    ).toBe("No measurable progress for 4 turns");
  });
});
