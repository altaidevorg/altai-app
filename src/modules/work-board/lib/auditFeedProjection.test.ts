import { describe, expect, it } from "vitest";
import type { AuditEvent } from "@altai/host-contract";
import { projectAuditFeed } from "./auditFeedProjection";

function event(overrides: Partial<AuditEvent> = {}): AuditEvent {
  return {
    id: 3,
    workId: "work_1",
    workTitle: "Ship the audit feed",
    kind: "state_changed",
    payloadJson: JSON.stringify({ from: "in_progress", to: "in_review" }),
    createdAtMs: 9_000,
    ...overrides,
  };
}

describe("projectAuditFeed", () => {
  it("carries the Work each fact belongs to, order preserved", () => {
    const rows = projectAuditFeed([
      event({ id: 2, workId: "work_2", workTitle: "Other Work" }),
      event(),
    ]);
    expect(rows[0]).toMatchObject({
      id: 2,
      workId: "work_2",
      workTitle: "Other Work",
    });
    expect(rows[1]).toMatchObject({ id: 3, workId: "work_1" });
  });

  it("reuses the timeline vocabulary for labels and details", () => {
    const rows = projectAuditFeed([event()]);
    expect(rows[0].label).toBe("State changed");
    expect(rows[0].detail).toBe("in progress → in review");
    expect(rows[0].atMs).toBe(9_000);
  });

  it("labels decisions and stops a governance read can rely on", () => {
    const rows = projectAuditFeed([
      event({ kind: "accepted", payloadJson: "{}" }),
      event({ kind: "returned", payloadJson: "{}" }),
      event({
        kind: "attempt_finished",
        payloadJson: JSON.stringify({ phase: "failed" }),
      }),
    ]);
    expect(rows.map((row) => row.label)).toEqual([
      "Accepted",
      "Returned",
      "Attempt failed",
    ]);
  });

  it("degrades unknown kinds to a readable label, never raw JSON", () => {
    const rows = projectAuditFeed([
      event({ kind: "governance_vote", payloadJson: "{not json" }),
    ]);
    expect(rows[0].label).toBe("governance vote");
    expect(rows[0].detail).toBeNull();
  });
});
