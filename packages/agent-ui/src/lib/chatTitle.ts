/**
 * Pure chat title derivation from user message text (A6.148).
 * Hosts strip tool/context XML then take the first meaningful line.
 */

export type ChatTitleMessagePart = {
  type?: string;
  text?: string;
};

export type ChatTitleMessage = {
  role?: string;
  parts?: readonly ChatTitleMessagePart[];
};

const BLOCK_TAG =
  /<(env|system-reminder|environment_details|terminal-context|git-diff|folder|selection|file|tool_call|task|instructions|context)[\s\S]*?<\/\1>\s*/gi;
const ORPHAN_TAG =
  /<\/?(env|system-reminder|environment_details|terminal-context|git-diff|folder|selection|file)[^>]*>\s*/gi;

export function stripChatTitleNoise(text: string): string {
  return text
    .replace(BLOCK_TAG, "")
    .replace(ORPHAN_TAG, "")
    .trim();
}

export function deriveChatTitleFromMessages(
  messages: readonly ChatTitleMessage[],
  emptyTitle = "New chat",
  maxLen = 40,
): string {
  for (const m of messages) {
    if (m.role !== "user") continue;
    for (const p of m.parts ?? []) {
      if (p.type !== "text" || typeof p.text !== "string") continue;
      const text = stripChatTitleNoise(p.text);
      if (!text) continue;
      const first = text
        .split("\n")
        .map((line) =>
          line
            .replace(/^```[^\n]*/g, "")
            .replace(/^\s{0,3}#+\s*/, "")
            .replace(/^\s{0,3}>\s*/, "")
            .replace(/^\s{0,3}[-*]\s*/, "")
            .trim(),
        )
        .find((line) => line.length > 0);
      if (!first) continue;
      return first.length > maxLen ? `${first.slice(0, maxLen)}…` : first;
    }
  }
  return emptyTitle;
}
