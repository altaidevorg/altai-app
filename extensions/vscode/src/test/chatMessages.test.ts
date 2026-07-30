import { describe, expect, it } from "vitest";
import { isChatHostMessage, parseChatRunEvent, parseChatWebviewMessage } from "../views/chatMessages.js";

describe("chat boundary validation", () => {
  it("accepts only bounded, exact webview messages", () => {
    expect(parseChatWebviewMessage({ type: "chat/send", prompt: "  inspect this  " })).toEqual({ type: "chat/send", prompt: "inspect this" });
    expect(parseChatWebviewMessage({ type: "chat/send", prompt: "ok", injected: true })).toBeUndefined();
    expect(parseChatWebviewMessage({ type: "chat/stop", runId: "run-1" })).toEqual({ type: "chat/stop", runId: "run-1" });
    expect(parseChatWebviewMessage({ type: "chat/removeContext", id: "file:1" })).toEqual({ type: "chat/removeContext", id: "file:1" });
    expect(parseChatWebviewMessage({ type: "chat/removeContext", id: "file:1", extra: true })).toBeUndefined();
  });

  it("accepts only implemented host run events", () => {
    const event = parseChatRunEvent({ chat_id: "chat-1", run_id: "run-1", seq: 2, event: { type: "agent_message", role: "assistant", content: "Hello" } });
    expect(event).toEqual({ type: "agent_message", chatId: "chat-1", runId: "run-1", seq: 2, content: "Hello" });
    expect(parseChatRunEvent({ chat_id: "chat-1", run_id: "run-1", seq: 0, event: { type: "thinking", content: "x" } })).toBeUndefined();
    expect(parseChatRunEvent({ chat_id: "chat-1", run_id: "run-1", seq: 1, event: { type: "file_read", path: "/secret" } })).toBeUndefined();
    expect(isChatHostMessage({ type: "chat/status", message: "Ready", tone: "info" })).toBe(true);
    expect(isChatHostMessage({ type: "chat/context", items: [{ id: "file:1", kind: "file", label: "a.ts", uri: "file:///app/a.ts" }] })).toBe(true);
    expect(isChatHostMessage({ type: "chat/context", items: [{ id: "file:1", kind: "shell", label: "a.ts", uri: "file:///app/a.ts" }] })).toBe(false);
  });
});
