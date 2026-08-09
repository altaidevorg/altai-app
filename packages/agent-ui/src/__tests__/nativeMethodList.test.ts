import { describe, expect, it } from "vitest";
import {
  nativeMethodAvailable,
  parseNativeMethodList,
} from "../lib/nativeMethodList.js";

describe("parseNativeMethodList", () => {
  it("accepts string arrays only", () => {
    expect(parseNativeMethodList(["a", "b"])).toEqual(["a", "b"]);
    expect(parseNativeMethodList(null)).toBe(null);
    expect(parseNativeMethodList(["a", 1])).toBe(null);
    expect(parseNativeMethodList({})).toBe(null);
  });
});

describe("nativeMethodAvailable", () => {
  it("pending list unlocks; empty locks; partial matches", () => {
    expect(nativeMethodAvailable(null, "x")).toBe(true);
    expect(nativeMethodAvailable(undefined, "x")).toBe(true);
    expect(nativeMethodAvailable([], "x")).toBe(false);
    expect(nativeMethodAvailable(["x"], "x")).toBe(true);
    expect(nativeMethodAvailable(["y"], "x")).toBe(false);
  });
});
