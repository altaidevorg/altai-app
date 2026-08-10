import { describe, expect, it } from "vitest";
import { sessionIdSet, sessionTitleMap } from "../lib/sessionTitleMap.js";

describe("sessionTitleMap", () => {
  it("maps ids to titles", () => {
    const map = sessionTitleMap([
      { id: "a", title: "Alpha" },
      { id: "b", title: "Beta" },
    ]);
    expect(map.get("a")).toBe("Alpha");
    expect(map.get("b")).toBe("Beta");
  });
});

describe("sessionIdSet", () => {
  it("collects ids", () => {
    expect([...sessionIdSet([{ id: "x" }, { id: "y" }])].sort()).toEqual([
      "x",
      "y",
    ]);
  });
});
