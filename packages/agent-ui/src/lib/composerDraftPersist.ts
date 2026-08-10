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

/** Timer API injected by hosts so draft persistence remains runtime-neutral. */
export type ComposerDraftTimers = {
  setTimeout: (fn: () => void, ms: number) => number;
  clearTimeout: (id: number) => void;
};

export type ComposerDraftPersistenceOptions = {
  debounceMs: number;
  shouldPersistImmediately: (draft: string) => boolean;
};

export type ComposerDraftPersistence = {
  onChange: (draft: string) => void;
  flush: () => void;
};

/**
 * Debounced draft persistence used by host shells. Clearing a draft can be
 * flushed immediately through the injected policy, preventing stale text from
 * being restored after a reload.
 */
export function createComposerDraftPersistence(
  persist: (draft: string) => void,
  timers: ComposerDraftTimers,
  options: ComposerDraftPersistenceOptions,
): ComposerDraftPersistence {
  let timer: number | null = null;
  let pending: string | null = null;
  const { debounceMs, shouldPersistImmediately } = options;

  function flush(): void {
    if (timer !== null) {
      timers.clearTimeout(timer);
      timer = null;
    }
    if (pending !== null) {
      persist(pending);
      pending = null;
    }
  }

  function onChange(draft: string): void {
    pending = draft;
    if (shouldPersistImmediately(draft)) {
      flush();
      return;
    }
    if (timer !== null) {
      timers.clearTimeout(timer);
    }
    timer = timers.setTimeout(() => {
      timer = null;
      if (pending !== null) {
        persist(pending);
        pending = null;
      }
    }, debounceMs);
  }

  return { onChange, flush };
}
