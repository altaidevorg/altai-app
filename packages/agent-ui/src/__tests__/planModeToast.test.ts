import { describe, expect, it } from "vitest";
import {
  planModeOffToast,
  planModeOnToast,
  planModeToggleToast,
} from "../lib/planModeToast.js";

describe("planModeToast", () => {
  it("labels on/off", () => {
    expect(planModeOnToast()).toBe("Plan mode on");
    expect(planModeOffToast()).toBe("Plan mode off");
    expect(planModeToggleToast(true)).toBe("Plan mode on");
    expect(planModeToggleToast(false)).toBe("Plan mode off");
  });
});
