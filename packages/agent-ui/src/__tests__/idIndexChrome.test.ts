import { describe, expect, it } from "vitest";
import {
  filterUnreadBySeenAtMs,
  mapById,
  removeListValue,
} from "../lib/idIndexChrome.js";

describe("mapById", () => {
  it("indexes by id", () => {
    const map = mapById([
      { id: "a", n: 1 },
      { id: "b", n: 2 },
    ]);
    expect(map.get("b")?.n).toBe(2);
    expect(map.size).toBe(2);
  });
});

describe("filterUnreadBySeenAtMs", () => {
  it("keeps only unseen", () => {
    expect(
      filterUnreadBySeenAtMs([
        { id: "1", seenAtMs: null },
        { id: "2", seenAtMs: 1 },
      ]).map((r) => r.id),
    ).toEqual(["1"]);
  });
});

describe("removeListValue", () => {
  it("drops matching values", () => {
    expect(removeListValue(["/a", "/b", "/a"], "/a")).toEqual(["/b"]);
  });
});
