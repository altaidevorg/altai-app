import { describe, expect, it, vi } from "vitest";
import {
  fetchOperationsAttentionCount,
  shouldRefreshAttentionOnEvent,
} from "../lib/operationsAttentionPoll.js";

describe("shouldRefreshAttentionOnEvent", () => {
  it("matches lifecycle and notification", () => {
    expect(shouldRefreshAttentionOnEvent("lifecycle")).toBe(true);
    expect(shouldRefreshAttentionOnEvent("notification")).toBe(true);
    expect(shouldRefreshAttentionOnEvent("tool")).toBe(false);
  });
});

describe("fetchOperationsAttentionCount", () => {
  it("returns 0 without capabilities", async () => {
    const listTaskRuns = vi.fn(async () => []);
    const listNotifications = vi.fn(async () => []);
    const n = await fetchOperationsAttentionCount(
      { taskRuns: false, inbox: false },
      { listTaskRuns, listNotifications },
    );
    expect(n).toBe(0);
    expect(listTaskRuns).not.toHaveBeenCalled();
  });

  it("counts failed runs and unseen", async () => {
    const n = await fetchOperationsAttentionCount(
      { taskRuns: true, inbox: true },
      {
        listTaskRuns: async () => [
          {
            id: "1",
            title: "t",
            status: "failed",
            createdAt: "",
            updatedAt: "",
          } as never,
        ],
        listNotifications: async () => [
          {
            id: "n1",
            title: "x",
            seen: false,
            createdAt: "",
          } as never,
        ],
      },
    );
    expect(n).toBe(2);
  });
});
