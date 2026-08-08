/**
 * Pure composer draft / attach / slash-prelude helpers for ports-first adapters
 * (A6.30). Hosts own I/O (FileReader, native fs) and slash-command registries.
 */

import type { ComposerFileAttachment } from "./composerAttachments.js";
import { MAX_TEXT_INLINE } from "./composerAttachments.js";

/** Max browser-picked PDF size before rejection. */
export const MAX_PDF_INLINE_BYTES = 10 * 1024 * 1024;

/**
 * When the user picked slash chips but typed a normal prompt, prefix the first
 * command so tryRunSlashCommand can expand it (Desktop / VS Code shared).
 */
export function buildComposerCommandSource(
  trimmed: string,
  pickedCommandNames: readonly string[],
): string {
  if (
    pickedCommandNames.length > 0 &&
    !trimmed.startsWith("/") &&
    !trimmed.startsWith("#")
  ) {
    const head = pickedCommandNames[0]!;
    return `#${head} ${trimmed}`.trim();
  }
  return trimmed;
}

/**
 * Map an external slash-command resolver result onto submit text fields.
 * `outcome` comes from host registries (Desktop tryRunSlashCommand, VS Code…).
 */
export type ComposerSlashOutcome =
  | { kind: "none" }
  | { kind: "handled"; toast?: string }
  | { kind: "send-prompt"; prompt: string; commandName?: string };

export function applyComposerSlashOutcome(
  outcome: ComposerSlashOutcome,
  fallbackText: string,
): {
  abortAsHandled: boolean;
  toast?: string;
  effectiveText: string;
  commandMarker: string | null;
} {
  if (outcome.kind === "handled") {
    return {
      abortAsHandled: true,
      ...(outcome.toast ? { toast: outcome.toast } : {}),
      effectiveText: fallbackText,
      commandMarker: null,
    };
  }
  if (outcome.kind === "send-prompt") {
    return {
      abortAsHandled: false,
      effectiveText: outcome.prompt,
      commandMarker: outcome.commandName
        ? `<altai-command name="${outcome.commandName}" />`
        : null,
    };
  }
  return {
    abortAsHandled: false,
    effectiveText: fallbackText,
    commandMarker: null,
  };
}

/** Editor / terminal pending selection → composer file chip. */
export function selectionToComposerAttachment(input: {
  id: string;
  source: "terminal" | "editor";
  text: string;
}): ComposerFileAttachment {
  return {
    id: input.id,
    name:
      input.source === "editor" ? "Editor selection" : "Terminal selection",
    kind: "selection",
    mediaType: "text/plain",
    text: input.text,
    size: input.text.length,
    source: input.source,
  };
}

/** Append pick if not already present (by id/name identity helper). */
export function appendUniqueByKey<T>(
  list: readonly T[],
  item: T,
  keyOf: (item: T) => string,
): T[] {
  const key = keyOf(item);
  if (list.some((existing) => keyOf(existing) === key)) {
    return [...list];
  }
  return [...list, item];
}

export function removeAcceptedItems<T>(
  current: readonly T[],
  accepted: readonly T[],
): T[] {
  const drop = new Set(accepted);
  return current.filter((item) => !drop.has(item));
}

/**
 * Classifies a browser File meta for attach (no I/O). Hosts then load text/url.
 */
export type BrowserFileClass =
  | {
      ok: true;
      id: string;
      kind: "image" | "pdf" | "text";
      mediaType: string;
    }
  | { ok: false; reason: "too-large-pdf" | "too-large-text" };

export function classifyBrowserFile(file: {
  name: string;
  type: string;
  size: number;
  lastModified: number;
}): BrowserFileClass {
  const id = `${file.name}-${file.size}-${file.lastModified}`;
  if (file.type.startsWith("image/")) {
    return {
      ok: true,
      id,
      kind: "image",
      mediaType: file.type || "image/png",
    };
  }
  if (file.type === "application/pdf" || /\.pdf$/i.test(file.name)) {
    if (file.size > MAX_PDF_INLINE_BYTES) {
      return { ok: false, reason: "too-large-pdf" };
    }
    return {
      ok: true,
      id,
      kind: "pdf",
      mediaType: "application/pdf",
    };
  }
  if (file.size > MAX_TEXT_INLINE) {
    return { ok: false, reason: "too-large-text" };
  }
  return {
    ok: true,
    id,
    kind: "text",
    mediaType: file.type || "text/plain",
  };
}

/**
 * Hosts read payload then call this to assemble the attachment (shared shape).
 */
export function browserFileToAttachment(
  cls: Extract<BrowserFileClass, { ok: true }>,
  name: string,
  payload: { url?: string; text?: string; size: number },
): ComposerFileAttachment {
  if (cls.kind === "image" || cls.kind === "pdf") {
    return {
      id: cls.id,
      name,
      kind: cls.kind,
      mediaType: cls.mediaType,
      url: payload.url,
      size: payload.size,
    };
  }
  return {
    id: cls.id,
    name,
    kind: "text",
    mediaType: cls.mediaType,
    text: payload.text ?? "",
    size: payload.size,
  };
}

/** Basename of an absolute or workspace-relative path for attach chips. */
export function basenameForAttach(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts[parts.length - 1] || path;
}
