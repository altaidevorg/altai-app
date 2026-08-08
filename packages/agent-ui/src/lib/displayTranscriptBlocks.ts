/**
 * Flat host transcript grouping (role=tool rows → collapsible groups).
 * Wave 4 / A6.29 — same collapse rules as VS Code ChatMessageList chrome.
 * AI SDK part-based grouping remains in `transcriptToolGroups` (A6.3).
 */

import {
  groupKindFromToolName,
  normalizeToolName,
  pathBasename,
  type TranscriptGroupKind,
} from "./transcriptToolGroups.js";

export type TranscriptDisplayMessage = {
  id: string;
  role: string;
  toolName?: string;
  content: string;
  filePath?: string;
  streaming?: boolean;
};

/** Includes host-only "tools" bucket for unknown/unnamed tool rows. */
export type DisplayToolGroupKind = TranscriptGroupKind | "tools";

export type DisplayTranscriptBlock<T extends TranscriptDisplayMessage = TranscriptDisplayMessage> =
  | { kind: "message"; message: T }
  | {
      kind: "tool-group";
      groupKind: DisplayToolGroupKind;
      key: string;
      label: string;
      countLabel: string;
      preview: string | undefined;
      messages: T[];
    };

/**
 * Collapsible kind for a flat tool row, or null (not groupable / streaming).
 * Unknown completed tools → "tools".
 */
export function toolGroupKindFor(
  message: TranscriptDisplayMessage,
): DisplayToolGroupKind | null {
  if (message.role !== "tool" || message.streaming) {
    return null;
  }
  const name = normalizeToolName(message.toolName);
  if (!name) {
    return "tools";
  }
  return groupKindFromToolName(name) ?? "tools";
}

export function groupLabel(kind: DisplayToolGroupKind): string {
  switch (kind) {
    case "reads":
      return "Read";
    case "web":
      return "Web";
    case "cmd":
      return "Ran";
    default:
      return "Tools";
  }
}

export function groupCountLabel(
  kind: DisplayToolGroupKind,
  count: number,
): string {
  if (kind === "reads") {
    return count === 1 ? "1 file" : `${count} files`;
  }
  if (kind === "cmd") {
    return count === 1 ? "1 command" : `${count} commands`;
  }
  return count === 1 ? "1 call" : `${count} calls`;
}

export function groupPreview(
  kind: DisplayToolGroupKind,
  messages: readonly TranscriptDisplayMessage[],
  max = 3,
): string | undefined {
  const bits: string[] = [];
  for (const message of messages) {
    let fragment: string | undefined;
    if (kind === "reads" || kind === "tools") {
      const path = message.filePath?.trim();
      if (path) {
        fragment = pathBasename(path);
      } else {
        fragment = message.toolName || message.content.slice(0, 40) || undefined;
      }
    } else if (kind === "cmd") {
      fragment =
        message.content.split("\n")[0]?.trim().slice(0, 80) ||
        message.toolName ||
        undefined;
    } else {
      fragment = message.toolName || message.content.slice(0, 40) || undefined;
    }
    if (fragment && !bits.includes(fragment)) {
      bits.push(fragment);
    }
    if (bits.length >= max) {
      break;
    }
  }
  if (bits.length === 0) {
    return undefined;
  }
  const more =
    messages.length > bits.length
      ? `, +${messages.length - bits.length} more`
      : "";
  return `${bits.join(", ")}${more}`;
}

/**
 * Collapse consecutive groupable tool rows (≥2 same kind) into tool-groups.
 * Single tools and other roles stay as message blocks.
 */
export function buildDisplayTranscriptBlocks<
  T extends TranscriptDisplayMessage,
>(messages: readonly T[]): DisplayTranscriptBlock<T>[] {
  const out: DisplayTranscriptBlock<T>[] = [];
  let run: {
    kind: DisplayToolGroupKind;
    messages: T[];
  } | null = null;

  const flush = (): void => {
    if (!run) {
      return;
    }
    if (run.messages.length >= 2) {
      const first = run.messages[0]!;
      out.push({
        kind: "tool-group",
        groupKind: run.kind,
        key: `group-${run.kind}-${first.id}`,
        label: groupLabel(run.kind),
        countLabel: groupCountLabel(run.kind, run.messages.length),
        preview: groupPreview(run.kind, run.messages),
        messages: run.messages,
      });
    } else {
      for (const message of run.messages) {
        out.push({ kind: "message", message });
      }
    }
    run = null;
  };

  for (const message of messages) {
    const kind = toolGroupKindFor(message);
    if (kind) {
      if (run && run.kind === kind) {
        run.messages.push(message);
      } else {
        flush();
        run = { kind, messages: [message] };
      }
      continue;
    }
    flush();
    out.push({ kind: "message", message });
  }
  flush();
  return out;
}
