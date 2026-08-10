import { describe, expect, it } from "vitest";
import {
  planInspectorSectionSummary,
  planProgressMetricValue,
  runInspectorHeaderSubtitle,
  runInspectorUsageTokenLabel,
} from "../lib/runInspectorChrome.js";

describe("runInspectorChrome", () => {
  it("builds header subtitle for idle and working", () => {
    expect(runInspectorHeaderSubtitle("idle")).toBe("Ready for the next task");
    expect(runInspectorHeaderSubtitle("running", "Reading files")).toBe(
      "Reading files",
    );
    expect(runInspectorHeaderSubtitle("running")).toBe("Agent is working");
  });

  it("formats usage and plan progress labels", () => {
    expect(runInspectorUsageTokenLabel(0)).toBe("No usage yet");
    expect(runInspectorUsageTokenLabel(1500)).toMatch(/1[,.]500 tokens/);
    expect(planProgressMetricValue(2, 5)).toBe("2/5");
    expect(planProgressMetricValue(0, 0)).toBe("—");
    expect(planInspectorSectionSummary(1, 3)).toBe(
      "1 of 3 steps complete",
    );
    expect(planInspectorSectionSummary(0, 0)).toBe(
      "No checklist for this run",
    );
  });
});
