import { describe, expect, it } from "vitest";
import {
  automationFilterCounts,
  automationMatchesFilter,
  automationMatchesQuery,
  compareAutomationsForList,
} from "../lib/automationFilterChrome.js";

const items = [
  { id: "a", message: "hello", chatId: "c1", schedule: { kind: "at" } },
  { id: "b", message: "world", chatId: "c2", schedule: { kind: "every" } },
  { id: "c", message: "oops", chatId: "c3", schedule: { kind: "every" } },
];
const jobs = {
  c: { lastError: "failed" },
};

describe("automationFilterCounts", () => {
  it("tallies kinds and issues", () => {
    expect(automationFilterCounts(items, jobs)).toEqual({
      all: 3,
      once: 1,
      repeat: 2,
      issues: 1,
    });
  });
});

describe("automationMatchesFilter", () => {
  it("filters by once/repeat/issues", () => {
    expect(automationMatchesFilter(items[0], "once", jobs)).toBe(true);
    expect(automationMatchesFilter(items[1], "once", jobs)).toBe(false);
    expect(automationMatchesFilter(items[2], "issues", jobs)).toBe(true);
    expect(automationMatchesFilter(items[0], "issues", jobs)).toBe(false);
  });
});

describe("automationMatchesQuery", () => {
  it("matches message case-insensitively", () => {
    expect(automationMatchesQuery(items[0], "Hel", "t", "s", "")).toBe(true);
    expect(automationMatchesQuery(items[0], "zzz", "t", "s", "")).toBe(false);
  });
});

describe("compareAutomationsForList", () => {
  it("prioritizes failures", () => {
    const sorted = [...items].sort((l, r) =>
      compareAutomationsForList(l, r, jobs, () => 0),
    );
    expect(sorted[0].id).toBe("c");
  });
});
