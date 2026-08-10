import { describe, expect, it } from "vitest";
import { taskPermissionModeShortLabel } from "../lib/taskPermissionModeChrome.js";

describe("taskPermissionModeChrome", () => {
  it("maps known modes and falls back", () => {
    expect(taskPermissionModeShortLabel("ask")).toBe("Ask");
    expect(taskPermissionModeShortLabel("auto-edit")).toBe("Auto-edit");
    expect(taskPermissionModeShortLabel("plan")).toBe("Plan");
    expect(taskPermissionModeShortLabel("bypass")).toBe("Bypass");
    expect(taskPermissionModeShortLabel("other")).toBe("Ask");
  });
});
