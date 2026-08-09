/**
 * Pure cloud-catalog model target resolve (A6.170).
 * Host supplies catalog rows + provider base URL map + key lookup.
 */

export type CloudModelCatalogEntry = {
  id: string;
  provider: string;
  apiName?: string | null;
};

export type ResolvedProviderTarget = {
  providerName: string;
  apiKey: string;
  modelName: string;
  baseUrl: string;
};

/**
 * Resolve a known cloud catalog model id against keys and provider base URLs.
 * Returns null when id is unknown or a required key is missing.
 */
export function resolveCloudModelTarget(
  modelId: string,
  catalog: readonly CloudModelCatalogEntry[],
  opts: {
    providerBaseUrls: Readonly<Record<string, string>>;
    apiKeys: Readonly<Record<string, string | null | undefined>>;
    providerNeedsKey: (provider: string) => boolean;
  },
): ResolvedProviderTarget | null {
  const model = catalog.find((m) => m.id === modelId);
  if (!model) return null;
  const provider = model.provider;
  const needsKey = opts.providerNeedsKey(provider);
  const key = opts.apiKeys[provider] ?? "";
  if (needsKey && !key) return null;
  return {
    providerName: provider,
    apiKey: key,
    modelName: model.apiName ?? model.id,
    baseUrl: opts.providerBaseUrls[provider] ?? "",
  };
}
