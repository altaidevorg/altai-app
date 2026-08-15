import type { AgentRecord, AgentStatusInput } from "@altai/host-contract";

/**
 * Agents admin projection (package 064, PR 2). The embedded registry
 * (`agent_list`) becomes the admin surface's rows. The store owns the
 * lifecycle rules — terminated is final, reporting lines are acyclic — so
 * each row carries which actions are legal for its status, decided once
 * here instead of per button.
 */

export type AgentAdminRow = {
  id: string;
  name: string;
  status: AgentStatusInput;
  statusLabel: string;
  reportsToId: string | null;
  reportsToName: string | null;
  canPause: boolean;
  canResume: boolean;
  canTerminate: boolean;
};

export type AgentManagerOption = {
  id: string;
  name: string;
};

const STATUS_LABELS: Record<AgentStatusInput, string> = {
  active: "Active",
  paused: "Paused",
  terminated: "Terminated",
};

export function toAgentAdminRow(
  agent: AgentRecord,
  byId: ReadonlyMap<string, AgentRecord>,
): AgentAdminRow {
  const manager = agent.reportsTo ? byId.get(agent.reportsTo) : undefined;
  return {
    id: agent.id,
    name: agent.name,
    status: agent.status,
    statusLabel: STATUS_LABELS[agent.status],
    reportsToId: agent.reportsTo ?? null,
    reportsToName: manager ? manager.name : null,
    canPause: agent.status === "active",
    canResume: agent.status === "paused",
    canTerminate: agent.status !== "terminated",
  };
}

/** Project the registry rows, name-ordered like the store's listing, with
 * the id map the reporting-line editor resolves names against. */
export function projectAgentsAdmin(agents: readonly AgentRecord[]): {
  rows: AgentAdminRow[];
  byId: ReadonlyMap<string, AgentRecord>;
} {
  const byId = new Map(agents.map((agent) => [agent.id, agent] as const));
  const rows = [...agents]
    .sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id))
    .map((agent) => toAgentAdminRow(agent, byId));
  return { rows, byId };
}

/** Legal managers for a reporting move or the create form: every other
 * agent. The store — not this list — rejects cycles; the surface surfaces
 * that rejection as an inline error. */
export function toManagerOptions(
  agents: readonly AgentRecord[],
  selfId: string | null,
): AgentManagerOption[] {
  return agents
    .filter((agent) => agent.id !== selfId)
    .map((agent) => ({ id: agent.id, name: agent.name }));
}

/** The store's error vocabulary is Work-shaped ("invalid work transition",
 * "work item not found") because the registry shares it; the admin surface
 * should read as agent administration. */
export function toAgentAdminError(raw: string): string {
  const message = raw
    .replace(/^invalid work transition: /, "")
    .replace(/^work item not found: .*$/, "agent not found");
  const sentence = message.charAt(0).toUpperCase() + message.slice(1);
  return /[.!?]$/.test(sentence) ? sentence : `${sentence}.`;
}
