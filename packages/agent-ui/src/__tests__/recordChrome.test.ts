import { describe, expect, it } from "vitest";
import { omitListItemById, omitRecordKey } from "../lib/recordChrome.js";

describe("omitRecordKey", () => {
  it("drops one key", () => {
    expect(omitRecordKey({ a: 1, b: 2 }, "a")).toEqual({ b: 2 });
  });
});

describe("omitListItemById", () => {
  it("drops matching item", () => {
    expect(omitListItemById([{ id: "a" }, { id: "b" }], "a")).toEqual([{ id: "b" }]);
  });
});
