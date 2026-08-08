import { describe, expect, it } from "vitest";
import {
  deriveAgentStatusMeta,
  formatAgentStepLabel,
  isRecoverableRunAttention,
  type AgentStatusMessage,
} from "../lib/agentStatusChrome.js";

function msg(
  partial: AgentStatusMessage & { id?: string },
): AgentStatusMessage {
  return {
    role: partial.role,
    ...(partial.streaming !== undefined ? { streaming: partial.streaming } : {}),
    ...(partial.toolName !== undefined ? { toolName: partial.toolName } : {}),
  };
}

describe("deriveAgentStatusMeta", () => {
  it("prefers pending approvals", () => {
    const meta = deriveAgentStatusMeta({
      hasActiveRun: true,
      busy: true,
      approvalsPending: 2,
      blockedMessage: null,
      warningMessage: null,
      messages: [],
    });
    expect(meta.status).toBe("awaiting-approval");
    expect(meta.approvalsPending).toBe(2);
  });

  it("surfaces blocked runs as error", () => {
    const meta = deriveAgentStatusMeta({
      hasActiveRun: false,
      busy: false,
      approvalsPending: 0,
      blockedMessage: "Run failed",
      warningMessage: null,
      messages: [],
    });
    expect(meta).toMatchObject({ status: "error", error: "Run failed" });
  });

  it("marks streaming assistant and active tool steps", () => {
    const streaming = deriveAgentStatusMeta({
      hasActiveRun: true,
      busy: true,
      approvalsPending: 0,
      blockedMessage: null,
      warningMessage: null,
      messages: [msg({ role: "assistant", streaming: true })],
    });
    expect(streaming).toMatchObject({
      status: "streaming",
      step: "Responding",
    });

    const tool = deriveAgentStatusMeta({
      hasActiveRun: true,
      busy: true,
      approvalsPending: 0,
      blockedMessage: null,
      warningMessage: null,
      messages: [msg({ role: "tool", toolName: "edit_diff" })],
    });
    expect(tool).toMatchObject({ status: "thinking", step: "edit_diff" });
  });

  it("is idle when nothing is running", () => {
    expect(
      deriveAgentStatusMeta({
        hasActiveRun: false,
        busy: false,
        approvalsPending: 0,
        blockedMessage: null,
        warningMessage: null,
        messages: [],
      }).status,
    ).toBe("idle");
  });
});

describe("formatAgentStepLabel", () => {
  it("humanizes known and snake_case tools", () => {
    expect(formatAgentStepLabel("edit_diff")).toBe("Editing file");
    expect(formatAgentStepLabel("custom_tool")).toBe("Custom Tool");
  });
});

describe("isRecoverableRunAttention", () => {
  it("detects recoverable phrasing", () => {
    expect(isRecoverableRunAttention("Run paused for input")).toBe(true);
    expect(isRecoverableRunAttention("hard failure")).toBe(false);
  });
});
