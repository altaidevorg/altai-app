import { describe, expect, it } from "vitest";
import {
  isActiveFileInContext,
  isTerminalContextAvailable,
  isWorkspaceContextAvailable,
} from "../lib/taskContextAvailabilityChrome.js";

describe("taskContextAvailabilityChrome", () => {
  it("detects active file already in context", () => {
    expect(isActiveFileInContext("a.ts", ["a.ts", "b.ts"])).toBe(true);
    expect(isActiveFileInContext("c.ts", ["a.ts"])).toBe(false);
    expect(isActiveFileInContext(null, ["a.ts"])).toBe(false);
    expect(isActiveFileInContext("", ["a.ts"])).toBe(false);
  });

  it("requires non-private non-empty terminal text", () => {
    expect(isTerminalContextAvailable(false, "ls\n")).toBe(true);
    expect(isTerminalContextAvailable(true, "ls\n")).toBe(false);
    expect(isTerminalContextAvailable(false, "  ")).toBe(false);
    expect(isTerminalContextAvailable(false, null)).toBe(false);
  });

  it("accepts cwd or workspace root", () => {
    expect(isWorkspaceContextAvailable("/tmp", null)).toBe(true);
    expect(isWorkspaceContextAvailable(null, "/ws")).toBe(true);
    expect(isWorkspaceContextAvailable(null, null)).toBe(false);
  });
});
