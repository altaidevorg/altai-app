import { describe, expect, it } from "vitest";
import { renameSessionInList } from "../lib/renameSessionInList.js";

describe("renameSessionInList", () => {
  it("renames and stamps updatedAt", () => {
    const out = renameSessionInList(
      [{ id: "a", title: "Old", updatedAt: 1 }],
      "a",
      "New",
      99,
    );
    expect(out).toEqual([{ id: "a", title: "New", updatedAt: 99 }]);
  });
});
