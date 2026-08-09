/**
 * Pure failover target shape mapper (A6.168).
 * Host resolves full targets; package maps to Rust FallbackProviderSpec fields.
 */

export type ResolvedProviderTarget = {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  modelName: string;
};

export type FallbackProviderSpec = {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  modelName: string;
};

/** Map a resolved primary-style target to the failover IPC shape (or null for empty id). */
export function fallbackSpecFromTarget(
  fallbackModelId: string,
  target: ResolvedProviderTarget | null,
): FallbackProviderSpec | null {
  if (!fallbackModelId || !target) return null;
  return {
    providerName: target.providerName,
    baseUrl: target.baseUrl,
    apiKey: target.apiKey,
    modelName: target.modelName,
  };
}
