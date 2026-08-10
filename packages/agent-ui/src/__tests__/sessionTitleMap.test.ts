import { describe, expect, it } from "vitest";
import {
  sessionIdSet,
  sessionIds,
  sessionTitleMap,
} from "../lib/sessionTitleMap.js";

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

describe("sessionIds", () => {
  it("returns ordered ids", () => {
    expect(sessionIds([{ id: "x" }, { id: "y" }])).toEqual(["x", "y"]);
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
