/**
 * Pure provider-key presence helpers (A6.161).
 * Host owns the provider catalog; package only checks key map shape.
 */

export type ProviderKeySupport = {
  id: string;
  supportsKey: boolean;
};

/** True when any key-using provider has a non-empty key string in `keys`. */
export function hasAnyProviderKey(
  keys: Readonly<Record<string, string | null | undefined>>,
  providers: readonly ProviderKeySupport[],
): boolean {
  return providers.some((p) => p.supportsKey && !!keys[p.id]);
}
