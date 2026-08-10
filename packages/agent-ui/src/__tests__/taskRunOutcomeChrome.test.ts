import { describe, expect, it } from "vitest";
import { taskRunOutcomeCounts } from "../lib/taskRunOutcomeChrome.js";

describe("taskRunOutcomeCounts", () => {
  it("counts passed/failed checks", () => {
    expect(
      taskRunOutcomeCounts({
        changesCount: 3,
        verifications: [
          { status: "passed" },
          { status: "failed" },
          { status: "passed" },
          { status: "pending" },
        ],
      }),
    ).toEqual({
      changesCount: 3,
      checksPassed: 2,
      checksFailed: 1,
    });
  });
});
