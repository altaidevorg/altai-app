import { describe, expect, it } from "vitest";
import {
  editProposalInputFromQueued,
  markPlanEditAppliedState,
} from "../lib/planQueueChrome.js";

const base = {
  id: "1",
  kind: "edit",
  path: "/a.ts",
  originalContent: "a",
  proposedContent: "b",
  isNewFile: false,
};

describe("planQueueChrome", () => {
  it("maps proposal input", () => {
    expect(editProposalInputFromQueued(base).kind).toBe("edit");
    expect(
      editProposalInputFromQueued({ ...base, isNewFile: true }).kind,
    ).toBe("create_file");
  });

  it("marks applied and drops directory creates from undo list", () => {
    const next = markPlanEditAppliedState([], [], base, 10);
    expect(next.queue).toEqual([]);
    expect(next.applied[0]).toMatchObject({ id: "1", appliedAt: 10 });
    const dir = markPlanEditAppliedState(
      [{ ...base, id: "d", kind: "create_directory" }],
      [],
      { ...base, id: "d", kind: "create_directory" },
      11,
    );
    expect(dir.applied).toEqual([]);
  });
});
