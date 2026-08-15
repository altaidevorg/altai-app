import { describe, expect, it } from "vitest";
import type {
  WorkAttempt,
  WorkInboxItem,
  WorkItem,
} from "@altai/host-contract";
import {
  ATTENTION_PRIORITY,
  attentionByWork,
  latestAttemptByWork,
  projectWorkBoard,
  toWorkBoardRow,
} from "./workBoardProjection";

function work(id: string, state: WorkItem["state"], updatedAtMs: number): WorkItem {
  return {
    id,
    projectId: "project_1",
    title: `Work ${id}`,
    description: "",
    acceptanceCriteria: "",
    kind: "task",
    state,
    revision: 1,
    createdAtMs: updatedAtMs - 10_000,
    updatedAtMs,
  };
}

function attempt(
  workId: string,
  number: number,
  phase: WorkAttempt["phase"],
  updatedAtMs = number * 100,
): WorkAttempt {
  return {
    id: `${workId}-attempt-${number}`,
    workId,
    number,
    role: "executor",
    phase,
    createdAtMs: updatedAtMs - 50,
    updatedAtMs,
  };
}

function inbox(workId: string, kind: WorkInboxItem["kind"]): WorkInboxItem {
  return {
    id: `inbox-${workId}-${kind}`,
    workId,
    kind,
    title: `Work ${workId}`,
    why: "Needs you",
    createdAtMs: 5,
  };
}

describe("latestAttemptByWork", () => {
  it("keeps the highest-numbered attempt per work", () => {
    const latest = latestAttemptByWork([
      attempt("w1", 1, "succeeded"),
      attempt("w1", 3, "running"),
      attempt("w1", 2, "failed"),
      attempt("w2", 1, "queued"),
    ]);
    expect(latest.get("w1")?.number).toBe(3);
    expect(latest.get("w2")?.number).toBe(1);
  });

  it("breaks ties on updated time so a replayed attempt still wins", () => {
    const latest = latestAttemptByWork([
      attempt("w1", 2, "failed", 1_000),
      attempt("w1", 2, "running", 2_000),
    ]);
    expect(latest.get("w1")?.phase).toBe("running");
  });
});

describe("attentionByWork", () => {
  it("keeps one attention kind per work by priority, not arrival order", () => {
    const attention = attentionByWork([
      inbox("w1", "blocked"),
      inbox("w1", "review_required"),
      inbox("w2", "question"),
    ]);
    expect(attention.get("w1")).toBe("review_required");
    expect(attention.get("w2")).toBe("question");
  });

  it("orders approval above every other kind", () => {
    const attention = attentionByWork([
      inbox("w1", "review_required"),
      inbox("w1", "failed_attempt"),
      inbox("w1", "approval"),
    ]);
    expect(attention.get("w1")).toBe("approval");
  });

  it("exposes the full priority order for documentation and tests", () => {
    expect(ATTENTION_PRIORITY[0]).toBe("approval");
    expect(ATTENTION_PRIORITY).toContain("blocked");
  });
});

describe("toWorkBoardRow", () => {
  it("keeps status, execution phase, and attention as distinct fields", () => {
    const row = toWorkBoardRow({
      work: work("w1", "in_review", 9_000),
      attempt: attempt("w1", 2, "waiting"),
      attention: "review_required",
    });
    expect(row.status).toBe("in_review");
    expect(row.executionPhase).toBe("waiting");
    expect(row.attention).toBe("review_required");
  });

  it("leaves phase and attention explicitly null when absent, not placeholder text", () => {
    const row = toWorkBoardRow({
      work: work("w1", "backlog", 9_000),
      attempt: null,
      attention: null,
    });
    expect(row.executionPhase).toBeNull();
    expect(row.attention).toBeNull();
  });
});

describe("projectWorkBoard", () => {
  it("projects server rows into board rows newest-first", () => {
    const rows = projectWorkBoard({
      work: [work("w1", "in_progress", 1_000), work("w2", "ready", 5_000)],
      attempts: [attempt("w1", 1, "running")],
      inbox: [inbox("w2", "question")],
    });
    expect(rows.map((r) => r.id)).toEqual(["w2", "w1"]);
    expect(rows[0].status).toBe("ready");
    expect(rows[0].executionPhase).toBeNull();
    expect(rows[0].attention).toBe("question");
    expect(rows[1].executionPhase).toBe("running");
    expect(rows[1].attention).toBeNull();
  });
});
