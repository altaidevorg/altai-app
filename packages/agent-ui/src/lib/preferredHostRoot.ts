/**
 * Pure helpers for preferred multi-root workspace URI retention (A6.115).
 */

export const PREFERRED_HOST_ROOT_STATE_KEY = "altai.preferredHostRootUri";

/**
 * Keep preferred URI only when it still matches an open workspace folder string.
 */
export function retainPreferredHostRootUri(
  preferredUri: string | undefined,
  openRootUris: readonly string[],
): string | undefined {
  if (!preferredUri) {
    return undefined;
  }
  const normalized = preferredUri.trim();
  if (!normalized) {
    return undefined;
  }
  return openRootUris.includes(normalized) ? normalized : undefined;
}

export function readPreferredHostRootFromState(
  state: { get: (key: string) => unknown },
  openRootUris: readonly string[],
): string | undefined {
  const raw = state.get(PREFERRED_HOST_ROOT_STATE_KEY);
  if (typeof raw !== "string") {
    return undefined;
  }
  return retainPreferredHostRootUri(raw, openRootUris);
}
