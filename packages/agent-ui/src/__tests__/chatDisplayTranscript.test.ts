import { describe, expect, it } from "vitest";
import type { AgentEvent } from "@altai/host-contract";
import {
  appendUserMessage,
  applyAgentEventToMessages,
  displayMessagesFromSession,
  extractEditDiff,
  extractToolFileTarget,
  shouldShowChatEmptyHome,
  textFromAgentEvent,
} from "../lib/chatDisplayTranscript.js";
import { pathToFileUri as pathToFileUriCheck } from "../lib/chatHref.js";

describe("displayMessagesFromSession", () => {
  it("keeps user/assistant roles only", () => {
    const messages = displayMessagesFromSession([
      { id: "1", role: "system", content: "sys" },
      { id: "2", role: "user", content: "  hi  " },
      { id: "3", role: "tool", content: "t" },
      { id: "4", role: "assistant", content: "hello" },
    ]);
    expect(messages.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect(messages[0]?.content).toBe("hi");
  });

  it("strips command markers and context chips for user turns", () => {
    const raw = [
      '<altai-command name="init" />',
      '<terminal-context name="zsh">\nls\n</terminal-context>',
      "please review",
    ].join("\n\n");
    const [user] = displayMessagesFromSession([
      { id: "u", role: "user", content: raw },
    ]);
    expect(user?.commandName).toBe("init");
    expect(user?.chips?.some((c) => c.kind === "terminal")).toBe(true);
    expect(user?.content).toBe("please review");
  });

  it("treats only user/assistant as conversation content", () => {
    expect(shouldShowChatEmptyHome([])).toBe(true);
    expect(
      shouldShowChatEmptyHome([
        { id: "m", role: "meta", content: "status" },
      ]),
    ).toBe(true);
    expect(
      shouldShowChatEmptyHome([
        { id: "u", role: "user", content: "hi" },
      ]),
    ).toBe(false);
  });
});

describe("applyAgentEventToMessages", () => {
  it("coalesces streaming assistant deltas", () => {
    const e1: AgentEvent = {
      type: "message",
      chatId: "c1",
      runId: "r1",
      seq: 1,
      payload: { type: "agent_message", role: "assistant", content: "Hel" },
    };
    const e2: AgentEvent = {
      type: "message",
      chatId: "c1",
      runId: "r1",
      seq: 2,
      payload: { type: "agent_message", role: "assistant", content: "lo" },
    };
    let next = applyAgentEventToMessages([], e1, { activeChatId: "c1" });
    next = applyAgentEventToMessages(next, e2, { activeChatId: "c1" });
    expect(next).toHaveLength(1);
    expect(next[0]?.content).toBe("Hello");
    expect(next[0]?.streaming).toBe(true);
  });

  it("ignores events for other chats", () => {
    const base = appendUserMessage([], "hi");
    const event: AgentEvent = {
      type: "message",
      chatId: "other",
      runId: "r1",
      seq: 1,
      payload: { type: "agent_message", content: "nope" },
    };
    expect(
      applyAgentEventToMessages(base, event, { activeChatId: "c1" }),
    ).toEqual(base);
  });

  it("reads nested agent_message text", () => {
    const event: AgentEvent = {
      type: "message",
      chatId: "c1",
      runId: "r1",
      seq: 1,
      payload: {
        type: "agent_message",
        role: "assistant",
        content: "nested ok",
      },
    };
    expect(textFromAgentEvent(event)).toEqual({
      role: "assistant",
      text: "nested ok",
      done: false,
    });
  });

  it("maps tool starts with file path metadata", () => {
    const event: AgentEvent = {
      type: "tool",
      chatId: "c1",
      runId: "r1",
      seq: 3,
      payload: {
        type: "tool_call_start",
        name: "edit",
        input: { path: "/Users/me/proj/src/App.tsx" },
      },
    };
    const next = applyAgentEventToMessages([], event, { activeChatId: "c1" });
    expect(next).toHaveLength(1);
    expect(next[0]?.role).toBe("tool");
    expect(next[0]?.toolName).toBe("edit");
    expect(next[0]?.filePath).toBe("/Users/me/proj/src/App.tsx");
    expect(next[0]?.fileUri).toBe("file:///Users/me/proj/src/App.tsx");
    expect(next[0]?.content).toContain("App.tsx");
  });

  it("maps todo_write tool inputs into TodoChecklist data", () => {
    const event: AgentEvent = {
      type: "tool",
      chatId: "c1",
      runId: "r1",
      seq: 5,
      payload: {
        type: "tool_call_start",
        name: "todo_write",
        input: {
          items: [
            { id: "1", content: "Wire ports", status: "completed" },
            { id: "2", content: "Mount UI", status: "in_progress" },
          ],
        },
      },
    };
    const next = applyAgentEventToMessages([], event, { activeChatId: "c1" });
    expect(next[0]?.todos).toHaveLength(2);
    expect(next[0]?.content).toContain("1/2");
  });
});

describe("extractToolFileTarget", () => {
  it("finds nested path and builds file URIs", () => {
    expect(
      extractToolFileTarget({
        event: { input: { file_path: "/tmp/a.ts" } },
      }),
    ).toEqual({
      path: "/tmp/a.ts",
      uri: "file:///tmp/a.ts",
    });
    expect(pathToFileUriCheck("/tmp/x.ts")).toBe("file:///tmp/x.ts");
  });
});

describe("extractEditDiff", () => {
  it("reads nested edit_diff crates", () => {
    expect(
      extractEditDiff({
        edit_diff: {
          file: "/tmp/x.ts",
          before: "1",
          after: "2",
          hunk_id: "h",
        },
      }),
    ).toEqual({
      path: "/tmp/x.ts",
      originalText: "1",
      modifiedText: "2",
      hunkId: "h",
    });
  });
});
