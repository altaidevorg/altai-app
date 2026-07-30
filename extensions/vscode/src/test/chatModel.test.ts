import { describe, expect, it } from "vitest";
import { initialChatState, reduceChat } from "../views/chatModel.js";

describe("chat stream reducer", () => {
  it("renders ordered lifecycle events and releases the composer on terminal", () => {
    let state = reduceChat(initialChatState, { type: "promptSent", prompt: "Explain this project" });
    state = reduceChat(state, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "run_started", chatId: "chat", runId: "run", seq: 1 } } });
    state = reduceChat(state, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "thinking", chatId: "chat", runId: "run", seq: 2, content: "Looking…" } } });
    state = reduceChat(state, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "agent_message", chatId: "chat", runId: "run", seq: 3, content: "Here is the summary." } } });
    state = reduceChat(state, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "run_terminated", chatId: "chat", runId: "run", seq: 4, outcome: "completed" } } });
    expect(state.activeRunId).toBeUndefined();
    expect(state.entries.map((item) => item.content)).toEqual(["Explain this project", "Looking…", "Here is the summary.", "Run completed."]);
  });

  it("ignores out-of-order and foreign stream events", () => {
    const active = reduceChat(initialChatState, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "run_started", chatId: "chat", runId: "run", seq: 2 } } });
    const stale = reduceChat(active, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "thinking", chatId: "chat", runId: "run", seq: 1, content: "stale" } } });
    const foreign = reduceChat(stale, { type: "hostMessage", message: { type: "chat/run-event", event: { type: "thinking", chatId: "other", runId: "run", seq: 3, content: "foreign" } } });
    expect(foreign.entries).toHaveLength(0);
  });
});
