/**
 * Pure helpers for composer context attachments (files, selection, git diff,
 * terminal) before a startRun (A6.98). File-backed items become RunAttachment
 * URIs; text context is prepended to the prompt as fenced host markers.
 */

import type { RunAttachment } from "@altai/host-contract";
import type { ContextChip } from "../components/ContextChips.js";
import type { ComposerAttachFile } from "../components/ComposerAttachChips.js";

export type ComposerContextItem =
  | {
      id: string;
      kind: "file";
      uri: string;
      name: string;
      path: string;
    }
  | {
      id: string;
      kind: "selection";
      uri?: string;
      path: string;
      text: string;
      lines: number;
    }
  | {
      id: string;
      kind: "diff";
      name: string;
      text: string;
      lines: number;
    }
  | {
      id: string;
      kind: "terminal";
      name: string;
      text: string;
      lines: number;
    };

const MAX_TEXT_CHARS = 24_000;

export function countLines(text: string): number {
  if (!text) {
    return 0;
  }
  const trimmed = text.replace(/\n+$/, "");
  if (!trimmed) {
    return 0;
  }
  return trimmed.split("\n").length;
}

export function clipContextText(
  text: string,
  maxChars: number = MAX_TEXT_CHARS,
): { text: string; truncated: boolean } {
  if (text.length <= maxChars) {
    return { text, truncated: false };
  }
  return {
    text: `${text.slice(0, maxChars)}\n… (truncated)`,
    truncated: true,
  };
}

export function basenamePath(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || path;
}

export function newContextItemId(kind: string): string {
  return `${kind}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

/** Deduplicate by stable key (uri or kind+path+name). */
export function addContextItem(
  items: readonly ComposerContextItem[],
  item: ComposerContextItem,
): ComposerContextItem[] {
  const key = contextItemKey(item);
  if (items.some((existing) => contextItemKey(existing) === key)) {
    return [...items];
  }
  return [...items, item].slice(-12);
}

export function removeContextItem(
  items: readonly ComposerContextItem[],
  id: string,
): ComposerContextItem[] {
  return items.filter((item) => item.id !== id);
}

function contextItemKey(item: ComposerContextItem): string {
  if (item.kind === "file") {
    return `file:${item.uri}`;
  }
  if (item.kind === "selection") {
    return `selection:${item.path}`;
  }
  if (item.kind === "diff") {
    return `diff:${item.name}`;
  }
  return `terminal:${item.name}`;
}

export function toRunAttachments(
  items: readonly ComposerContextItem[],
): RunAttachment[] {
  return items
    .filter((item): item is Extract<ComposerContextItem, { kind: "file" }> => {
      return item.kind === "file";
    })
    .map((item) => ({
      uri: item.uri,
      name: item.name,
    }));
}

/**
 * Build fenced text blocks for non-file context (selection / diff / terminal).
 * File attachments travel as URI attachments, not prompt text.
 */
export function formatTextContextBlocks(
  items: readonly ComposerContextItem[],
): string {
  const blocks: string[] = [];
  for (const item of items) {
    if (item.kind === "file") {
      continue;
    }
    const clipped = clipContextText(item.text);
    if (item.kind === "selection") {
      blocks.push(
        [
          "```context selection",
          `path: ${item.path}`,
          clipped.text,
          "```",
        ].join("\n"),
      );
    } else if (item.kind === "diff") {
      blocks.push(
        ["```context diff", `name: ${item.name}`, clipped.text, "```"].join(
          "\n",
        ),
      );
    } else if (item.kind === "terminal") {
      blocks.push(
        [
          "```context terminal",
          `name: ${item.name}`,
          clipped.text,
          "```",
        ].join("\n"),
      );
    }
  }
  return blocks.join("\n\n");
}

export function composeRunPrompt(
  userText: string,
  items: readonly ComposerContextItem[],
): { prompt: string; attachments: RunAttachment[] } {
  const blocks = formatTextContextBlocks(items);
  const prompt = blocks
    ? `${blocks}\n\n${userText.trim()}`
    : userText.trim();
  return {
    prompt,
    attachments: toRunAttachments(items),
  };
}

export function toComposerAttachFiles(
  items: readonly ComposerContextItem[],
): ComposerAttachFile[] {
  return items.map((item) => {
    if (item.kind === "file") {
      return {
        id: item.id,
        name: item.name,
        kind: "text",
      };
    }
    if (item.kind === "selection") {
      return {
        id: item.id,
        name: basenamePath(item.path),
        kind: "selection",
        text: item.text,
        source: "editor",
      };
    }
    if (item.kind === "diff") {
      return {
        id: item.id,
        name: item.name,
        kind: "diff",
        text: item.text,
      };
    }
    return {
      id: item.id,
      name: item.name,
      kind: "terminal",
      text: item.text,
    };
  });
}

export function toContextChips(
  items: readonly ComposerContextItem[],
): ContextChip[] {
  return items.map((item) => {
    if (item.kind === "file") {
      return { kind: "file", name: item.name, lines: 0 };
    }
    if (item.kind === "selection") {
      return {
        kind: "selection",
        source: "editor",
        lines: item.lines,
      };
    }
    if (item.kind === "diff") {
      return { kind: "diff", name: item.name, lines: item.lines };
    }
    return { kind: "terminal", name: item.name, lines: item.lines };
  });
}

/**
 * URI that can open in the editor for an attachment (file or selection with uri).
 * Diff/terminal context stays text-only.
 */
export function resolveContextOpenUri(
  item: ComposerContextItem,
): string | null {
  if (item.kind === "file" && item.uri.trim()) {
    return item.uri;
  }
  if (item.kind === "selection" && item.uri && item.uri.trim()) {
    return item.uri;
  }
  return null;
}

export type OpenableContextItem = {
  id: string;
  label: string;
  uri: string;
};

/** Attachments the host can open with workspace.openFile. */
export function listOpenableContextItems(
  items: readonly ComposerContextItem[],
): OpenableContextItem[] {
  const out: OpenableContextItem[] = [];
  for (const item of items) {
    const uri = resolveContextOpenUri(item);
    if (!uri) {
      continue;
    }
    const label =
      item.kind === "file"
        ? item.name
        : item.kind === "selection"
          ? basenamePath(item.path)
          : item.id;
    out.push({ id: item.id, label, uri });
  }
  return out;
}
