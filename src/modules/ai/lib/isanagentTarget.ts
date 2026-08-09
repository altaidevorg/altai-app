import {
  describeUnresolvedIsanAgentTarget,
  fallbackSpecFromTarget,
  isConfiguredLocalCatalogId,
  resolveCloudModelTarget,
  resolveCompactionSpecFromContext,
  resolveConfiguredLocalTargetCandidate,
} from "@altai/agent-ui";
/**
 * Resolve the UI's selected model into the concrete (provider, apiKey,
 * modelName, baseUrl) tuple the IsanAgent runtime needs.
 *
 * The frontend model picker operates on a flat `ModelId`, but IsanAgent speaks
 * provider + model + key + base URL. This module is the single bridge: it maps a
 * `ModelId` to its provider, looks up the key, and computes the provider-specific
 * chat-completions endpoint (overridable for local/runtime providers).
 */
import {
  getModel,
  getModelContextLimit,
  KEYLESS_PROVIDERS,
  MODELS,
  providerNeedsKey,
  type ModelId,
  type ProviderId,
} from "../config";
import type { ProviderKeys } from "./keyring";

/** Inputs shared by both resolution paths (primary + fallback). */
export type TargetInputs = {
  lmstudioBaseURL: string;
  lmstudioModelId: string;
  mlxBaseURL: string;
  mlxModelId: string;
  openaiCompatibleBaseURL: string;
  openaiCompatibleModelId: string;
};

/** A fully-resolved provider target for the runtime. */
export type ResolvedTarget = {
  providerName: string;
  apiKey: string;
  modelName: string;
  baseUrl: string;
};

/** Result of resolving the primary target. */
export type TargetResolution =
  | { ok: true; target: ResolvedTarget }
  | { ok: false; error: string };

/**
 * Mirrors `FallbackProviderSpec` on the Rust side (camelCase over the wire).
 * Returned by `resolveFallbackSpec` for the failover model.
 */
export type FallbackSpec = {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  modelName: string;
};

/** Standard OpenAI-compatible chat-completions endpoints per cloud provider. */
const PROVIDER_BASE_URLS: Record<ProviderId, string> = {
  openai: "https://api.openai.com/v1/chat/completions",
  anthropic: "https://api.anthropic.com/v1/messages",
  // Gemini OpenAI-compat endpoint (isanagent POSTs to this URL as-is).
  google:
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
  xai: "https://api.x.ai/v1/chat/completions",
  cerebras: "https://api.cerebras.ai/v1/chat/completions",
  groq: "https://api.groq.com/openai/v1/chat/completions",
  deepseek: "https://api.deepseek.com/v1/chat/completions",
  mistral: "https://api.mistral.ai/v1/chat/completions",
  zai: "https://api.z.ai/api/paas/v4/chat/completions",
  "zai-coding-plan": "https://api.z.ai/api/coding/paas/v4/chat/completions",
  openrouter: "https://openrouter.ai/api/v1/chat/completions",
  // Local/runtime providers: their base URL comes from user prefs (below).
  "openai-compatible": "",
  lmstudio: "",
  mlx: "",
};

/** Resolve catalog aliases and raw configured local-model ids through one table. */
function resolveConfiguredLocalTarget(
  modelId: string,
  apiKeys: ProviderKeys,
  inputs: TargetInputs,
): ResolvedTarget | null {
  return resolveConfiguredLocalTargetCandidate(modelId, [
    {
      catalogId: "lmstudio-local",
      providerName: "lmstudio",
      modelName: inputs.lmstudioModelId.trim(),
      baseUrl: inputs.lmstudioBaseURL,
      apiKey: "",
    },
    {
      catalogId: "mlx-local",
      providerName: "mlx",
      modelName: inputs.mlxModelId.trim(),
      baseUrl: inputs.mlxBaseURL,
      apiKey: "",
    },
    {
      catalogId: "openai-compatible-custom",
      providerName: "openai-compatible",
      modelName: inputs.openaiCompatibleModelId.trim(),
      baseUrl: inputs.openaiCompatibleBaseURL,
      apiKey: apiKeys["openai-compatible"] ?? "",
    },
  ]);
}

