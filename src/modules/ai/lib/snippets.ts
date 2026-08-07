import { createAppStore } from "@/lib/appStore";
import {
  expandSnippetTokens as expandSnippetTokensShared,
  isValidHandle,
  normalizeHandle,
  type ComposerSnippet,
} from "@altai/agent-ui";

/**
 * Desktop snippet record. Shape matches shared `ComposerSnippet` so host and
 * package expansion stay interchangeable.
 */
export type Snippet = ComposerSnippet;

const STORE_PATH = "altai-ai-snippets.json";
const KEY_LIST = "snippets";

const store = createAppStore(STORE_PATH, { defaults: {}, autoSave: 200 });

export async function loadSnippets(): Promise<Snippet[]> {
  return (await store.get<Snippet[]>(KEY_LIST)) ?? [];
}

export async function saveSnippets(list: Snippet[]): Promise<void> {
  await store.set(KEY_LIST, list);
  await store.save();
}

export function newSnippetId(): string {
  return `sn-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export { isValidHandle, normalizeHandle };

/**
 * Desktop-facing wrapper: same #handle expansion as agent-ui; drops the
 * `matched` list for legacy call-sites that only use body/blocks.
 */
export function expandSnippetTokens(
  text: string,
  snippets: readonly Snippet[],
): { body: string; blocks: string[] } {
  const { body, blocks } = expandSnippetTokensShared(text, snippets);
  return { body, blocks };
}
