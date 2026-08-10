import { describe, expect, it } from "vitest";
import {
  filterSessionsForHistorySearch,
  sessionMatchesHistorySearch,
} from "../lib/sessionHistorySearch.js";

describe("sessionMatchesHistorySearch", () => {
  it("matches empty query", () => {
    expect(
      sessionMatchesHistorySearch({ title: "x", query: "  " }),
    ).toBe(true);
  });

  it("matches title and snippet case-insensitively", () => {
    expect(
      sessionMatchesHistorySearch({ title: "Hello", query: "hel" }),
    ).toBe(true);
    expect(
      sessionMatchesHistorySearch({
        title: "Other",
        snippet: "Ship the fix",
        query: "SHIP",
      }),
    ).toBe(true);
    expect(
      sessionMatchesHistorySearch({ title: "Other", snippet: "", query: "z" }),
    ).toBe(false);
  });

  it("uses default title when unset", () => {
    expect(
      sessionMatchesHistorySearch({ title: "", query: "new" }),
    ).toBe(true);
  });
});

describe("filterSessionsForHistorySearch", () => {
  it("drops empty drafts and applies search", () => {
    const rows = [
      { id: "a", title: "Alpha" },
      { id: "b", title: "Beta" },
      { id: "c", title: "Gamma" },
    ];
    expect(
      filterSessionsForHistorySearch(rows, {
        query: "alph",
        hasContent: { a: true, b: true, c: false },
        snippets: { a: "", b: "zzz", c: "alpha" },
      }).map((s) => s.id),
    ).toEqual(["a"]);
  });
});
