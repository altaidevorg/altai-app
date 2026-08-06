import { describe, expect, it } from "vitest";
import type { AgentEvent } from "@altai/host-contract";
import {
  applyInteractivePrompt,
  interactivePromptFromAgentEvent,
  normalizeAgentEventType,
} from "../lib/interactivePrompt.js";

function event(
  partial: Partial<AgentEvent> & Pick<AgentEvent, "type">,
): AgentEvent {
  return {
    chatId: "chat-1",
    runId: "run-1",
    seq: 1,
    payload: null,
    ...partial,
  };
}

describe("normalizeAgentEventType", () => {
  it("maps approval_request onto approval", () => {
    expect(normalizeAgentEventType("approval_request")).toBe("approval");
    expect(normalizeAgentEventType("clarification")).toBe("clarification");
    expect(normalizeAgentEventType("unknown")).toBe("lifecycle");
  });
});

describe("interactivePromptFromAgentEvent", () => {
  it("parses nested approval_request payloads", () => {
    const prompt = interactivePromptFromAgentEvent(
      event({
        type: "approval",
        payload: {
          type: "approval_request",
          id: "appr-1",
          action: "write_file",
          payload: { path: "a.ts" },
        },
      }),
    );
    expect(prompt).toEqual({
      kind: "tool",
      chatId: "chat-1",
      runId: "run-1",
      approvalId: "appr-1",
      toolName: "write_file",
      input: { path: "a.ts" },
    });
  });

  it("parses clarification with edit diff and choices", () => {
    const prompt = interactivePromptFromAgentEvent(
      event({
        type: "clarification",
        payload: {
          type: "clarification",
          content: "Apply this edit?",
          choices: ["approve", "deny"],
          edit_diff: {
            file: "a.ts",
            diff: "+x",
            truncated: false,
          },
        },
      }),
    );
    expect(prompt).toMatchObject({
      kind: "clarification",
      content: "Apply this edit?",
      choices: ["approve", "deny"],
      editDiff: { file: "a.ts", diff: "+x", truncated: false },
    });
  });

  it("returns null for unrelated events", () => {
    expect(
      interactivePromptFromAgentEvent(
        event({ type: "message", payload: { text: "hi" } }),
      ),
    ).toBeNull();
  });
});

describe("applyInteractivePrompt", () => {
  it("dedupes tool approvals by id", () => {
    const first = interactivePromptFromAgentEvent(
      event({
        type: "approval",
        payload: {
          type: "approval_request",
          id: "a",
          action: "bash_run",
        },
      }),
    );
    expect(first?.kind).toBe("tool");
    if (!first || first.kind !== "tool") {
      return;
    }
    const once = applyInteractivePrompt([], null, first);
    const twice = applyInteractivePrompt(once.approvals, null, first);
    expect(once.approvals).toHaveLength(1);
    expect(twice.approvals).toHaveLength(1);
  });
});
