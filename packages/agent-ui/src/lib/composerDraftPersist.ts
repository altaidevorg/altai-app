/**
 * Debounce timing for presentation-only composer draft persistence (A6.125).
 * Pure constants / helpers for tests (no timers here).
 */

/** Delay between draft keystrokes and getState/setState write. */
export const COMPOSER_DRAFT_DEBOUNCE_MS = 200;

/**
 * Whether an immediate (non-debounced) flush is warranted for empty drafts
 * so reload does not resurrect deleted text after a partial debounce window.
 */
export function shouldPersistComposerDraftImmediately(draft: string): boolean {
  return draft.length === 0;
}
