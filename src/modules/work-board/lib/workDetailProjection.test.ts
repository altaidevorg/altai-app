import { describe, expect, it } from "vitest";
import type {
  WorkAttempt,
  WorkInboxItem,
  WorkItem,
} from "@altai/host-contract";
import {
  toWorkDetailModel,
  toWorkGraphModel,
} from "./workDetailProjection";

function work(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    id: "w1",
    projectId: "project_1",
    title: "Ship the board",
    description: "Do it",
    acceptanceCriteria: "Gates green",
    kind: "task",
    state: "in_review",
    revision: 3,
    createdAtMs: 1_000,
    updatedAtMs: 9_000,
    ...overrides,
  };
}

function attempt(
  number: number,
  phase: WorkAttempt["phase"],
  overrides: Partial<WorkAttempt> = {},
): WorkAttempt {
  return {
    id: `w1-attempt-${number}`,
    workId: "w1",
    number,
    role: "executor",
    phase,
    createdAtMs: number * 100,
    updatedAtMs: number * 100,
    ...overrides,
  };
}

function inboxFor(workId: string, kind: WorkInboxItem["kind"]): WorkInboxItem {
  return {
    id: `inbox-${workId}`,
    workId,
    kind,
    title: "Work",
    why: "Needs you",
    createdAtMs: 5,
  };
}

describe("toWorkDetailModel", () => {
  it("keeps status, latest phase, and attention distinct on the detail", () => {
    const model = toWorkDetailModel({
      work: work(),
      attempts: [attempt(1, "failed"), attempt(2, "waiting")],
      inbox: [inboxFor("w1", "review_required"), inboxFor("other", "blocked")],
    });
    expect(model.status).toBe("in_review");
    expect(model.attention).toBe("review_required");
    expect(model.latestPhase).toBe("waiting");
  });

  it("lists attempts newest first, each with its own phase label", () => {
    const model = toWorkDetailModel({
      work: work(),
      attempts: [attempt(1, "succeeded"), attempt(2, "running")],
      inbox: [],
    });
    expect(model.attemptRows.map((row) => row.number)).toEqual([2, 1]);
    expect(model.attemptRows[0].phaseLabel).toBe("running");
    expect(model.attemptRows[1].phaseLabel).toBe("succeeded");
  });

  it("carries each attempt's recorded result as its evidence summary", () => {
    const model = toWorkDetailModel({
      work: work(),
      attempts: [
        attempt(1, "failed", {
          resultJson: '{"kind":"failed","failure":"runtime rejected the Work attempt"}',
          chatId: "chat-1",
        }),
        attempt(2, "succeeded", { resultJson: '{"kind":"completed"}' }),
      ],
      inbox: [],
    });
    // Rows are newest-first: attempt 2 above attempt 1.
    expect(model.attemptRows[0].resultSummary).toBeNull();
    expect(model.attemptRows[1].resultSummary).toBe(
      "runtime rejected the Work attempt",
    );
    expect(model.attemptRows[1].chatId).toBe("chat-1");
  });

  it("degrades unparsable or oversized results and missing bindings", () => {
    const long = "x".repeat(240);
    const model = toWorkDetailModel({
      work: work(),
      attempts: [
        attempt(1, "failed", { resultJson: "not json{" }),
        attempt(2, "failed", {
          resultJson: `{"failure":"${long}"}`,
        }),
        attempt(3, "succeeded"),
      ],
      inbox: [],
    });
    expect(model.attemptRows[0].resultSummary).toBeNull();
    expect(model.attemptRows[1].resultSummary?.length).toBe(200);
    expect(model.attemptRows[1].resultSummary?.endsWith("…")).toBe(true);
    expect(model.attemptRows[2].chatId).toBeNull();
  });

  it("leaves phase and attention null rather than placeholder text", () => {
    const model = toWorkDetailModel({
      work: work({ state: "backlog" }),
      attempts: [],
      inbox: [],
    });
    expect(model.latestPhase).toBeNull();
    expect(model.attention).toBeNull();
  });
});

describe("toWorkGraphModel", () => {
  it("maps the parent chain and children with their own status labels", () => {
    const model = toWorkGraphModel({
      work: work({ parentWorkId: "parent-1" }),
      parent: work({ id: "parent-1", title: "Campaign", state: "in_progress" }),
      children: [
        work({ id: "child-1", title: "Child one", state: "ready" }),
        work({ id: "child-2", title: "Child two", state: "done" }),
      ],
    });
    expect(model.parent).toEqual({
      id: "parent-1",
      title: "Campaign",
      stateLabel: "in progress",
    });
    expect(model.children).toEqual([
      { id: "child-1", title: "Child one", stateLabel: "ready" },
      { id: "child-2", title: "Child two", stateLabel: "done" },
    ]);
  });

  it("treats a root work as having no parent, not an unknown one", () => {
    const model = toWorkGraphModel({
      work: work({ parentWorkId: "ghost" }),
      parent: null,
      children: [],
    });
    expect(model.parent).toBeNull();
    expect(model.children).toEqual([]);
  });
});
