/**
 * Pure helpers for AI-SDK assistant turn run actions (A6.41 / A6.48).
 */

import type { ToolLikePart } from "./transcriptToolGroups.js";
import { toolNameOf } from "./transcriptToolGroups.js";

/** Stop/retry footer visibility on an assistant bubble. */
export function shouldShowAssistantRunActions(input: {
  streaming: boolean;
  canRetry?: boolean;
}): boolean {
  return resolveAssistantRunActionMode(input) !== "hidden";
}

/** Footer control mode for stop vs retry vs none (A6.48). */
export type AssistantRunActionMode = "stop" | "retry" | "hidden";

export function resolveAssistantRunActionMode(input: {
  streaming: boolean;
  canRetry?: boolean;
}): AssistantRunActionMode {
  if (input.streaming) return "stop";
  if (input.canRetry) return "retry";
  return "hidden";
}

/**
 * Single read_file tool rows render as TranscriptReadRow (not approval card).
 */
export function isStandaloneReadToolPart(part: ToolLikePart): boolean {
  const name = toolNameOf(part);
  if (name !== "read_file") return false;
  return (part.state ?? "") !== "approval-requested";
}
