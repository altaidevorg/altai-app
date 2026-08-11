import { describe, expect, it, vi } from "vitest";
import type { WorkInboxItem } from "@altai/host-contract";
import {
  loadWorkInboxAttentionCount,
  WORK_INBOX_INVALIDATION_EVENTS,
  WorkInboxRequestGate,
} from "./workInboxAttention";

function item(id: string, kind: WorkInboxItem["kind"]): WorkInboxItem {
  return {
    id,
    workId: "work_1",
    kind,
    title: "Work",
    why: "Needs you",
    createdAtMs: 1,
  };
}

describe("Work Inbox attention count", () => {
  it("reconciles before counting canonical projection rows", async () => {
    const calls: string[] = [];
    const count = await loadWorkInboxAttentionCount({
      reconcile: async () => {
        calls.push("reconcile");
      },
      list: async () => {
        calls.push("list");
        return [item("review:1", "review_required")];
      },
    });
    expect(count).toBe(1);
    expect(calls).toEqual(["reconcile", "list"]);
  });

  it("keeps persisted attention visible when reconciliation fails", async () => {
    await expect(
      loadWorkInboxAttentionCount({
        reconcile: async () => {
          throw new Error("runtime unavailable");
        },
        list: async () => [item("blocked:1", "blocked")],
      }),
    ).resolves.toBe(1);
  });

  it("preserves the last known count when the projection transiently fails", async () => {
    await expect(
      loadWorkInboxAttentionCount({
        reconcile: vi.fn(async () => undefined),
        list: async () => {
          throw new Error("work.db unavailable");
        },
      }),
    ).resolves.toBeNull();
  });

  it("rejects an older response after a newer request begins", () => {
    const gate = new WorkInboxRequestGate("/workspace-a");
    const older = gate.begin("/workspace-a");
    const newer = gate.begin("/workspace-a");
    expect(gate.isCurrent(older)).toBe(false);
    expect(gate.isCurrent(newer)).toBe(true);
  });

  it("invalidates the old workspace epoch before the new request resolves", () => {
    const gate = new WorkInboxRequestGate("/workspace-a");
    const oldWorkspace = gate.begin("/workspace-a");
    gate.reset("/workspace-b");
    const newWorkspace = gate.begin("/workspace-b");
    expect(gate.isCurrent(oldWorkspace)).toBe(false);
    expect(gate.isCurrent(newWorkspace)).toBe(true);
  });

  it("does not let a late old-workspace request invalidate the new workspace", () => {
    const gate = new WorkInboxRequestGate("/workspace-a");
    gate.reset("/workspace-b");
    const newWorkspace = gate.begin("/workspace-b");
    const lateOldWorkspace = gate.begin("/workspace-a");
    expect(gate.isCurrent(lateOldWorkspace)).toBe(false);
    expect(gate.isCurrent(newWorkspace)).toBe(true);
  });

  it("refreshes for Work mutations and durable terminal journal events", () => {
    expect(WORK_INBOX_INVALIDATION_EVENTS).toEqual([
      "altai:work-inbox-changed",
      "altai:agent-terminal-journaled",
    ]);
  });
});
