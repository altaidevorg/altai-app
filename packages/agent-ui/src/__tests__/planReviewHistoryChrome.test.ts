import { describe, expect, it } from "vitest";
import {
  checkpointHistoryRows,
  composePlanReviewHistoryRows,
  isPlanRestoreRowId,
  planAppliedHistoryRows,
  planIdFromRestoreRowId,
} from "../lib/planReviewHistoryChrome.js";

describe("planAppliedHistoryRows", () => {
  it("reverses and labels", () => {
    expect(
      planAppliedHistoryRows([
        { id: "1", path: "a.ts", isNewFile: false },
        { id: "2", path: "b.ts", isNewFile: true },
      ]),
    ).toEqual([
      {
        id: "plan-2",
        path: "b.ts",
        detail: "Accepted review · remove new file",
      },
      {
        id: "plan-1",
        path: "a.ts",
        detail: "Accepted review · restore prior content",
      },
    ]);
  });
});

describe("composePlanReviewHistoryRows", () => {
  it("joins applied + checkpoints", () => {
    const rows = composePlanReviewHistoryRows(
      [{ id: "1", path: "a.ts", isNewFile: false }],
      [{ id: "c1", path: "z.ts", label: "snap", createdMs: 1 }],
      () => "12:00",
    );
    expect(rows.map((r) => r.id)).toEqual(["plan-1", "c1"]);
    expect(rows[1]?.detail).toBe("snap · 12:00");
  });
});

describe("plan restore id helpers", () => {
  it("parses plan row ids", () => {
    expect(isPlanRestoreRowId("plan-xyz")).toBe(true);
    expect(planIdFromRestoreRowId("plan-xyz")).toBe("xyz");
    expect(checkpointHistoryRows([], () => "").length).toBe(0);
  });
});
