import { describe, expect, it } from "vitest";
import {
  catalogEntryName,
  skillsListLabel,
  sumRunTokens,
} from "../lib/taskRunCardChrome.js";

describe("sumRunTokens", () => {
  it("sums input and output", () => {
    expect(sumRunTokens({ input: 10, output: 5 })).toBe(15);
    expect(sumRunTokens(null)).toBe(0);
  });
});

describe("skillsListLabel", () => {
  it("joins or returns undefined", () => {
    expect(skillsListLabel(["a", "b"])).toBe("a, b");
    expect(skillsListLabel([])).toBeUndefined();
    expect(skillsListLabel(undefined)).toBeUndefined();
  });
});

describe("catalogEntryName", () => {
  it("resolves name with fallback", () => {
    expect(
      catalogEntryName([{ id: "1", name: "Alpha" }], "1", "Custom"),
    ).toBe("Alpha");
    expect(catalogEntryName([{ id: "1" }], "1", "Custom")).toBe("Custom");
    expect(catalogEntryName([], "x", "Custom")).toBe("Custom");
    expect(catalogEntryName([], null, "Custom")).toBeUndefined();
  });
});
