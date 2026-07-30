import { describe, expect, it } from "vitest";
import { chatRunDeepLink, RunProjectionStore } from "../views/runProjection.js";
import type { ChatRunEvent } from "../views/chatMessages.js";

const event = (type: ChatRunEvent["type"], seq: number, overrides: Record<string, unknown> = {}): ChatRunEvent => {
  const base = { chatId: "chat-a", runId: "run-a", seq, ...overrides };
  switch (type) {
    case "run_started": return { type, ...base } as ChatRunEvent;
    case "thinking": return { type, ...base, content: "Thinking" } as ChatRunEvent;
    case "agent_message": return { type, ...base, content: "Answer" } as ChatRunEvent;
    case "tool_call_start": return { type, ...base, toolId: "tool-a", name: "read_file" } as ChatRunEvent;
    case "run_terminated": return { type, ...base, outcome: "completed", ...overrides } as ChatRunEvent;
  }
};

describe("run projection", () => {
  it("deduplicates stale and repeated events for one Chat run identity", () => {
    const store = new RunProjectionStore();
    expect(store.ingest(event("run_started", 1))).toBe(true);
    expect(store.ingest(event("thinking", 2))).toBe(true);
    expect(store.ingest(event("thinking", 2))).toBe(false);
    expect(store.ingest(event("agent_message", 1))).toBe(false);
    const [active] = store.snapshot().active;
    expect(active).toMatchObject({ chatId: "chat-a", runId: "run-a", lastSeq: 2, title: "ALTAI is thinking" });
  });

  it("orders the most recently updated runs first", () => {
    const store = new RunProjectionStore();
    store.ingest(event("run_started", 1));
    store.ingest(event("run_started", 1, { chatId: "chat-b", runId: "run-b" }));
    expect(store.snapshot().active.map((run) => run.key)).toEqual(["chat-b:run-b", "chat-a:run-a"]);
  });

  it("projects terminal results into history and only failures into Inbox attention", () => {
    const store = new RunProjectionStore();
    store.ingest(event("run_started", 1));
    store.ingest(event("run_terminated", 2, { outcome: "failed" }));
    store.ingest(event("run_started", 1, { chatId: "chat-b", runId: "run-b" }));
    store.ingest(event("run_terminated", 2, { chatId: "chat-b", runId: "run-b", outcome: "completed" }));
    store.ingest(event("run_started", 1, { chatId: "chat-c", runId: "run-c" }));
    store.ingest(event("run_terminated", 2, { chatId: "chat-c", runId: "run-c", outcome: "approval_required" }));
    const projection = store.snapshot();
    expect(projection.history.map((run) => run.key)).toEqual(["chat-c:run-c", "chat-b:run-b", "chat-a:run-a"]);
    expect(projection.attention.map((run) => [run.key, run.attention])).toEqual([
      ["chat-c:run-c", "unsupported_attention"],
      ["chat-a:run-a", "failure"],
    ]);
  });

  it("uses a Chat command deep link that carries the exact shared identity", () => {
    expect(chatRunDeepLink({ chatId: "chat-a", runId: "run-a" })).toEqual({
      command: "altai.revealRun",
      title: "Reveal in Chat",
      arguments: [{ chatId: "chat-a", runId: "run-a" }],
    });
  });
});
