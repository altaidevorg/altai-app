import { describe, expect, it } from "vitest";
import { resolveAssignmentStatusFromRun } from "../lib/taskAssignmentStatus.js";

describe("resolveAssignmentStatusFromRun", () => {
  it("keeps terminal assignment status", () => {
    expect(
      resolveAssignmentStatusFromRun("done", {
        completed: false,
        status: "streaming",
      }),
    ).toBe("done");
  });
  it("maps completed run outcomes", () => {
    expect(
      resolveAssignmentStatusFromRun("running", {
        completed: true,
        outcome: { kind: "completed" },
      }),
    ).toBe("done");
    expect(
      resolveAssignmentStatusFromRun("running", {
        completed: true,
        outcome: { kind: "cancelled" },
      }),
    ).toBe("cancelled");
    expect(
      resolveAssignmentStatusFromRun("running", {
        completed: true,
        outcome: { kind: "failed" },
      }),
    ).toBe("failed");
  });
  it("maps live run status", () => {
    expect(
      resolveAssignmentStatusFromRun("queued", {
        completed: false,
        status: "thinking",
      }),
    ).toBe("running");
    expect(
      resolveAssignmentStatusFromRun("queued", {
        completed: false,
        status: "awaiting-approval",
      }),
    ).toBe("awaiting-approval");
    expect(
      resolveAssignmentStatusFromRun("queued", {
        completed: false,
        status: "error",
      }),
    ).toBe("failed");
  });
  it("returns assignment when no run", () => {
    expect(resolveAssignmentStatusFromRun("running", undefined)).toBe("running");
  });
});
