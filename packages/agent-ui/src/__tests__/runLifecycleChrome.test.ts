import { describe, expect, it } from "vitest";
import {
  recoveryCopy,
  runBlockedMessageFromEvent,
  runWarningMessageFromEvent,
  shouldShowChangeReviewBanner,
  shouldShowRunRecovery,
} from "../lib/runLifecycleChrome.js";

describe("shouldShowChangeReviewBanner", () => {
  it("requires a non-empty queue", () => {
    expect(shouldShowChangeReviewBanner(0)).toBe(false);
    expect(shouldShowChangeReviewBanner(2)).toBe(true);
  });
});

describe("runBlockedMessageFromEvent", () => {
  it("returns null on clean success", () => {
    expect(
      runBlockedMessageFromEvent({
        type: "run_terminated",
        outcome: "success",
      }),
    ).toBeNull();
  });

  it("surfaces failures and cancellations", () => {
    expect(
      runBlockedMessageFromEvent({
        type: "run_terminated",
        outcome: "failed",
        error: "model timeout",
      }),
    ).toBe("model timeout");
    expect(runBlockedMessageFromEvent({ type: "run_cancelled" })).toBe(
      "Run cancelled",
    );
  });

  it("unwraps nested payload envelopes", () => {
    expect(
      runBlockedMessageFromEvent({
        payload: {
          type: "run_terminated",
          outcome: "error",
          error: { message: "nested" },
        },
      }),
    ).toBe("nested");
  });
});

describe("run recovery helpers", () => {
  it("prefers recovery strip when retry is available", () => {
    expect(
      shouldShowRunRecovery({
        blockedMessage: "boom",
        warningMessage: null,
        canRetry: true,
        canSteer: false,
        hasActiveRun: false,
      }),
    ).toBe(true);
    expect(
      shouldShowRunRecovery({
        blockedMessage: "boom",
        warningMessage: null,
        canRetry: false,
        canSteer: false,
        hasActiveRun: false,
      }),
    ).toBe(false);
  });

  it("maps warning copy", () => {
    expect(
      recoveryCopy({
        blockedMessage: null,
        warningMessage: "rate limited",
        canRetry: false,
        canSteer: true,
        hasActiveRun: true,
      }),
    ).toEqual({
      warning: true,
      title: "Run needs attention",
      detail: "rate limited",
    });
  });

  it("parses run_warning events", () => {
    expect(
      runWarningMessageFromEvent("lifecycle", {
        type: "run_warning",
        warning: "slow tool",
      }),
    ).toBe("slow tool");
    expect(
      runWarningMessageFromEvent("lifecycle", { type: "run_warning_cleared" }),
    ).toBe("");
  });
});
