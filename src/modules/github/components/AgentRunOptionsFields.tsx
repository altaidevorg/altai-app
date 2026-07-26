import { MODELS } from "@/modules/ai/config";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import type { AssignmentRunConfig } from "@/modules/github/lib/assignments";
import { usePreferencesStore } from "@/modules/settings/preferences";
import type { PermissionMode } from "@/modules/settings/store";
import { useEffect, useMemo } from "react";

export type AgentRunOptions = Required<
  Pick<AssignmentRunConfig, "agentId" | "modelId" | "permissionMode">
>;

type Props = {
  value: AgentRunOptions;
  onChange: (value: AgentRunOptions) => void;
  disabled?: boolean;
  className?: string;
};

const SELECT_CLASS =
  "mt-1 h-8 w-full rounded-lg border border-border/60 bg-background/70 px-2 text-[11px] text-foreground outline-none focus:border-sky-500/60 focus:ring-2 focus:ring-sky-500/10 disabled:cursor-not-allowed disabled:opacity-50";

/** Shared runtime selectors for GitHub-backed agent work. */
export function AgentRunOptionsFields({
  value,
  onChange,
  disabled = false,
  className = "",
}: Props) {
  const customAgents = useAgentsStore((s) => s.customAgents);
  const overrides = useAgentsStore((s) => s.overrides);
  const disabledIds = useAgentsStore((s) => s.disabledIds);
  const hydrate = useAgentsStore((s) => s.hydrate);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  const agents = useMemo(
    () => useAgentsStore.getState().enabled(),
    [customAgents, overrides, disabledIds],
  );

  useEffect(() => {
    if (agents.length > 0 && !agents.some((agent) => agent.id === value.agentId)) {
      onChange({ ...value, agentId: agents[0].id });
    }
  }, [agents, onChange, value]);

  return (
    <div
      className={`grid grid-cols-1 gap-2 rounded-xl border border-border/50 bg-background/35 p-2.5 sm:grid-cols-2 ${className}`}
    >
      <label className="min-w-0">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Agent
        </span>
        <select
          value={value.agentId}
          onChange={(event) =>
            onChange({ ...value, agentId: event.target.value })
          }
          disabled={disabled}
          className={SELECT_CLASS}
        >
          {agents.map((agent) => (
            <option key={agent.id} value={agent.id}>
              {agent.name}
            </option>
          ))}
        </select>
      </label>

      <label className="min-w-0">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Model
        </span>
        <select
          value={value.modelId}
          onChange={(event) =>
            onChange({ ...value, modelId: event.target.value })
          }
          disabled={disabled}
          className={SELECT_CLASS}
        >
          {MODELS.map((model) => (
            <option key={model.id} value={model.id}>
              {model.label}
            </option>
          ))}
        </select>
      </label>

      <label className="min-w-0 sm:col-span-2">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Permissions
        </span>
        <select
          value={value.permissionMode}
          onChange={(event) =>
            onChange({
              ...value,
              permissionMode: event.target.value as PermissionMode,
            })
          }
          disabled={disabled}
          className={SELECT_CLASS}
        >
          <option value="ask">Ask before changes</option>
          <option value="auto-edit">Edit workspace automatically</option>
          <option value="plan">Plan only (read-only)</option>
          {bypassEnabled ? <option value="bypass">Bypass approvals</option> : null}
        </select>
      </label>
    </div>
  );
}
