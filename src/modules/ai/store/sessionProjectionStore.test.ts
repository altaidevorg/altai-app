import { beforeEach, describe, expect, it } from "vitest";
import { useSessionProjectionStore } from "./sessionProjectionStore";

const projection = (seq: number, runStatus = "running") => ({
  seq,
  timestampRfc3339: "2026-08-15T10:00:00Z",
  runStatus,
  todos: [],
  subagents: [],
  jobs: [],
});

describe("sessionProjectionStore", () => {
  beforeEach(() => useSessionProjectionStore.setState({ bySession: {} }));

  it("accepts only newer snapshots per session", () => {
    const store = useSessionProjectionStore.getState();
    expect(store.apply("chat-1", projection(2))).toBe(true);
    expect(store.apply("chat-1", projection(2, "completed"))).toBe(false);
    expect(store.apply("chat-1", projection(1, "completed"))).toBe(false);
    expect(store.apply("chat-1", projection(3, "completed"))).toBe(true);
    expect(useSessionProjectionStore.getState().bySession["chat-1"]).toMatchObject({
      seq: 3,
      runStatus: "completed",
    });
  });

  it("tracks session sequences independently", () => {
    const store = useSessionProjectionStore.getState();
    expect(store.apply("chat-a", projection(5))).toBe(true);
    expect(store.apply("chat-b", projection(1))).toBe(true);
    expect(useSessionProjectionStore.getState().bySession["chat-b"].seq).toBe(1);
  });
});
