import { describe, expect, it } from "vitest";
import {
  pushRecentId,
  sameIdSequence,
  toggleIdInList,
} from "../lib/modelListChrome.js";

describe("toggleIdInList", () => {
  it("adds and removes", () => {
    expect(toggleIdInList(["a"], "b")).toEqual(["a", "b"]);
    expect(toggleIdInList(["a", "b"], "a")).toEqual(["b"]);
  });
});

describe("pushRecentId", () => {
  it("dedupes and clamps", () => {
    expect(pushRecentId(["b", "c"], "a", 5)).toEqual(["a", "b", "c"]);
    expect(pushRecentId(["b", "a", "c"], "a", 5)).toEqual(["a", "b", "c"]);
    expect(pushRecentId(["a", "b", "c"], "z", 2)).toEqual(["z", "a"]);
  });
});

describe("sameIdSequence", () => {
  it("compares order-sensitive", () => {
    expect(sameIdSequence(["a", "b"], ["a", "b"])).toBe(true);
    expect(sameIdSequence(["a", "b"], ["b", "a"])).toBe(false);
  });
});
