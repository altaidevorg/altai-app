import { describe, expect, it } from "vitest";
import {
  modelDropdownAutoDetail,
  modelDropdownEmptyMessage,
  modelDropdownTriggerLabel,
  modelDropdownTriggerTitle,
} from "../lib/modelDropdownChrome.js";

describe("modelDropdownChrome", () => {
  it("builds trigger label", () => {
    expect(modelDropdownTriggerLabel(false, "GPT")).toBe("GPT");
    expect(modelDropdownTriggerLabel(true, "GPT", "Claude")).toBe(
      "Auto · Claude",
    );
    expect(modelDropdownTriggerLabel(true, "GPT")).toBe("Auto · GPT");
  });

  it("builds trigger title for auto/usable/missing key", () => {
    expect(modelDropdownTriggerTitle(true, true, "GPT", "Claude")).toContain(
      "Claude",
    );
    expect(modelDropdownTriggerTitle(false, true, "GPT")).toBe("Model: GPT");
    expect(modelDropdownTriggerTitle(false, false, "GPT")).toContain(
      "API key",
    );
  });

  it("resolves empty message and auto detail", () => {
    expect(modelDropdownEmptyMessage(1, 0)).toBeNull();
    expect(modelDropdownEmptyMessage(0, 0)).toContain("No models available");
    expect(modelDropdownEmptyMessage(0, 3, "Agent limit")).toBe("Agent limit");
    expect(modelDropdownEmptyMessage(0, 3)).toBe("No models match.");
    expect(modelDropdownAutoDetail("X")).toBe("Recommended now: X");
    expect(modelDropdownAutoDetail(null)).toBe(
      "Choose from compatible models",
    );
  });
});
