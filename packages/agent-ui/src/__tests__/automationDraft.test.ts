import { describe, expect, it } from "vitest";
import {
  AUTOMATION_INTERVAL_PRESETS,
  validateAutomationDraft,
} from "../lib/automationDraft.js";

describe("validateAutomationDraft", () => {
  it("accepts once schedules", () => {
    const r = validateAutomationDraft({
      title: "Daily dig",
      prompt: "Run dig",
      scheduleKind: "once",
      onceAt: "2026-01-01T12:00:00.000Z",
      everyMs: 0,
    });
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.draft.schedule.kind).toBe("once");
    }
  });

  it("accepts interval presets every", () => {
    const everyMs = AUTOMATION_INTERVAL_PRESETS[0]!.everyMs;
    const r = validateAutomationDraft({
      title: "Hourly",
      prompt: "Check",
      scheduleKind: "every",
      onceAt: "",
      everyMs,
    });
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.draft.schedule).toEqual({ kind: "every", everyMs });
    }
  });

  it("rejects empty title", () => {
    expect(
      validateAutomationDraft({
        title: " ",
        prompt: "x",
        scheduleKind: "every",
        onceAt: "",
        everyMs: 60_000,
      }).ok,
    ).toBe(false);
  });
});
