/**
 * Pure provider API-key presence checks (A6.220).
 */

export type ProviderKeyMap = Readonly<
  Record<string, string | null | undefined>
>;

/**
 * Whether the given provider is ready: no key required, or a non-empty key
 * is present in `apiKeys`.
 */
export function hasProviderApiKey(input: {
  provider: string;
  apiKeys: ProviderKeyMap;
  providerNeedsKey: (provider: string) => boolean;
}): boolean {
  if (!input.providerNeedsKey(input.provider)) return true;
  return Boolean(input.apiKeys[input.provider]);
}

/**
 * Model-level readiness when the host can resolve `modelId → provider`.
 */
export function hasApiKeyForModel(input: {
  modelId: string;
  apiKeys: ProviderKeyMap;
  providerForModel: (modelId: string) => string;
  providerNeedsKey: (provider: string) => boolean;
}): boolean {
  return hasProviderApiKey({
    provider: input.providerForModel(input.modelId),
    apiKeys: input.apiKeys,
    providerNeedsKey: input.providerNeedsKey,
  });
}
