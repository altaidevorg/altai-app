/**
 * Pure chat history search filter (A6.226).
 */

export type HistorySessionLike = {
  id: string;
  title?: string | null;
};

/**
 * Match a session against free-text history search (title + snippet).
 * Empty query matches all; host still applies has-content gating separately.
 */
export function sessionMatchesHistorySearch(input: {
  title?: string | null;
  snippet?: string | null;
  query: string;
  defaultTitle?: string;
}): boolean {
  const q = input.query.trim().toLowerCase();
  if (!q) return true;
  const defaultTitle = input.defaultTitle ?? "New chat";
  const title = (input.title || defaultTitle).toLowerCase();
  if (title.includes(q)) return true;
  return (input.snippet ?? "").toLowerCase().includes(q);
}

/**
 * Keep sessions that have local content and match the search query.
 */
export function filterSessionsForHistorySearch<T extends HistorySessionLike>(
  sessions: readonly T[],
  options: {
    query: string;
    hasContent: Readonly<Record<string, boolean>> | ((id: string) => boolean);
    snippets: Readonly<Record<string, string | undefined | null>>;
    defaultTitle?: string;
  },
): T[] {
  const hasContent =
    typeof options.hasContent === "function"
      ? options.hasContent
      : (id: string) => Boolean(options.hasContent[id]);

  return sessions.filter((session) => {
    if (!hasContent(session.id)) return false;
    return sessionMatchesHistorySearch({
      title: session.title,
      snippet: options.snippets[session.id],
      query: options.query,
      defaultTitle: options.defaultTitle,
    });
  });
}
