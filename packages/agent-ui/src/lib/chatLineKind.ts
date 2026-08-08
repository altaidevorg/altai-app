/**
 * Classify TASK-009 flat log lines for Desktop-like bubble styling (A6.96).
 */
export type ChatLineKind = "user" | "agent" | "meta";

export function chatLineKind(line: string): ChatLineKind {
  if (line.startsWith("You: ")) {
    return "user";
  }
  if (
    line.startsWith("ALTAI:") ||
    line.startsWith("Host ready") ||
    line.startsWith("Loading transcript") ||
    line.startsWith("Transcript unavailable") ||
    line.startsWith("Run cancelled") ||
    line.includes(" · chat ") ||
    line.startsWith("Opened ") ||
    line.startsWith("Focus ")
  ) {
    return "meta";
  }
  // Status / deep-link lines from chatFocusStatusLine
  if (
    line.startsWith("Session ") ||
    line.includes("focused") ||
    line.includes("Open conversation")
  ) {
    return "meta";
  }
  return "agent";
}

/** True when there is no transcript yet (show shared EmptyState home). */
export function shouldShowChatEmptyHome(lines: readonly string[]): boolean {
  return lines.length === 0;
}
