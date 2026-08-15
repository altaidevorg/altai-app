import { describe, expect, it } from "vitest";
import type { WorkRun } from "@altai/host-contract";
import { projectRunsHub } from "./runsHubProjection";

function run(overrides: Partial<WorkRun> = {}): WorkRun {
  return {
    id: "a1",
    workId: "w1",
    workTitle: "Ship the hub",
    workState: "in_review",
    number: 2,
    role: "executor",
    phase: "succeeded",
    createdAtMs: 1_000,
    updatedAtMs: 9_000,
    ...overrides,
  };
}

describe("projectRunsHub", () => {
  it("keeps work status and attempt phase distinct on each row", () => {
    const rows = projectRunsHub([run()]);
    expect(rows[0].status).toBe("in_review");
    expect(rows[0].statusLabel).toBe("in review");
    expect(rows[0].phase).toBe("succeeded");
    expect(rows[0].phaseLabel).toBe("succeeded");
    expect(rows[0].attemptLabel).toBe("Attempt 2");
  });

  it("carries the work id so a row can open the Work detail", () => {
    const rows = projectRunsHub([run({ workId: "w-9" })]);
    expect(rows[0].workId).toBe("w-9");
    expect(rows[0].workTitle).toBe("Ship the hub");
  });

  it("preserves the server's newest-first order", () => {
    const rows = projectRunsHub([
      run({ id: "a2", updatedAtMs: 9_000 }),
      run({ id: "a1", updatedAtMs: 1_000 }),
    ]);
    expect(rows.map((row) => row.id)).toEqual(["a2", "a1"]);
    expect(rows.map((row) => row.updatedMs)).toEqual([9_000, 1_000]);
  });

  it("returns an empty hub for an empty workspace", () => {
    expect(projectRunsHub([])).toEqual([]);
  });
});
