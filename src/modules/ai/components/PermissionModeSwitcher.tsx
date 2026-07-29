import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  effectivePermissionMode,
  PERMISSION_MODE_DESCRIPTIONS,
  PERMISSION_MODE_LABELS,
  setBypassPermissionsEnabled,
  setPermissionMode,
  type PermissionMode,
} from "@/modules/settings/store";
import {
  ArrowDown01Icon,
  CheckmarkCircle02Icon,
  Edit02Icon,
  Route01Icon,
  Settings01Icon,
  ShieldEnergyIcon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

const ICONS: Record<PermissionMode, typeof CheckmarkCircle02Icon> = {
  ask: CheckmarkCircle02Icon,
  "auto-edit": Edit02Icon,
  plan: Route01Icon,
  bypass: ShieldEnergyIcon,
};

const MODE_COLORS: Record<
  PermissionMode,
  { trigger: string; icon: string; label: string }
> = {
  ask: {
    trigger: "text-success hover:text-success",
    icon: "text-success",
    label: "text-success",
  },
  "auto-edit": {
    trigger: "text-info hover:text-info",
    icon: "text-info",
    label: "text-info",
  },
  plan: {
    trigger: "text-warning hover:text-warning",
    icon: "text-warning",
    label: "text-warning",
  },
  bypass: {
    trigger: "text-destructive hover:text-destructive",
    icon: "text-destructive",
    label: "text-destructive",
  },
};

const VISIBLE_MODES: readonly PermissionMode[] = [
  "ask",
  "auto-edit",
  "plan",
  "bypass",
];

type Variant = "toolbar" | "toolbar-icon";

export function PermissionModeSwitcher({
  variant = "toolbar",
}: {
  variant?: Variant;
}) {
  const mode = usePreferencesStore((s) => s.permissionMode);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);

  // If bypass got disabled in Settings while it was the active mode, the
  // setter already downgrades to "ask" — but defensively reflect that here
  // for the trigger label (shared with the send-flow guard).
  const effectiveMode: PermissionMode = effectivePermissionMode(mode, bypassEnabled);
  const ActiveIcon = ICONS[effectiveMode];
  const activeColors = MODE_COLORS[effectiveMode];
  const isIconOnly = variant === "toolbar-icon";

  const selectMode = (next: PermissionMode) => {
    void (async () => {
      // Selecting bypass from the toolbar also unlocks the settings gate so
      // the mode is actually effective (not silently downgraded to ask).
      if (next === "bypass" && !bypassEnabled) {
        await setBypassPermissionsEnabled(true);
      }
      await setPermissionMode(next);
    })();
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          size="xs"
          variant="ghost"
          className={cn(
            "group flex h-7 min-w-0 max-w-[10rem] items-center gap-1.5 rounded-md px-2 text-[11.5px] transition-colors hover:bg-foreground/[0.055]",
            activeColors.trigger,
          )}
          aria-label={`Permission mode: ${PERMISSION_MODE_LABELS[effectiveMode]}`}
          title={`Permission mode: ${PERMISSION_MODE_LABELS[effectiveMode]}`}
        >
          <HugeiconsIcon
            icon={ActiveIcon}
            size={13}
            strokeWidth={1.75}
            className="shrink-0"
            aria-hidden="true"
          />
          {!isIconOnly && (
            <>
              <span className="truncate font-medium">
                {PERMISSION_MODE_LABELS[effectiveMode]}
              </span>
              <HugeiconsIcon
                icon={ArrowDown01Icon}
                size={11}
                strokeWidth={2}
                className="shrink-0 opacity-60 transition-opacity group-hover:opacity-90"
              />
            </>
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        sideOffset={6}
        collisionPadding={8}
        align="start"
        className="w-[min(22rem,calc(100vw-1rem))] bg-popover text-popover-foreground"
      >
        <div className="px-2 pt-1.5 pb-1 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
          Permissions
        </div>
        {VISIBLE_MODES.map((m) => {
          const Icon = ICONS[m];
          const isActive = m === effectiveMode;
          const danger = m === "bypass";
          const colors = MODE_COLORS[m];
          return (
            <DropdownMenuItem
              key={m}
              onSelect={() => selectMode(m)}
              className={cn(
                "flex items-start gap-2 pr-2 text-[12px]",
                isActive && "bg-foreground/[0.085]",
                danger && "focus:bg-destructive/10",
              )}
            >
              <HugeiconsIcon
                icon={Icon}
                size={13}
                strokeWidth={1.75}
                className={cn("mt-0.5 shrink-0", colors.icon)}
              />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className={colors.label}>
                  {PERMISSION_MODE_LABELS[m]}
                </span>
                <span className="line-clamp-2 text-[10.5px] text-muted-foreground">
                  {PERMISSION_MODE_DESCRIPTIONS[m]}
                </span>
              </span>
              {isActive ? (
                <HugeiconsIcon
                  icon={Tick02Icon}
                  size={12}
                  strokeWidth={2}
                  className={cn("mt-0.5 shrink-0", colors.icon)}
                />
              ) : null}
            </DropdownMenuItem>
          );
        })}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={() => void openSettingsWindow("general")}
          className="gap-2 text-[12px] text-muted-foreground"
        >
          <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.75} />
          Manage permissions…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
