/**
 * Pure agent lookup by id or display name (A6.174).
 * Case-insensitive; host owns agent catalogs and activation.
 */

export type NamedAgent = {
  id: string;
  name: string;
};

/** Find an agent whose id or name equals `query` (trim + lower). */
export function findAgentByIdOrName<T extends NamedAgent>(
  agents: readonly T[],
  query: string,
): T | undefined {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return undefined;
  return agents.find(
    (item) =>
      item.id.toLowerCase() === normalized ||
      item.name.toLowerCase() === normalized,
  );
}
