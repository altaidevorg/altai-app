/**
 * Pure display-transcript chrome helpers (A6.38).
 * Role labels + bubble modifiers for flat ChatDisplayMessage hosts.
 */

import type { ChatDisplayRole } from "./chatDisplayTranscript.js";

/** Accessible / visual role label for a chat bubble header. */
export function chatDisplayRoleLabel(role: string): string {
  switch (role) {
    case "user":
      return "You";
    case "assistant":
      return "ALTAI";
    case "tool":
      return "Tool";
    case "system":
      return "System";
    case "meta":
      return "";
    default:
      return "";
  }
}

/**
 * CSS modifiers under `altai-chat-bubble--*` for host stylesheets.
 * Returns just the suffix (“user”, “assistant”, …).
 */
export function chatDisplayBubbleModifier(
  role: string,
): ChatDisplayRole | "meta" {
  switch (role) {
    case "user":
    case "assistant":
    case "tool":
    case "system":
    case "meta":
      return role;
    default:
      return "meta";
  }
}

/** Full host class list for a VS Code / flat-host transcript bubble. */
export function chatDisplayBubbleClassName(role: string): string {
  return `altai-chat-bubble altai-chat-bubble--${chatDisplayBubbleModifier(role)}`;
}
