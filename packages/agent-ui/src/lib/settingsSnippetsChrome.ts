/**
 * Map preference-store snippets into the composer catalog shape (A6.91).
 * Hosts own DEFAULT_SNIPPETS and SnippetPref persistence.
 */

import {
  mergeSnippetCatalogs,
  normalizeHandle,
  type ComposerSnippet,
} from "./composerSnippets.js";

export type SnippetPrefEntry = {
  id: string;
  handle: string;
  body: string;
};

export function prefsToComposerSnippets(
  prefs: readonly SnippetPrefEntry[],
): ComposerSnippet[] {
  return prefs
    .map((pref) => {
      const handle = normalizeHandle(pref.handle);
      if (!handle) {
        return null;
      }
      return {
        id: pref.id || `pref-${handle}`,
        handle,
        name: `#${handle}`,
        description: "Custom snippet from Settings → Agents",
        content: pref.body,
      } satisfies ComposerSnippet;
    })
    .filter((item): item is ComposerSnippet => item !== null);
}

/** Built-ins first; user snippets override same handle. */
export function mergeSnippetCatalogFromPrefs(
  defaults: readonly ComposerSnippet[],
  user: readonly SnippetPrefEntry[],
): ComposerSnippet[] {
  return mergeSnippetCatalogs(defaults, prefsToComposerSnippets(user));
}
