/**
 * Pure Desktop ModelDropdown list filter / section partition (A6.235).
 */

export type DropdownModelLike = {
  id: string;
  label: string;
  hint?: string;
  description?: string;
  provider: string;
};

/** True when free-text search and provider rail both show "all / flat". */
export function modelDropdownShowSections(
  search: string,
  activeProvider: string | null,
): boolean {
  return !search.trim() && activeProvider === null;
}

/**
 * Filter catalog models by provider rail, free-text (label/hint/description/
 * provider/id), and an optional host-supplied agent-capability predicate.
 */
export function filterCatalogModelsForDropdown<T extends DropdownModelLike>(
  models: readonly T[],
  options: {
    search: string;
    provider?: string | null;
    isCompatible?: (model: T) => boolean;
  },
): T[] {
  const q = options.search.trim().toLowerCase();
  let pool = models as readonly T[];
  if (options.provider != null) {
    pool = pool.filter((m) => m.provider === options.provider);
  }
  if (q) {
    pool = pool.filter((m) => {
      return (
        m.label.toLowerCase().includes(q) ||
        (m.hint ?? "").toLowerCase().includes(q) ||
        (m.description ?? "").toLowerCase().includes(q) ||
        m.provider.toLowerCase().includes(q) ||
        m.id.toLowerCase().includes(q)
      );
    });
  }
  if (options.isCompatible) {
    pool = pool.filter(options.isCompatible);
  }
  return [...pool];
}

/**
 * Split a filtered model list into favorite / recent / remaining buckets.
 * When `showSections` is false, all models stay in `remaining`.
 */
export function partitionModelsByFavRecent<T extends { id: string }>(
  filtered: readonly T[],
  favoriteIds: readonly string[],
  recentIds: readonly string[],
  showSections: boolean,
): { pinned: T[]; recent: T[]; remaining: T[] } {
  if (!showSections) {
    return { pinned: [], recent: [], remaining: [...filtered] };
  }
  const byId = new Map(filtered.map((model) => [model.id, model]));
  const favoriteSet = new Set(favoriteIds);
  const recentSet = new Set(recentIds);
  const pinned = favoriteIds
    .map((id) => byId.get(id))
    .filter((model): model is T => model !== undefined);
  const recent = recentIds
    .map((id) => byId.get(id))
    .filter((model): model is T => model !== undefined)
    .filter((model) => !favoriteSet.has(model.id));
  const remaining = filtered.filter(
    (model) => !favoriteSet.has(model.id) && !recentSet.has(model.id),
  );
  return { pinned, recent, remaining };
}
