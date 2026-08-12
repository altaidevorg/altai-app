import {
  CheckListIcon,
  Home01Icon,
  Robot01Icon,
  Settings01Icon,
  SourceCodeIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";

export type DesktopDestination = "home" | "work" | "agents";

type Props = {
  activeDestination: DesktopDestination;
  inboxCount?: number;
  onNavigate: (destination: DesktopDestination) => void;
  onOpenIde: () => void;
  onOpenSettings: () => void;
};

const DESTINATIONS = [
  { id: "home", label: "Home", icon: Home01Icon },
  { id: "work", label: "Work", icon: CheckListIcon },
  { id: "agents", label: "Agents", icon: Robot01Icon },
] as const;

/**
 * M5-A primary Desktop navigation. Destinations use navigation semantics;
 * tabs remain reserved for switching content inside a destination.
 */
export function DesktopPrimaryNav({
  activeDestination,
  inboxCount = 0,
  onNavigate,
  onOpenIde,
  onOpenSettings,
}: Props) {
  return (
    <nav
      aria-label="Primary"
      className={cn(
        "flex h-10 shrink-0 items-center gap-0.5 border-b border-border-subtle bg-raised px-2",
        IS_MAC && "pl-20",
      )}
    >
      {DESTINATIONS.map((item) => {
        const isActive = item.id === activeDestination;
        const badge = item.id === "home" ? inboxCount : 0;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={isActive ? "page" : undefined}
            onClick={() => onNavigate(item.id)}
            className={cn(
              "relative inline-flex min-h-7 items-center gap-1.5 rounded-md px-2.5 text-[11px] font-medium outline-none transition-colors",
              "focus-visible:ring-2 focus-visible:ring-ring/50",
              isActive
                ? "bg-accent text-foreground"
                : "text-muted-foreground hover:bg-accent/70 hover:text-foreground",
            )}
          >
            <HugeiconsIcon icon={item.icon} size={14} strokeWidth={1.8} />
            <span>{item.label}</span>
            {badge > 0 ? (
              <span
                aria-label={`${badge} items need attention`}
                className="rounded-full bg-info/15 px-1 text-[9px] font-semibold tabular-nums text-info dark:bg-info/25"
              >
                {badge > 9 ? "9+" : badge}
              </span>
            ) : null}
          </button>
        );
      })}
      <span data-tauri-drag-region className="h-full flex-1" />
      <button
        type="button"
        onClick={onOpenIde}
        className="inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-[11px] font-medium text-muted-foreground outline-none transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
      >
        <HugeiconsIcon icon={SourceCodeIcon} size={14} strokeWidth={1.8} />
        <span>IDE</span>
      </button>
      <button
        type="button"
        aria-label="Settings"
        title="Settings"
        onClick={onOpenSettings}
        className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
      >
        <HugeiconsIcon icon={Settings01Icon} size={14} strokeWidth={1.8} />
      </button>
    </nav>
  );
}
