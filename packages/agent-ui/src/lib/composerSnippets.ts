/**
 * Pure prompt snippet helpers (#handle expansion / catalog filter).
 * Hosts supply catalogs and persistence; no React or filesystem.
 */

export type ComposerSnippet = {
  id: string;
  /** The "handle" after #. Lowercase, [a-z0-9-]+. */
  handle: string;
  name: string;
  description: string;
  content: string;
};

export type PickedComposerSnippet = Pick<
  ComposerSnippet,
  "id" | "handle" | "name" | "description"
>;

const HANDLE_RE = /^[a-z0-9][a-z0-9-]*$/;

export function normalizeHandle(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function isValidHandle(h: string): boolean {
  return HANDLE_RE.test(h);
}

export function findSnippets(
  snippets: readonly ComposerSnippet[],
  query = "",
): ComposerSnippet[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return [...snippets];
  }
  return snippets.filter((snippet) =>
    [snippet.handle, snippet.name, snippet.description].some((value) =>
      value.toLowerCase().includes(normalized),
    ),
  );
}

/**
 * Replace `#handle` tokens in `text` with snippet blocks (handles stripped).
 * Unknown tokens are left as-is.
 */
export function expandSnippetTokens(
  text: string,
  snippets: readonly ComposerSnippet[],
): { body: string; blocks: string[]; matched: ComposerSnippet[] } {
  const byHandle = new Map(snippets.map((s) => [s.handle, s]));
  const matched = new Map<string, ComposerSnippet>();
  const re = /(^|\s)#([a-z0-9][a-z0-9-]*)\b/gi;
  const body = text.replace(re, (full, lead: string, raw: string) => {
    const h = raw.toLowerCase();
    const snip = byHandle.get(h);
    if (!snip) {
      return full;
    }
    matched.set(snip.id, snip);
    return lead;
  });
  const list = Array.from(matched.values());
  const blocks = list.map(
    (s) => `<snippet name="${s.handle}">\n${s.content}\n</snippet>`,
  );
  return {
    body: body
      .replace(/[ \t]{2,}/g, " ")
      .replace(/[ \t]+\n/g, "\n")
      .trim(),
    blocks,
    matched: list,
  };
}

/** Prepend expanded snippet blocks to a user prompt. */
export function composePromptWithSnippets(
  userText: string,
  catalog: readonly ComposerSnippet[],
  picked: readonly ComposerSnippet[] = [],
): { prompt: string; matched: ComposerSnippet[] } {
  const { body, matched: fromTokens } = expandSnippetTokens(userText, catalog);
  const byId = new Map<string, ComposerSnippet>();
  for (const s of fromTokens) {
    byId.set(s.id, s);
  }
  for (const s of picked) {
    byId.set(s.id, s);
  }
  const matched = Array.from(byId.values());
  if (matched.length === 0) {
    return { prompt: userText.trim(), matched: [] };
  }
  const blocks = matched.map(
    (s) => `<snippet name="${s.handle}">\n${s.content}\n</snippet>`,
  );
  const prompt = `${blocks.join("\n\n")}\n\n${body}`.trim();
  return { prompt, matched };
}

/** Replace the open `#query` with a chosen `#handle ` token. */
export function insertSnippetHandle(
  text: string,
  range: { start: number; end: number },
  handle: string,
): string {
  const token = `#${handle} `;
  return `${text.slice(0, range.start)}${token}${text.slice(range.end)}`;
}

/**
 * Parse workspace `.altai/snippets.json` (array of snippets). Unknown records
 * skipped.
 */
export function parseWorkspaceSnippetsJson(raw: string): ComposerSnippet[] {
  let data: unknown;
  try {
    data = JSON.parse(raw) as unknown;
  } catch {
    return [];
  }
  if (!Array.isArray(data)) {
    return [];
  }
  const out: ComposerSnippet[] = [];
  for (const [index, item] of data.entries()) {
    if (!item || typeof item !== "object") {
      continue;
    }
    const record = item as Record<string, unknown>;
    const rawHandle = typeof record.handle === "string" ? record.handle : "";
    if (!/^[a-z0-9][a-z0-9\s-]*$/i.test(rawHandle.trim())) {
      continue;
    }
    const handle = normalizeHandle(rawHandle);
    const content =
      typeof record.content === "string" ? record.content.trim() : "";
    if (!isValidHandle(handle) || !content) {
      continue;
    }
    const name =
      (typeof record.name === "string" && record.name.trim()) || handle;
    const description =
      (typeof record.description === "string" && record.description.trim()) ||
      "";
    const id =
      (typeof record.id === "string" && record.id.trim()) ||
      `workspace-${handle}-${index}`;
    out.push({ id, handle, name, description, content });
  }
  return out;
}

/** Merge workspace snippets over defaults (workspace handles win). */
export function mergeSnippetCatalogs(
  defaults: readonly ComposerSnippet[],
  workspace: readonly ComposerSnippet[],
): ComposerSnippet[] {
  const byHandle = new Map<string, ComposerSnippet>();
  for (const s of defaults) {
    byHandle.set(s.handle, s);
  }
  for (const s of workspace) {
    byHandle.set(s.handle, s);
  }
  return Array.from(byHandle.values()).sort((a, b) =>
    a.handle.localeCompare(b.handle),
  );
}

export function addPickedSnippet(
  picked: readonly ComposerSnippet[],
  snippet: ComposerSnippet,
): ComposerSnippet[] {
  if (picked.some((s) => s.id === snippet.id || s.handle === snippet.handle)) {
    return [...picked];
  }
  return [...picked, snippet].slice(-12);
}

export function removePickedSnippet(
  picked: readonly ComposerSnippet[],
  id: string,
): ComposerSnippet[] {
  return picked.filter((s) => s.id !== id);
}
