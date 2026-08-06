/**
 * Pure parsing of host AgentEvent payloads into Chat interactive prompts.
 * Wave 4 / A6.17 — Desktop + VS Code share approval / clarification shapes.
 */

import type { AgentEvent, AgentEventType } from "@altai/host-contract";

export type PendingToolApproval = {
  kind: "tool";
  chatId: string;
  runId: string;
  approvalId: string;
  toolName: string;
  input?: unknown;
};

export type PendingEditDiff = {
  file: string;
  diff: string;
  truncated: boolean;
};

export type PendingClarificationPrompt = {
  kind: "clarification";
  chatId: string;
  runId: string;
  /** Optional ticket id when host provides one. */
  ticketId?: string;
  content?: string;
  choices: string[];
  editDiff: PendingEditDiff | null;
};

export type InteractivePrompt =
  | PendingToolApproval
  | PendingClarificationPrompt;

/**
 * Map native event type strings onto HostPorts AgentEventType values.
 * `approval_request` is the crate/Desktop name; host-contract uses `approval`.
 */
export function normalizeAgentEventType(rawType: string): AgentEventType {
  if (rawType === "approval_request") {
    return "approval";
  }
  const known: AgentEventType[] = [
    "message",
    "reasoning",
    "tool",
    "usage",
    "diff",
    "approval",
    "clarification",
    "subagent",
    "lifecycle",
    "notification",
    "warning",
  ];
  return known.includes(rawType as AgentEventType)
    ? (rawType as AgentEventType)
    : "lifecycle";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function unwrapBody(payload: unknown): Record<string, unknown> {
  if (!isRecord(payload)) {
    return {};
  }
  // Some adapters nest the crate event under `payload`; prefer the outer
  // record when it already carries a type tag.
  if (typeof payload.type === "string") {
    return payload;
  }
  if (isRecord(payload.payload) && typeof payload.payload.type === "string") {
    return payload.payload;
  }
  return payload;
}

function parseEditDiff(value: unknown): PendingEditDiff | null {
  if (!isRecord(value)) {
    return null;
  }
  if (typeof value.file !== "string" || typeof value.diff !== "string") {
    return null;
  }
  return {
    file: value.file,
    diff: value.diff,
    truncated: value.truncated === true,
  };
}

/**
 * Extract a pending tool approval or clarification from a host AgentEvent.
 * Returns null when the event is unrelated or malformed.
 */
export function interactivePromptFromAgentEvent(
  event: AgentEvent,
): InteractivePrompt | null {
  const body = unwrapBody(event.payload);
  const kind =
    typeof body.type === "string"
      ? body.type
      : event.type === "approval"
        ? "approval_request"
        : event.type;

  if (kind === "approval" || kind === "approval_request") {
    const approvalId =
      (typeof body.id === "string" && body.id) ||
      (typeof body.approval_id === "string" && body.approval_id) ||
      (typeof body.approvalId === "string" && body.approvalId) ||
      "";
    const toolName =
      (typeof body.action === "string" && body.action) ||
      (typeof body.tool_name === "string" && body.tool_name) ||
      (typeof body.toolName === "string" && body.toolName) ||
      "tool";
    if (!approvalId || !event.chatId || !event.runId) {
      return null;
    }
    return {
      kind: "tool",
      chatId: event.chatId,
      runId: event.runId,
      approvalId,
      toolName,
      ...(body.payload !== undefined
        ? { input: body.payload }
        : body.input !== undefined
          ? { input: body.input }
          : {}),
    };
  }

  if (kind === "clarification") {
    if (!event.chatId || !event.runId) {
      return null;
    }
    const choices = Array.isArray(body.choices)
      ? body.choices.filter((item): item is string => typeof item === "string")
      : [];
    const editDiff =
      parseEditDiff(body.edit_diff) ?? parseEditDiff(body.editDiff) ?? null;
    const ticketId =
      (typeof body.ticket_id === "string" && body.ticket_id) ||
      (typeof body.ticketId === "string" && body.ticketId) ||
      undefined;
    const content =
      typeof body.content === "string" ? body.content : undefined;
    if (!editDiff && choices.length === 0 && !content) {
      return null;
    }
    return {
      kind: "clarification",
      chatId: event.chatId,
      runId: event.runId,
      ...(ticketId ? { ticketId } : {}),
      ...(content ? { content } : {}),
      choices,
      editDiff,
    };
  }

  return null;
}

/** Merge a new prompt into local pending state (no duplicates). */
export function applyInteractivePrompt(
  currentApprovals: PendingToolApproval[],
  currentClarification: PendingClarificationPrompt | null,
  prompt: InteractivePrompt,
): {
  approvals: PendingToolApproval[];
  clarification: PendingClarificationPrompt | null;
} {
  if (prompt.kind === "tool") {
    if (
      currentApprovals.some((item) => item.approvalId === prompt.approvalId)
    ) {
      return {
        approvals: currentApprovals,
        clarification: currentClarification,
      };
    }
    return {
      approvals: [...currentApprovals, prompt],
      clarification: currentClarification,
    };
  }
  return {
    approvals: currentApprovals,
    clarification: prompt,
  };
}
