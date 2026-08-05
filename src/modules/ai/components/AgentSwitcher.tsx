import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import {
  AbsoluteIcon,
  AtomicPowerIcon,
  BookSearchIcon,
  CodeIcon,
  DatabaseIcon,
  Notebook01Icon,
  PaintBrush04Icon,
  PencilEdit02Icon,
  Settings01Icon,
  ShieldUserIcon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { ISANAGENT_AGENT_IDS, type AgentIconId } from "../lib/agents";
import { useAgentsStore } from "../store/agentsStore";
import { AgentOptionRow, AgentSwitcherTrigger } from "@altai/agent-ui";

const ICONS: Record<AgentIconId, typeof CodeIcon> = {
  coder: CodeIcon,
  architect: AbsoluteIcon,
  reviewer: PencilEdit02Icon,
  security: ShieldUserIcon,
  designer: PaintBrush04Icon,
  paper: BookSearchIcon,
  notebook: Notebook01Icon,
  dataset: DatabaseIcon,
  spark: SparklesIcon,
};

type AgentSwitcherVariant = "default" | "mini" | "toolbar" | "toolbar-icon";

export function AgentSwitcher({
  isMiniWindow,
  variant,
}: {
  isMiniWindow?: boolean;
  variant?: AgentSwitcherVariant;
}) {
  // Subscribe to the underlying state so any change (custom agents,
  // disabled set, builtin overrides, active id) re-renders the picker.
  const customAgents = useAgentsStore((s) => s.customAgents);
  const disabledIds = useAgentsStore((s) => s.disabledIds);
  const overrides = useAgentsStore((s) => s.overrides);
  const activeId = useAgentsStore((s) => s.activeId);
  const setActiveId = useAgentsStore((s) => s.setActiveId);

  // Keep these subscriptions live — selectors above are what trigger re-renders.
  void customAgents;
  void disabledIds;
  void overrides;

  const list = useAgentsStore.getState().enabled();
  const allList = useAgentsStore.getState().all();
  // Resolve active from the full list (including disabled) so the trigger
  // still labels the disabled-but-active edge correctly until the store
  // downgrades it on the next setDisabled call.
  const active = allList.find((a) => a.id === activeId) ?? list[0] ?? allList[0];
  const builtIn = list.filter(
    (a) => a.builtIn && !ISANAGENT_AGENT_IDS.has(a.id),
  );
  const mlAgents = list.filter(
    (a) => a.builtIn && ISANAGENT_AGENT_IDS.has(a.id),
  );
  const custom = list.filter((a) => !a.builtIn);
  const ActiveIcon = ICONS[active.icon] ?? SparklesIcon;
  const activeIsMl = ISANAGENT_AGENT_IDS.has(active.id);

  const resolved: AgentSwitcherVariant =
    variant ?? (isMiniWindow ? "mini" : "default");
  const isToolbar = resolved === "toolbar" || resolved === "toolbar-icon";
  const dropdownSide = isToolbar ? "top" : undefined;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <AgentSwitcherTrigger
          name={active.name}
          icon={ActiveIcon}
          variant={resolved}
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side={dropdownSide}
        sideOffset={isToolbar ? 6 : undefined}
        collisionPadding={isToolbar ? 8 : undefined}
        align="start"
        className={cn(
          "min-w-60 bg-popover text-popover-foreground",
          isToolbar && "w-[min(22rem,calc(100vw-1rem))]",
        )}
      >
        <div className="px-2 pt-1.5 pb-1 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
          Built-in
        </div>
        {builtIn.map((a) => {
          const Icon = ICONS[a.icon] ?? SparklesIcon;
          return (
            <DropdownMenuItem
              key={a.id}
              onSelect={() => setActiveId(a.id)}
              className={cn(
                "flex items-start gap-2 pr-2 text-[12px]",
                a.id === activeId && "bg-foreground/[0.085]",
              )}
            >
              <AgentOptionRow
                name={a.name}
                description={a.description}
                icon={Icon}
                selected={a.id === activeId}
              />
            </DropdownMenuItem>
          );
        })}
        {mlAgents.length > 0 ? (
          <DropdownMenuSub>
            <DropdownMenuSubTrigger
              className={cn(
                "flex items-center gap-2 px-2 py-1.5 text-[12px] font-normal",
                activeIsMl && "bg-foreground/[0.085]",
              )}
            >
              <HugeiconsIcon
                icon={AtomicPowerIcon}
                size={13}
                strokeWidth={1.75}
                className={cn(
                  "shrink-0",
                  activeIsMl ? "text-foreground" : "text-muted-foreground",
                )}
              />
              <span className="flex-1">ML Agents</span>
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent
              sideOffset={4}
              collisionPadding={8}
              className="min-w-60 bg-popover text-popover-foreground"
            >
              {mlAgents.map((a) => {
                const Icon = ICONS[a.icon] ?? SparklesIcon;
                return (
                  <DropdownMenuItem
                    key={a.id}
                    onSelect={() => setActiveId(a.id)}
                    className={cn(
                      "flex items-start gap-2 pr-2 text-[12px]",
                      a.id === activeId && "bg-foreground/[0.085]",
                    )}
                  >
                    <AgentOptionRow
                      name={a.name}
                      description={a.description}
                      icon={Icon}
                      selected={a.id === activeId}
                    />
                  </DropdownMenuItem>
                );
              })}
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        ) : null}
        {custom.length > 0 ? (
          <>
            <DropdownMenuSeparator />
            <div className="px-2 pt-1 pb-1 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
              Custom
            </div>
            {custom.map((a) => {
              const Icon = ICONS[a.icon] ?? SparklesIcon;
              return (
                <DropdownMenuItem
                  key={a.id}
                  onSelect={() => setActiveId(a.id)}
                  className={cn(
                    "flex items-start gap-2 text-[12px]",
                    a.id === activeId && "bg-foreground/[0.085]",
                  )}
                >
                  <AgentOptionRow
                    name={a.name}
                    description={a.description}
                    icon={Icon}
                    selected={a.id === activeId}
                    iconAlwaysMuted
                  />
                </DropdownMenuItem>
              );
            })}
          </>
        ) : null}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={() => void openSettingsWindow("agents")}
          className="gap-2 text-[12px] text-muted-foreground"
        >
          <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.75} />
          Manage agents…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export { ICONS as AGENT_ICONS };
