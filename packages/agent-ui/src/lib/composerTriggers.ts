/**
 * Pure composer caret triggers for `@` mentions, `#` snippets, and `/` commands.
 * Wave 4 / A6.9 — shared by Desktop and VS Code hosts.
 */

export type AtMentionRange = {
  /** Inclusive start index of the `@` in the prompt. */
  start: number;
  /** Exclusive end index of the query (typically cursor). */
  end: number;
  /** Text after `@` (may be empty). */
  query: string;
};

/**
 * Detect an open `@query` token under the caret. The token starts at the last
 * `@` after a word boundary and continues until the cursor.
 */
export function detectAtMention(
  text: string,
  cursor: number,
): AtMentionRange | null {
  if (cursor < 0 || cursor > text.length) {
    return null;
  }
  let at = -1;
  for (let i = cursor - 1; i >= 0; i -= 1) {
    const ch = text[i];
    if (ch === undefined) continue;
    if (ch === "@") {
      at = i;
      break;
    }
    if (ch === " " || ch === "\n" || ch === "\t" || ch === "\r") {
      return null;
    }
  }
  if (at < 0) {
    return null;
  }
  if (at > 0) {
    const prev = text[at - 1];
    if (
      prev !== " " &&
      prev !== "\n" &&
      prev !== "\t" &&
      prev !== "(" &&
      prev !== "["
    ) {
      return null;
    }
  }
  const query = text.slice(at + 1, cursor);
  if (/[\s]/.test(query)) {
    return null;
  }
  return { start: at, end: cursor, query };
}

/**
 * Replace the open `@query` token with nothing (attachment owns the reference).
 * Collapses surrounding double spaces after removal.
 */
export function removeAtMentionToken(
  text: string,
  mention: AtMentionRange,
): string {
  const before = text.slice(0, mention.start);
  const after = text.slice(mention.end);
  return `${before}${after}`.replace(/ {2,}/g, " ").replace(/\n +/g, "\n");
}

/** Require at least one character after `@` before searching. */
export const AT_MENTION_MIN_QUERY = 1;

export function shouldSearchAtMention(query: string): boolean {
  return query.length >= AT_MENTION_MIN_QUERY && query.length <= 128;
}

/** Normalize path separators for the shared suggestion list. */
export function pathForSuggestionList(path: string): string {
  return path.replace(/\\/g, "/");
}

export function nextAtMentionIndex(
  key: string,
  activeIndex: number,
  fileCount: number,
): { activeIndex: number; pick: boolean; dismiss: boolean } {
  if (fileCount === 0) {
    if (key === "Escape") {
      return { activeIndex, pick: false, dismiss: true };
    }
    return { activeIndex, pick: false, dismiss: false };
  }
  if (key === "ArrowDown") {
    return {
      activeIndex: Math.min(fileCount - 1, activeIndex + 1),
      pick: false,
      dismiss: false,
    };
  }
  if (key === "ArrowUp") {
    return {
      activeIndex: Math.max(0, activeIndex - 1),
      pick: false,
      dismiss: false,
    };
  }
  if (key === "Enter" || key === "Tab") {
    return { activeIndex, pick: true, dismiss: false };
  }
  if (key === "Escape") {
    return { activeIndex, pick: false, dismiss: true };
  }
  return { activeIndex, pick: false, dismiss: false };
}

export type ComposerTokenTrigger = {
  start: number;
  end: number;
  query: string;
  prefix: "#" | "/";
};

/**
 * Detect open `#snippet` or `/command` token under the caret.
 * `/` only matches as the first token of the message (executable slash commands).
 */
export function detectSlashOrSnippetTrigger(
  value: string,
  caret: number,
): ComposerTokenTrigger | null {
  for (let i = caret - 1; i >= 0; i--) {
    const ch = value[i];
    if (ch === undefined) continue;
    if (ch === "#" || ch === "/") {
      // Slash commands are executable only as the first token in a message.
      if (ch === "/" && value.slice(0, i).trim()) return null;
      const prev = i === 0 ? " " : (value[i - 1] ?? " ");
      if (!/\s/.test(prev)) return null;
      const slice = value.slice(i + 1, caret);
      if (!/^[a-z0-9-]*$/i.test(slice)) return null;
      return {
        start: i,
        end: caret,
        query: slice.toLowerCase(),
        prefix: ch,
      };
    }
    if (/\s/.test(ch)) return null;
    if (!/[a-z0-9-]/i.test(ch)) return null;
  }
  return null;
}
