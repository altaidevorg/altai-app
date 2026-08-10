/**
 * Pure composer #snippet / @file picker filters (A6.241).
 */

export type SnippetPickerFields = {
  handle: string;
  name: string;
  description: string;
};

/**
 * Filter snippet candidates for the `#` picker.
 * Handle match stays case-sensitive (Desktop parity); name/description are
 * case-insensitive.
 */
export function filterSnippetsForPicker<T extends SnippetPickerFields>(
  snippets: readonly T[],
  query: string,
): T[] {
  const q = query.trim();
  if (!q) return [...snippets];
  const qLower = q.toLowerCase();
  return snippets.filter(
    (snippet) =>
      snippet.handle.includes(q) ||
      snippet.name.toLowerCase().includes(qLower) ||
      snippet.description.toLowerCase().includes(qLower),
  );
}

/**
 * Filter workspace file paths for the `@` picker, capped to `maxResults`.
 */
export function filterWorkspacePathsForPicker(
  files: readonly string[],
  query: string,
  maxResults = 30,
): string[] {
  const q = query.trim().toLowerCase();
  if (!q) return files.slice(0, maxResults);
  const out: string[] = [];
  for (const file of files) {
    if (file.toLowerCase().includes(q)) {
      out.push(file);
      if (out.length >= maxResults) break;
    }
  }
  return out;
}
