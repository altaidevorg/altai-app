/**
 * Pure configured local/runtime model target match (A6.166).
 * Host builds candidates (prefs + keys); package matches + normalizes URL.
 */

import { toChatCompletionsUrl } from "./isanagentTargetChrome.js";

export type ConfiguredLocalTargetCandidate = {
  catalogId: string;
  providerName: string;
  modelName: string;
  baseUrl: string;
  apiKey: string;
};

export type ResolvedConfiguredLocalTarget = {
  providerName: string;
  apiKey: string;
  modelName: string;
  baseUrl: string;
};

/**
 * Match catalog alias or raw configured model id against host candidates.
 * Returns null when no match or missing modelName/base URL after normalize.
 */
export function resolveConfiguredLocalTargetCandidate(
  modelId: string,
  candidates: readonly ConfiguredLocalTargetCandidate[],
): ResolvedConfiguredLocalTarget | null {
  const target = candidates.find(
    (candidate) =>
      modelId === candidate.catalogId ||
      (!!candidate.modelName && modelId === candidate.modelName),
  );
  if (!target) return null;

  const baseUrl = toChatCompletionsUrl(target.baseUrl);
  if (!target.modelName || !baseUrl) return null;
  return {
    providerName: target.providerName,
    apiKey: target.apiKey,
    modelName: target.modelName,
    baseUrl,
  };
}