/**
 * Resolve a single (provider, modelId) pair against keys + local-pref base URLs.
 * Returns `null` when the model id is unknown or a required key/pref is missing.
 *
 * Catalog ids for local/runtime providers (`lmstudio-local`, `mlx-local`,
 * `openai-compatible-custom`) are remapped to the user-configured model id and
 * base URL from Settings — matching `buildConfiguredLanguageModel`.
 */
function resolveOne(
  modelId: string,
  apiKeys: ProviderKeys,
  inputs: TargetInputs,
): ResolvedTarget | null {
  // Local/runtime catalog aliases must resolve before the generic catalog path;
  // otherwise the runtime would receive an empty base URL. Raw configured ids
  // use the same table, so fallback resolution cannot drift from the primary.
  const localTarget = resolveConfiguredLocalTarget(modelId, apiKeys, inputs);
  if (localTarget) return localTarget;
  if (isConfiguredLocalCatalogId(modelId)) return null;

  // Cloud providers: model id must be a known MODELS entry.
  return resolveCloudModelTarget(modelId, MODELS, {
    providerBaseUrls: PROVIDER_BASE_URLS,
    apiKeys,
    providerNeedsKey: (provider) => providerNeedsKey(provider as ProviderId),
  });
}

/**
 * Resolve the primary IsanAgent target for a chat send.
 *
 * Returns `{ ok: false, error }` (surfaced as an error banner in the UI) when
 * the selected model can't be resolved — either unknown id or a missing key for
 * a key-requiring provider. Keyless providers (lmstudio/mlx) never fail on key.
 */
export function resolveIsanAgentTarget(
  selectedModelId: string,
  apiKeys: ProviderKeys,
  inputs: TargetInputs,
): TargetResolution {
  const target = resolveOne(selectedModelId, apiKeys, inputs);
  if (!target) {
    // Distinguish "unknown model" from "missing key" for a clearer message.
    const known = MODELS.find((m) => m.id === selectedModelId) as
      | (typeof MODELS)[number]
      | undefined;
    const knownKeyProvider =
      known && providerNeedsKey(known.provider) ? known.provider : null;
    return {
      ok: false,
      error: describeUnresolvedIsanAgentTarget(
        selectedModelId,
        knownKeyProvider,
      ),
    };
  }
  return { ok: true, target };
}

/**
 * Resolve the failover target for the selected fallback model. Returns `null`
 * when no fallback is configured (empty id) or it can't be resolved — the
 * runtime treats `null` as "failover off".
 */
export function resolveFallbackSpec(
  fallbackModelId: string,
  apiKeys: ProviderKeys,
  inputs: TargetInputs,
): FallbackSpec | null {
  return fallbackSpecFromTarget(
    fallbackModelId,
    resolveOne(fallbackModelId, apiKeys, inputs),
  );
}

// Re-export for callers that want the catalog without a second import.
export { getModel, KEYLESS_PROVIDERS };
export type { ModelId };

/**
 * Compaction config spec — the camelCase shape the Tauri IPC expects
 * (mirrors `CompactionArg` in `runtime.rs`).
 */
export type CompactionSpec = {
  auto: boolean;
  thresholdTokens: number;
  tailTurns: number;
};

/**
 * Resolve the user-facing compaction prefs into the `(auto, thresholdTokens,
 * tailTurns)` tuple the runtime consumes. When a percent threshold is set,
 * it's converted to tokens against the active model's context window
 * (taking precedence over the absolute token setting). Otherwise the
 * absolute token setting is used as-is.
 *
 * The runtime also floors the threshold at 8k and forces MAX when auto is
 * off — no need to replicate that here (the Rust side is the source of
 * truth since it bakes the values into `AgentLogic`).
 */
export function resolveCompactionSpec(
  prefs: {
    compactionAuto: boolean;
    compactionThresholdPercent: number | null;
    compactionThresholdTokens: number;
    compactionTailTurns: number;
  },
  modelId: string,
  compatOverride?: number,
): CompactionSpec {
  return resolveCompactionSpecFromContext(
    prefs,
    getModelContextLimit(modelId, compatOverride),
  );
}
