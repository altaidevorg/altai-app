import { describe, expect, it } from "vitest";
import {
  partitionTasksByGroupStatus,
  taskFilterCounts,
  taskMatchesListFilter,
  taskMatchesQuery,
} from "../lib/taskFilterChrome.js";

const active = ["dispatching", "running", "awaiting-approval"];
const terminal = ["done", "failed", "cancelled"];
const rows = [
  { status: "running" },
  { status: "failed" },
  { status: "done" },
  { status: "awaiting-approval" },
];

describe("taskFilterCounts", () => {
  it("tallies buckets", () => {
    expect(taskFilterCounts(rows, active, terminal)).toEqual({
      all: 4,
      active: 2,
      attention: 2,
      finished: 2,
    });
  });
});

describe("taskMatchesListFilter", () => {
  it("matches filter ids", () => {
    expect(taskMatchesListFilter("running", "active", active, terminal)).toBe(
      true,
    );
    expect(taskMatchesListFilter("done", "finished", active, terminal)).toBe(
      true,
    );
    expect(taskMatchesListFilter("running", "attention", active, terminal)).toBe(
      false,
    );
  });
});

describe("taskMatchesQuery", () => {
  it("matches fields case-insensitively", () => {
    expect(taskMatchesQuery(["Fix Bug", "step"], "bug")).toBe(true);
    expect(taskMatchesQuery(["Fix Bug"], "zzz")).toBe(false);
  });
});

describe("partitionTasksByGroupStatus", () => {
  it("keeps non-empty groups in order", () => {
    const groups = partitionTasksByGroupStatus(rows);
    expect(groups.map((g) => g.id)).toEqual([
      "attention",
      "active",
      "ready",
    ]);
  });
});
