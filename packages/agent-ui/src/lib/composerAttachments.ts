/**
 * Pure composer attachment helpers (text context chips, draft flags, token estimate).
 * Hosts own File reading / native fs; no React, Tauri, or HostPorts.
 */

export type ComposerFileKind =
  | "image"
  | "pdf"
  | "text"
  | "selection"
  | "terminal"
  | "diff"
  | "folder";

export type ComposerFileAttachment = {
  id: string;
  name: string;
  kind: ComposerFileKind;
  mediaType: string;
  url?: string;
  text?: string;
  size: number;
  /** For kind === "selection": which surface it came from. */
  source?: "terminal" | "editor";
};

/** Max size for browser-picked text files before discarding. */
export const MAX_TEXT_INLINE = 200_000;

/** Max chars for terminal / folder / generic text context chips. */
export const MAX_CONTEXT_TEXT_CHARS = 60_000;

/** MIME/extension accept list for the open-file picker. */
export const ACCEPTED_COMPOSER_FILES =
  "image/*,.pdf,.txt,.md,.json,.yaml,.yml,.toml,.sh,.zsh,.bash,.py,.js,.jsx,.ts,.tsx,.rs,.go,.java,.c,.cpp,.h,.hpp,.html,.css,.csv,.log,.env,.config,.conf,.ini,Dockerfile,.dockerfile";

/**
 * Back-compat alias used by Desktop composer file inputs.
 * Prefer `ACCEPTED_COMPOSER_FILES` in new hosts.
 */
export const ACCEPTED_FILES = ACCEPTED_COMPOSER_FILES;

export function boundContextText(
  text: string,
  maxChars: number = MAX_CONTEXT_TEXT_CHARS,
): string {
  const trimmed = text.trim();
  if (!trimmed) {
    return "";
  }
  if (trimmed.length <= maxChars) {
    return trimmed;
  }
  return `${trimmed.slice(0, maxChars)}\n…[truncated]`;
}

/**
 * Build (or reject empty of) a text context attachment for terminal/diff/folder.
 */
export function buildTextContextAttachment(input: {
  kind: "terminal" | "diff" | "folder";
  name: string;
  text: string;
  id?: string;
  maxChars?: number;
}): ComposerFileAttachment | null {
  const bounded = boundContextText(input.text, input.maxChars);
  if (!bounded) {
    return null;
  }
  return {
    id: input.id ?? `context-${input.kind}-${input.name}`,
    name: input.name,
    kind: input.kind,
    mediaType: "text/plain",
    text: bounded,
    size: bounded.length,
  };
}

/** Upsert by id (replace matching attachment, else append). */
export function upsertComposerAttachment(
  list: readonly ComposerFileAttachment[],
  attachment: ComposerFileAttachment,
): ComposerFileAttachment[] {
  const existing = list.findIndex((file) => file.id === attachment.id);
  if (existing < 0) {
    return [...list, attachment];
  }
  const next = [...list];
  next[existing] = attachment;
  return next;
}

export function hasNativeBinaryAttachment(
  files: readonly Pick<ComposerFileAttachment, "kind">[],
): boolean {
  return files.some((file) => file.kind === "image" || file.kind === "pdf");
}

export function hasComposerDraft(input: {
  value: string;
  files: readonly unknown[];
  snippets?: readonly unknown[];
  commands?: readonly unknown[];
}): boolean {
  return (
    input.value.trim().length > 0 ||
    input.files.length > 0 ||
    (input.snippets?.length ?? 0) > 0 ||
    (input.commands?.length ?? 0) > 0
  );
}

/** Rough token estimate (~4 chars/token) for chip token badge. */
export function estimateComposerContextTokens(input: {
  files: readonly Pick<ComposerFileAttachment, "kind" | "text">[];
  snippets?: readonly { content: string }[];
}): number {
  const fileChars = input.files.reduce(
    (total, file) =>
      total + (file.kind === "image" ? 0 : (file.text?.length ?? 0)),
    0,
  );
  const snippetChars = (input.snippets ?? []).reduce(
    (total, snippet) => total + snippet.content.length,
    0,
  );
  return Math.ceil((fileChars + snippetChars) / 4);
}
