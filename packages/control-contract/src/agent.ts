import type { Revision } from "./revision.js";
import type { TypedId } from "./ids.js";

export type AgentProfileRevision = {
  readonly id: TypedId; readonly profile_id: TypedId; readonly revision: Revision;
  readonly instructions: string; readonly model: string | null;
  readonly capabilities: readonly string[]; readonly created_at: string;
};
export type AgentStatus = "active" | "paused" | "terminated";
export type AgentInstance = {
  readonly id: TypedId; readonly organization_id: TypedId; readonly profile_revision_id: TypedId;
  readonly reports_to_agent_id: TypedId | null; readonly name: string; readonly role: string;
  readonly capabilities: readonly string[]; readonly status: AgentStatus; readonly pause_reason: string | null;
  readonly revision: Revision; readonly created_at: string; readonly updated_at: string;
};
export const canReceiveDispatch = (agent: AgentInstance) => agent.status === "active";
