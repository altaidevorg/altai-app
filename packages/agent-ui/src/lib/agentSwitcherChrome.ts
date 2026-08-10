/**
 * Pure AgentSwitcher list partition (A6.244).
 */

/**
 * Split enabled agents into product built-ins, IsanAgent ML built-ins, and custom.
 * Host supplies `isIsanagentAgent` so package avoids product id coupling.
 */
export function partitionAgentsForSwitcher<
  T extends { id: string; builtIn?: boolean },
>(
  agents: readonly T[],
  isIsanagentAgent: (id: string) => boolean,
): { builtIn: T[]; mlAgents: T[]; custom: T[] } {
  const builtIn: T[] = [];
  const mlAgents: T[] = [];
  const custom: T[] = [];
  for (const agent of agents) {
    if (!agent.builtIn) {
      custom.push(agent);
      continue;
    }
    if (isIsanagentAgent(agent.id)) {
      mlAgents.push(agent);
    } else {
      builtIn.push(agent);
    }
  }
  return { builtIn, mlAgents, custom };
}

/**
 * Resolve which agent labels the switcher trigger.
 * Prefers the full catalog so a disabled-but-active id still renders.
 */
export function resolveSwitcherActiveAgent<T extends { id: string }>(
  allAgents: readonly T[],
  enabledAgents: readonly T[],
  activeId: string | null | undefined,
): T | undefined {
  if (activeId) {
    const active = allAgents.find((agent) => agent.id === activeId);
    if (active) return active;
  }
  return enabledAgents[0] ?? allAgents[0];
}
