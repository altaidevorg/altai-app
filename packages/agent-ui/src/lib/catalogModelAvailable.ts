/**
 * Pure catalog model availability (A6.222).
 */

/** True when model is visible and its provider is key-ready. */
export function isCatalogModelAvailable(input: {
  modelId: string;
  provider: string;
  hiddenIds: ReadonlySet<string> | readonly string[];
  providerNeedsKey: (provider: string) => boolean;
  hasProviderKey: (provider: string) => boolean;
}): boolean {
  const hidden =
    input.hiddenIds instanceof Set
      ? input.hiddenIds
      : new Set(input.hiddenIds);
  if (hidden.has(input.modelId)) return false;
  if (
    input.providerNeedsKey(input.provider) &&
    !input.hasProviderKey(input.provider)
  ) {
    return false;
  }
  return true;
}

/** Filter a catalog list with host-provided key/hidden predicates. */
export function filterAvailableCatalogModels<
  T extends { id: string; provider: string },
>(
  models: readonly T[],
  apiKeys: Readonly<Record<string, string | null | undefined>>,
  hiddenIds: ReadonlySet<string> | readonly string[],
  providerNeedsKey: (provider: string) => boolean,
): T[] {
  const hidden =
    hiddenIds instanceof Set ? hiddenIds : new Set(hiddenIds);
  return models.filter((model) =>
    isCatalogModelAvailable({
      modelId: model.id,
      provider: model.provider,
      hiddenIds: hidden,
      providerNeedsKey,
      hasProviderKey: (provider) =>
        providerNeedsKey(provider) ? Boolean(apiKeys[provider]) : true,
    }),
  );
}
