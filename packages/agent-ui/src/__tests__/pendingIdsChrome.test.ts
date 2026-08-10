import { describe, expect, it } from "vitest";
import { withPendingEnded, withPendingStarted } from "../lib/pendingIdsChrome.js";

describe("withPendingStarted", () => {
  it("adds key and clears error", () => {
    expect(withPendingStarted({ a: true }, "b")).toEqual({
      error: null,
      pendingIds: { a: true, b: true },
    });
  });
});

describe("withPendingEnded", () => {
  it("removes key when present", () => {
    expect(withPendingEnded({ a: true, b: true }, "a")).toEqual({
      pendingIds: { b: true },
    });
  });
  it("returns empty patch when missing", () => {
    expect(withPendingEnded({ a: true }, "x")).toEqual({});
  });
});
