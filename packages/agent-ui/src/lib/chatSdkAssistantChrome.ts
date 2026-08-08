/**
 * Pure helpers for AI-SDK assistant turn run actions (A6.41).
 */

import type { ToolLikePart } from "./transcriptToolGroups.js";
import { toolNameOf } from "./transcriptToolGroups.js";

/** Stop/retry footer visibility on an assistant bubble. */
export function shouldShowAssistantRunActions(input: {
  streaming: boolean;
  canRetry?: boolean;
}): boolean {
  return input.streaming || Boolean(input.canRetry);
}

/**
 * Single read_file tool rows render as TranscriptReadRow (not approval card).
 */
export function isStandaloneReadToolPart(part: ToolLikePart): boolean {
  const name = toolNameOf(part);
  if (name !== "read_file") return false;
  return (part.state ?? "") !== "approval-requested";
}
