import { describe, expect, it } from "vitest";
import {
  indexLatestCronJobsByAutomationId,
  sortAutomationItemsById,
} from "../lib/automationListChrome.js";

describe("sortAutomationItemsById", () => {
  it("sorts by id stable order", () => {
    expect(sortAutomationItemsById([{ id: "b" }, { id: "a" }]).map((x) => x.id)).toEqual([
      "a",
      "b",
    ]);
  });
});

describe("indexLatestCronJobsByAutomationId", () => {
  it("indexes latest cron jobs only", () => {
    const map = indexLatestCronJobsByAutomationId([
      { id: "manual", updatedAtMs: 9 },
      { id: "cron:auto-1", updatedAtMs: 10 },
      { id: "cron:auto-1", updatedAtMs: 20 },
      { id: "cron:", updatedAtMs: 1 },
      { id: "cron:auto-2", updatedAtMs: 5 },
    ]);
    expect(Object.keys(map).sort()).toEqual(["auto-1", "auto-2"]);
    expect(map["auto-1"].updatedAtMs).toBe(20);
    expect(map["auto-2"].updatedAtMs).toBe(5);
  });
});
