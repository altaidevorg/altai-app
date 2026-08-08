import { describe, expect, it } from "vitest";
import {
  formatProblemsBundles,
  formatProblemsContextText,
} from "../lib/problemsContext.js";

describe("problemsContext", () => {
  it("formats single file problems", () => {
    const text = formatProblemsContextText("src/a.ts", [
      {
        severity: 0,
        message: "oops",
        startLine: 0,
        startCharacter: 0,
        endLine: 0,
        endCharacter: 1,
        source: "tsc",
      },
    ]);
    expect(text).toContain("Problems in src/a.ts");
    expect(text).toContain("Error [tsc] L1:1: oops");
  });
  it("returns null for empty", () => {
    expect(formatProblemsBundles([])).toBeNull();
    expect(formatProblemsBundles([{ pathLabel: "x", diagnostics: [] }])).toBeNull();
  });
});
