/**
 * Pure formatter: export chat transcript for clipboard paste (A6.85).
 */

export type TranscriptCopyLine = {
  role: string;
  content: string;
};

export function roleLabelForCopy(role: string): string {
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
      return "Note";
    default:
      return role;
  }
}

/**
 * Build a plain-text export of the transcript (skips empty streaming stubs).
 */
export function formatTranscriptForCopy(
  messages: readonly TranscriptCopyLine[],
): string {
  const blocks: string[] = [];
  for (const message of messages) {
    const body = message.content.trim();
    if (!body) {
      continue;
    }
    blocks.push(`${roleLabelForCopy(message.role)}:\n${body}`);
  }
  return blocks.join("\n\n").trim();
}
