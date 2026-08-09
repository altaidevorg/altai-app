import { describe, expect, it } from "vitest";
import { insertSessionAfterActive } from "../lib/insertSessionAfterActive.js";

describe("insertSessionAfterActive", () => {
  it("inserts after active", () => {
    expect(
      insertSessionAfterActive(
        [{ id: "a" }, { id: "b" }],
        "a",
        { id: "n" },
      ),
    ).toEqual([{ id: "a" }, { id: "n" }, { id: "b" }]);
  });
  it("appends when missing active", () => {
    expect(insertSessionAfterActive([{ id: "a" }], "z", { id: "n" })).toEqual([
      { id: "a" },
      { id: "n" },
    ]);
  });
});
