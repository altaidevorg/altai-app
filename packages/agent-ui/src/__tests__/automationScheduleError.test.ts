import { describe, expect, it } from "vitest";
import { automationScheduleError } from "../lib/automationScheduleError.js";

describe("automationScheduleError", () => {
  it("validates once schedule", () => {
    expect(automationScheduleError("at", 100, 60, 200)).toBe(
      "Choose a valid future time",
    );
    expect(automationScheduleError("at", 300, 60, 200)).toBeNull();
  });
  it("validates interval minutes", () => {
    expect(automationScheduleError("every", 0, 0, 0)).toBe(
      "Minimum interval is 1 minute",
    );
    expect(automationScheduleError("every", 0, 1, 0)).toBeNull();
  });
});
