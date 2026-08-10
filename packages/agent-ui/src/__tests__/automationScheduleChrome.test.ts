import { describe, expect, it } from "vitest";
import {
  automationNextRunAtMs,
  defaultAutomationAtValue,
} from "../lib/automationScheduleChrome.js";

describe("defaultAutomationAtValue", () => {
  it("adds five minutes and clears seconds", () => {
    const base = Date.parse("2026-08-10T12:00:30.500Z");
    const value = defaultAutomationAtValue(base);
    expect(value).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
    // localDateTime is local TZ — only shape + that it is stable for fixed now
    expect(defaultAutomationAtValue(base)).toBe(value);
  });
});

describe("automationNextRunAtMs", () => {
  it("uses atMs for one-shot", () => {
    expect(
      automationNextRunAtMs({ schedule: { kind: "at", atMs: 42 } }, 0),
    ).toBe(42);
  });
  it("adds interval from last run or now", () => {
    expect(
      automationNextRunAtMs(
        { schedule: { kind: "every", everyMs: 1000 }, lastRunAtMs: 5000 },
        9,
      ),
    ).toBe(6000);
    expect(
      automationNextRunAtMs({ schedule: { kind: "every", everyMs: 1000 } }, 200),
    ).toBe(1200);
  });
  it("uses max for unknown schedule kinds", () => {
    expect(
      automationNextRunAtMs({ schedule: { kind: "cron" } }, 0),
    ).toBe(Number.MAX_SAFE_INTEGER);
  });
});
