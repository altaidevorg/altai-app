import { describe, expect, it } from "vitest";
import {
  appendUniqueContextPaths,
  normalizeDialogPathSelection,
  stripTaskBotTitlePrefix,
} from "../lib/taskContextPathChrome.js";

describe("normalizeDialogPathSelection", () => {
  it("accepts a single path string", () => {
    expect(normalizeDialogPathSelection("  /a.ts ")).toEqual(["/a.ts"]);
  });

  it("accepts path arrays and drops empties", () => {
    expect(normalizeDialogPathSelection(["/a", "  ", "/b"])).toEqual([
      "/a",
      "/b",
    ]);
  });

  it("returns empty for nullish", () => {
    expect(normalizeDialogPathSelection(null)).toEqual([]);
    expect(normalizeDialogPathSelection(undefined)).toEqual([]);
  });
});

describe("appendUniqueContextPaths", () => {
  it("dedupes and caps", () => {
    expect(
      appendUniqueContextPaths(["/a"], ["/a", "/b", "/c"], 2),
    ).toEqual(["/a", "/b"]);
  });
});

describe("stripTaskBotTitlePrefix", () => {
  it("removes bot emoji prefix", () => {
    expect(stripTaskBotTitlePrefix("🤖 Ship it")).toBe("Ship it");
    expect(stripTaskBotTitlePrefix("Plain")).toBe("Plain");
  });
});
