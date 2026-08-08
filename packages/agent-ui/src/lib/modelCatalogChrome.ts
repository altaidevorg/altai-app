/**
 * Merge host model list with a Studio catalog (A6.89).
 * Hosts supply catalog entries; this module owns merge/sort/filter rules.
 */

import type { ModelInfo } from "@altai/host-contract";
import { AUTO_MODEL_ID } from "./modelPickerChrome.js";

export type CatalogModelEntry = {
  id: string;
  label: string;
  providerId: string;
};

/**
 * Prefer host entries, then fill gaps from the catalog so the picker
 * always offers known models (still selected only via host config/update).
 */
export function mergeModelCatalog(
  hostModels: readonly ModelInfo[],
  catalogModels: readonly CatalogModelEntry[],
  autoModelId: string = AUTO_MODEL_ID,
): ModelInfo[] {
  const byId = new Map<string, ModelInfo>();
  byId.set(autoModelId, {
    id: autoModelId,
    label: "Auto",
    providerId: "auto",
  });
  for (const model of catalogModels) {
    byId.set(model.id, {
      id: model.id,
      label: model.label,
      providerId: model.providerId,
    });
  }
  for (const model of hostModels) {
    const id = model.id.trim();
    if (!id) {
      continue;
    }
    byId.set(id, {
      id,
      label: model.label?.trim() || id,
      providerId: model.providerId?.trim() || "unknown",
    });
  }
  return [...byId.values()].sort((a, b) => {
    if (a.id === autoModelId) {
      return -1;
    }
    if (b.id === autoModelId) {
      return 1;
    }
    const provider = a.providerId.localeCompare(b.providerId);
    if (provider !== 0) {
      return provider;
    }
    return a.label.localeCompare(b.label);
  });
}

/** Models usable now given connected (or keyless) providers. */
export function filterModelsByProviderKeys(
  models: readonly ModelInfo[],
  connectedProviderIds: ReadonlySet<string>,
  autoModelId: string = AUTO_MODEL_ID,
): ModelInfo[] {
  return models.filter((model) => {
    if (model.id === autoModelId) {
      return true;
    }
    if (model.providerId === "auto") {
      return true;
    }
    // Always show everything; the picker/UI can mark locked separately.
    // When no providers connected at all, still show full catalog.
    void connectedProviderIds;
    return true;
  });
}

export function providerIdForModel(
  modelId: string,
  models: readonly ModelInfo[],
  autoModelId: string = AUTO_MODEL_ID,
): string | undefined {
  if (!modelId || modelId === autoModelId) {
    return undefined;
  }
  return models.find((model) => model.id === modelId)?.providerId;
}
