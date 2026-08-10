import { describe, expect, it } from "vitest";
import { chatTabsFromOpenIds } from "../lib/chatTabsFromOpenIds.js";

describe("chatTabsFromOpenIds", () => {
  it("preserves open order and drops missing ids", () => {
    expect(
      chatTabsFromOpenIds(
        ["b", "missing", "a"],
        [
          { id: "a", title: "Alpha" },
          { id: "b", title: "Beta" },
        ],
      ),
    ).toEqual([
      { id: "b", title: "Beta" },
      { id: "a", title: "Alpha" },
    ]);
  });
});
