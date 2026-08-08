import { describe, expect, it } from "vitest";
import {
  enabledExcludePatterns,
  searchExcludeGlobFromSettings,
} from "../lib/searchExcludeGlobs.js";

describe("searchExcludeGlobs", () => {
  it("collects enabled patterns only", () => {
    expect(
      enabledExcludePatterns({ "**/.git": true, "tmp": false, "x": true }),
    ).toEqual(["**/.git", "x"]);
  });
  it("merges defaults into brace glob", () => {
    const g = searchExcludeGlobFromSettings({
      filesExclude: { "**/dist": true },
    });
    expect(g.startsWith("{")).toBe(true);
    expect(g).toContain("**/node_modules");
    expect(g).toContain("**/dist");
  });
});
