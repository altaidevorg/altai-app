import { describe, expect, it } from "vitest";
import {
  clampProjectInstructions,
  combineAgentInstructions,
  projectInstructionsPath,
  PROJECT_INSTRUCTIONS_FILE,
} from "../lib/projectInstructions.js";

describe("projectInstructions (A6.150)", () => {
  it("joins path and combines sections", () => {
    expect(projectInstructionsPath("/repo/")).toBe(
      `/repo/${PROJECT_INSTRUCTIONS_FILE}`,
    );
    expect(combineAgentInstructions("persona", "rules")).toContain(
      "project-instructions",
    );
    expect(combineAgentInstructions("persona", "rules")).toContain("persona");
    expect(combineAgentInstructions(undefined, undefined)).toBe(undefined);
  });

  it("clamps project text", () => {
    expect(clampProjectInstructions("  hi  ")).toBe("hi");
    expect(clampProjectInstructions("")).toBe(undefined);
    expect(clampProjectInstructions("abcdef", 3)).toBe("abc");
  });
});
