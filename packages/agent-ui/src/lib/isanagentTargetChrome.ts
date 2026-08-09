/**
 * Pure IsanAgent target URL / catalog helpers (A6.160).
 * Host owns model catalog, keys, and provider base URL maps.
 */

/** Catalog ids for user-configured local/runtime providers. */
export const CONFIGURED_LOCAL_CATALOG_IDS = [
  "lmstudio-local",
  "mlx-local",
  "openai-compatible-custom",
] as const;

export type ConfiguredLocalCatalogId =
  (typeof CONFIGURED_LOCAL_CATALOG_IDS)[number];

export function isConfiguredLocalCatalogId(
  modelId: string,
): modelId is ConfiguredLocalCatalogId {
  return (CONFIGURED_LOCAL_CATALOG_IDS as readonly string[]).includes(modelId);
}

/**
 * Settings store AI-SDK style roots (`…/v1`). Isanagent POSTs to the URL as-is,
 * so append `/chat/completions` when the user hasn't already provided a full path.
 */
export function toChatCompletionsUrl(baseUrl: string): string {
  const trimmed = baseUrl.trim().replace(/\/+$/, "");
  if (!trimmed) return "";
  if (
    trimmed.endsWith("/chat/completions") ||
    trimmed.endsWith("/messages") ||
    trimmed.includes("/chat/completions")
  ) {
    return trimmed;
  }
  return `${trimmed}/chat/completions`;
}

/**
 * User-facing error when primary model target cannot be resolved.
 * Host passes whether the id is a known key-requiring provider.
 */
export function describeUnresolvedIsanAgentTarget(
  selectedModelId: string,
  knownKeyProvider: string | null | undefined,
): string {
  if (selectedModelId === "openai-compatible-custom") {
    return "OpenAI-compatible endpoint is not configured. Set Base URL and Model ID in Settings → Models.";
  }
  if (selectedModelId === "lmstudio-local") {
    return "LM Studio is not configured. Set Base URL and Model ID in Settings → Models.";
  }
  if (selectedModelId === "mlx-local") {
    return "MLX is not configured. Set Base URL and Model ID in Settings → Models.";
  }
  if (knownKeyProvider) {
    return `No API key set for ${knownKeyProvider}. Add it in Settings.`;
  }
  return `Unknown model: ${selectedModelId}`;
}
