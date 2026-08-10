import { describe, expect, it } from "vitest";
import {
  sessionHistoryItemsFromSessions,
  trimmedSessionRenameTitle,
} from "../lib/sessionHistory.js";

describe("sessionHistoryItemsFromSessions", () => {
  it("maps titles and snippets with default title", () => {
    expect(
      sessionHistoryItemsFromSessions(
        [
          { id: "a", title: "Hello", updatedAt: 2 },
          { id: "b", title: "", updatedAt: 1 },
        ],
        { a: "snip" },
      ),
    ).toEqual([
      { id: "a", title: "Hello", updatedAt: 2, snippet: "snip" },
      { id: "b", title: "New chat", updatedAt: 1, snippet: undefined },
    ]);
  });
});

describe("trimmedSessionRenameTitle", () => {
  it("returns null for blank", () => {
    expect(trimmedSessionRenameTitle("  ")).toBeNull();
    expect(trimmedSessionRenameTitle("  Ship  ")).toBe("Ship");
  });
});
