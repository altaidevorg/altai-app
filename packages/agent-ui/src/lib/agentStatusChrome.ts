/**
 * Pure helpers mapping run / transcript signals onto AgentStatusPill meta (A6.71).
 * Hosts supply busy/approvals/blocked + message roles; no vscode/Tauri.
 */

import type { AgentStatusMeta } from "../components/AgentStatusPill.js";

export type { AgentStatusMeta };

/** Minimal message shape for status (avoids host-specific DTO graphs). */
export type AgentStatusMessage = {
  role: string;
  streaming?: boolean;
  toolName?: string;
};

export type AgentStatusChromeInput = {
  hasActiveRun: boolean;
  busy: boolean;
  approvalsPending: number;
  blockedMessage: string | null;
  warningMessage: string | null;
  messages: readonly AgentStatusMessage[];
};

/**
 * Map run / approval / transcript signals onto shared AgentStatusMeta.
 * Prefer interactive approvals, then terminal block, then stream/tool step.
 */
export function deriveAgentStatusMeta(
  input: AgentStatusChromeInput,
): AgentStatusMeta {
  if (input.approvalsPending > 0) {
    return {
      status: "awaiting-approval",
      step: null,
      approvalsPending: input.approvalsPending,
      error: null,
      activeSubagentCount: 0,
    };
  }

  if (input.blockedMessage?.trim()) {
    return {
      status: "error",
      step: null,
      approvalsPending: 0,
      error: input.blockedMessage.trim(),
      activeSubagentCount: 0,
    };
  }

  if (input.warningMessage?.trim()) {
    return {
      status: "error",
      step: null,
      approvalsPending: 0,
      error: input.warningMessage.trim(),
      activeSubagentCount: 0,
    };
  }

  const streamStep = latestStreamingStep(input.messages);
  if (streamStep) {
    return {
      status: "streaming",
      step: streamStep,
      approvalsPending: 0,
      error: null,
      activeSubagentCount: 0,
    };
  }

  if (input.hasActiveRun || input.busy) {
    const toolStep = latestToolStep(input.messages);
    return {
      status: "thinking",
      step: toolStep,
      approvalsPending: 0,
      error: null,
      activeSubagentCount: 0,
    };
  }

  return {
    status: "idle",
    step: null,
    approvalsPending: 0,
    error: null,
    activeSubagentCount: 0,
  };
}

/** Friendly tool / step labels for the pill (unknown names pass through). */
export function formatAgentStepLabel(step: string): string {
  const trimmed = step.trim();
  if (!trimmed) {
    return trimmed;
  }
  const known: Record<string, string> = {
    edit_diff: "Editing file",
    todo_write: "Updating todos",
    read_file: "Reading file",
    write_file: "Writing file",
    bash: "Running command",
    shell: "Running command",
    search: "Searching",
  };
  if (known[trimmed]) {
    return known[trimmed];
  }
  if (trimmed.includes("_") || trimmed.includes("-")) {
    return trimmed
      .split(/[_-]+/)
      .filter(Boolean)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join(" ");
  }
  return trimmed;
}

export function isRecoverableRunAttention(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.startsWith("run paused") ||
    lower.includes("needs attention") ||
    lower.startsWith("warning")
  );
}

function latestStreamingStep(
  messages: readonly AgentStatusMessage[],
): string | null {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (!message?.streaming) {
      continue;
    }
    if (message.role === "tool" && message.toolName) {
      return message.toolName;
    }
    if (message.role === "assistant") {
      return "Responding";
    }
  }
  return null;
}

function latestToolStep(
  messages: readonly AgentStatusMessage[],
): string | null {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (message?.role === "tool" && message.toolName) {
      return message.toolName;
    }
  }
  return null;
}
