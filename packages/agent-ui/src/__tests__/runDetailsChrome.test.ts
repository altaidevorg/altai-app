import { describe, expect, it } from "vitest";
import {
  buildRunOverviewMetrics,
  canShowRunDetailsChrome,
  countToolMessages,
  runDetailsStatus,
  runDetailsStepLabel,
  runDetailsSubtitle,
  runDetailsTokenLabel,
} from "../lib/runDetailsChrome.js";

describe("canShowRunDetailsChrome", () => {
  it("shows for active run or attention messages", () => {
    expect(
      canShowRunDetailsChrome({
        hasActiveRun: true,
        blockedMessage: null,
      }),
    ).toBe(true);
    expect(
      canShowRunDetailsChrome({
        hasActiveRun: false,
        blockedMessage: "failed",
      }),
    ).toBe(true);
    expect(
      canShowRunDetailsChrome({
        hasActiveRun: false,
        blockedMessage: null,
        warningMessage: "warn",
      }),
    ).toBe(true);
    expect(
      canShowRunDetailsChrome({
        hasActiveRun: false,
        blockedMessage: null,
      }),
    ).toBe(false);
  });
});

describe("run details status and copy", () => {
  it("maps blocked/running/idle", () => {
    expect(
      runDetailsStatus({ hasActiveRun: true, blockedMessage: null }),
    ).toBe("running");
    expect(
      runDetailsStatus({
        hasActiveRun: false,
        blockedMessage: "err",
      }),
    ).toBe("blocked");
    expect(
      runDetailsStatus({ hasActiveRun: false, blockedMessage: null }),
    ).toBe("idle");
  });

  it("builds subtitle and token labels", () => {
    expect(
      runDetailsSubtitle({ chatId: "c1", status: "running" }),
    ).toContain("c1");
    expect(runDetailsTokenLabel({ hasActiveRun: true, status: "running" })).toBe(
      "Tokens · live",
    );
    expect(
      runDetailsTokenLabel({
        hasActiveRun: true,
        status: "running",
        totalTokens: 1500,
      }),
    ).toBe("Tokens · 1.5k");
  });

  it("prefers blocked step copy", () => {
    expect(
      runDetailsStepLabel({
        step: "Editing file",
        blockedMessage: "Run failed",
      }),
    ).toBe("Run failed");
  });
});

describe("run overview metrics", () => {
  it("counts roles and tools", () => {
    const messages = [
      { role: "user" },
      { role: "assistant" },
      { role: "tool" },
      { role: "tool" },
    ];
    expect(countToolMessages(messages)).toBe(2);
    const metrics = buildRunOverviewMetrics({
      messages,
      toolCount: 2,
      editDiffCount: 1,
      approvalsPending: 1,
    });
    expect(metrics.map((m) => m.label)).toEqual([
      "User turns",
      "Replies",
      "Tools",
      "Edits",
      "Approvals",
    ]);
  });
});
