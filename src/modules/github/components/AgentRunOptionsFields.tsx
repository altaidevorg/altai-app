import { type ModelId } from "@/modules/ai/config";
import { ModelDropdown } from "@/modules/ai/components/ModelDropdown";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import type { AssignmentRunConfig } from "@/modules/github/lib/assignments";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  PERMISSION_MODE_LABELS,
  type PermissionMode,
} from "@/modules/settings/store";
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

const PERMISSION_OPTIONS: PermissionMode[] = [
  "ask",
  "auto-edit",
  "plan",
  "bypass",
];

const SELECT_TRIGGER_CLASS =
  "mt-1 h-8 w-full min-w-0 rounded-md border-border/80 bg-popover px-2.5 text-[11px] hover:bg-foreground/[0.045]";

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
      className={cn(
        "grid grid-cols-1 gap-2 rounded-lg border border-border bg-muted/20 p-2.5 sm:grid-cols-2",
        className,
      )}
    >
      <div className="min-w-0">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Agent
        </span>
        <Select
          value={value.agentId}
          onValueChange={(agentId) => onChange({ ...value, agentId })}
          disabled={disabled}
        >
          <SelectTrigger
            size="sm"
            aria-label="Agent"
            className={SELECT_TRIGGER_CLASS}
          >
            <SelectValue placeholder="Choose an agent" />
          </SelectTrigger>
          <SelectContent position="popper" align="start" sideOffset={4}>
            {agents.map((agent) => (
              <SelectItem key={agent.id} value={agent.id}>
                {agent.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="min-w-0">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Model
        </span>
        <div className="mt-1">
          <ModelDropdown
            value={value.modelId}
            onChange={(modelId: ModelId) => onChange({ ...value, modelId })}
            className="h-8 w-full max-w-none justify-between rounded-md border border-border/80 bg-popover px-2.5 text-[11px] text-popover-foreground hover:bg-foreground/[0.045]"
          />
        </div>
      </div>

      <div className="min-w-0 sm:col-span-2">
        <span className="block text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Permissions
        </span>
        <Select
          value={value.permissionMode}
          onValueChange={(permissionMode: PermissionMode) =>
            onChange({ ...value, permissionMode })
          }
          disabled={disabled}
        >
          <SelectTrigger
            size="sm"
            aria-label="Permissions"
            className={SELECT_TRIGGER_CLASS}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent position="popper" align="start" sideOffset={4}>
            {PERMISSION_OPTIONS.filter(
              (mode) => mode !== "bypass" || bypassEnabled,
            ).map((mode) => (
              <SelectItem key={mode} value={mode}>
                {PERMISSION_MODE_LABELS[mode]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
