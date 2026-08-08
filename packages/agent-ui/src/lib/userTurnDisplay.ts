/**
 * Pure user-turn display helpers: parse command markers written at submit
 * time and strip context XML into ContextChips (display inverse of A6.27).
 */

import type { ContextChip } from "../components/ContextChips.js";
import { stripUserContextBlocks } from "./userContextBlocks.js";

/**
 * Leading marker written by composeComposerSubmitText / slash send-prompt.
 * Captures name and optional state.
 */
export const ALTAI_COMMAND_MARKER_RE =
  /^<altai-command\s+name="([a-z0-9-]+)"(?:\s+state="([a-z]+)")?\s*\/?>(?:\n+|$)/;

/** @deprecated Use ALTAI_COMMAND_MARKER_RE — legacy Desktop alias. */
export const ALTAI_CMD_RE = ALTAI_COMMAND_MARKER_RE;

export function wrapWithCommandMarker(prompt: string, name: string): string {
  return `<altai-command name="${name}" />\n\n${prompt}`;
}

export function parseCommandMarkerPrefix(text: string): {
  commandName: string | null;
  commandState: string | null;
  rest: string;
} {
  const match = text.match(ALTAI_COMMAND_MARKER_RE);
  if (!match) {
    return { commandName: null, commandState: null, rest: text };
  }
  return {
    commandName: match[1] ?? null,
    commandState: match[2] ?? null,
    rest: text.slice(match[0].length),
  };
}

export type UserTurnDisplay = {
  commandName: string | null;
  commandState: string | null;
  /** Body after marker + context-block strip. */
  text: string;
  chips: ContextChip[];
};

/** User bubble display model from stored / streamed raw user text. */
export function prepareUserTurnDisplay(rawText: string): UserTurnDisplay {
  const parsed = parseCommandMarkerPrefix(rawText);
  const stripped = stripUserContextBlocks(parsed.rest);
  return {
    commandName: parsed.commandName,
    commandState: parsed.commandState,
    text: stripped.text,
    chips: stripped.chips,
  };
}

/** Index of the trailing text part (live mid-stream; others finalized). */
export function indexOfLastTextPart(
  parts: readonly { type?: string }[],
): number {
  for (let i = parts.length - 1; i >= 0; i -= 1) {
    if (parts[i]?.type === "text") {
      return i;
    }
  }
  return -1;
}

/**
 * Id of the assistant message receiving stream chunks, or null when idle.
 */
export function resolveStreamingAssistantMessageId(
  messages: readonly { id: string; role: string }[],
  status: string,
): string | null {
  if (status !== "streaming") {
    return null;
  }
  const last = messages[messages.length - 1];
  return last?.role === "assistant" ? last.id : null;
}
