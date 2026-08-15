import { describe, expect, it } from "vitest";
import type { WorkEvent } from "@altai/host-contract";
import { toWorkTimeline } from "./runTimelineProjection";

function event(
  id: number,
  kind: string,
  payloadJson: string,
  createdAtMs = id * 1_000,
): WorkEvent {
  return { id, workId: "w1", kind, payloadJson, createdAtMs };
}

describe("toWorkTimeline", () => {
  it("labels each transition kind from the store's log", () => {
    const rows = toWorkTimeline([
      event(1, "created", '{"kind":"task"}'),
      event(2, "state_changed", '{"from":"backlog","to":"ready"}'),
      event(3, "attempt_started", '{"attemptId":"a1","number":2}'),
      event(4, "attempt_run_bound", '{"attemptId":"a1","runId":"run_9"}'),
      event(5, "attempt_finished", '{"attemptId":"a1","phase":"failed","from":"in_progress","to":"ready"}'),
      event(6, "accepted", '{"reviewId":"r1","to":"done"}'),
    ]);
    expect(rows.map((row) => row.label)).toEqual([
      "Created",
      "State changed",
      "Attempt 2 started",
      "Run bound",
      "Attempt failed",
      "Accepted",
    ]);
    expect(rows[1].detail).toBe("backlog → ready");
    expect(rows[3].detail).toBe("run_9");
  });

  it("keeps detail null when the event carries no typed fact", () => {
    const rows = toWorkTimeline([
      event(1, "created", '{"kind":"task"}'),
      event(2, "returned", '{"reviewId":"r1"}'),
    ]);
    expect(rows[0].detail).toBeNull();
    expect(rows[1].detail).toBeNull();
  });

  it("preserves the store's oldest-first order and row ids", () => {
    const rows = toWorkTimeline([
      event(2, "attempt_started", '{"number":1}'),
      event(1, "created", "{}"),
    ]);
    expect(rows.map((row) => row.id)).toEqual([2, 1]);
    expect(rows.map((row) => row.atMs)).toEqual([2_000, 1_000]);
  });

  it("degrades unparsable payloads to the label, never throws", () => {
    const rows = toWorkTimeline([
      event(1, "state_changed", "not json{"),
      event(2, "attempt_finished", '{"phase":42}'),
    ]);
    expect(rows[0]).toEqual({ id: 1, label: "State changed", detail: null, atMs: 1_000 });
    expect(rows[1].label).toBe("Attempt finished");
  });

  it("falls back to the raw kind for an unknown transition", () => {
    const rows = toWorkTimeline([event(1, "wake_requested", "{}")]);
    expect(rows[0].label).toBe("wake requested");
    expect(rows[0].detail).toBeNull();
  });
});
