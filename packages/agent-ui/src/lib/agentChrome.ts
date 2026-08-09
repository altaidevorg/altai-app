/**
 * Pure agent catalog helpers: id, find, override/diff (A6.152).
 * Hosts supply agent records; no storage I/O.
 */

export function newAgentId(
  now: () => number = Date.now,
  random: () => number = Math.random,
): string {
  return `a-${now().toString(36)}-${random().toString(36).slice(2, 6)}`;
}

export function findAgentById<T extends { id: string }>(
  agents: readonly T[],
  id: string | null | undefined,
  fallback: T,
): T {
  if (!id) return fallback;
  return agents.find((a) => a.id === id) ?? fallback;
}

export function applyAgentOverride<T extends object>(
  base: T,
  override: Partial<T> | undefined,
): T {
  return override ? { ...base, ...override } : base;
}

export type AgentEditableFields = {
  name: string;
  description: string;
  instructions: string;
  icon: string;
};

/**
 * Build the override patch — only fields that differ from the default.
 */
export function diffAgentAgainstBase(
  base: AgentEditableFields,
  edited: AgentEditableFields,
): Partial<AgentEditableFields> {
  const patch: Partial<AgentEditableFields> = {};
  if (edited.name !== base.name) patch.name = edited.name;
  if (edited.description !== base.description) {
    patch.description = edited.description;
  }
  if (edited.instructions !== base.instructions) {
    patch.instructions = edited.instructions;
  }
  if (edited.icon !== base.icon) patch.icon = edited.icon;
  return patch;
}
