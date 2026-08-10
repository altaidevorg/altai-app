/**
 * Pure catalog id lookup helpers (A6.227).
 */

/** Find a catalog entry by id (agents, models, …). */
export function findCatalogEntryById<T extends { id: string }>(
  catalog: readonly T[],
  id: string | null | undefined,
): T | undefined {
  if (id == null || id === "") return undefined;
  return catalog.find((entry) => entry.id === id);
}

/**
 * Display label for a model id, falling back to the raw id when unknown.
 * Returns undefined when no model id is set.
 */
export function catalogModelLabel(
  models: readonly { id: string; label?: string | null }[],
  modelId: string | null | undefined,
): string | undefined {
  if (!modelId) return undefined;
  const model = findCatalogEntryById(models, modelId);
  return model?.label ?? modelId;
}
