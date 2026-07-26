import { cn } from "@/lib/utils";
import type { SidebarViewId } from "./types";

export const SIDEBAR_RAIL_HEIGHT = 36;
export type SidebarRailItemId = SidebarViewId | "github" | "projects";

type RailItem = {
  id: SidebarRailItemId;
  label: string;
};

type Props = {
  activeItem: SidebarRailItemId;
  onSelectItem: (item: SidebarRailItemId) => void;
  projectsBadge?: number;
};

export function SidebarRail({
  activeItem,
  onSelectItem,
  projectsBadge = 0,
}: Props) {
  const items: RailItem[] = [
    { id: "explorer", label: "Files" },
    { id: "source-control", label: "Git" },
    { id: "github", label: "GitHub" },
    { id: "projects", label: "Projects" },
  ];

  return (
    <nav
      aria-label="Workspace views"
      style={{ height: SIDEBAR_RAIL_HEIGHT }}
      className="flex shrink-0 items-stretch gap-1 border-b border-border/60 bg-card/85 px-1.5 py-1 backdrop-blur"
    >
      {items.map((item) => {
        const isActive = item.id === activeItem;
        const badge = item.id === "projects" ? projectsBadge : 0;
        return (
          <button
            key={item.id}
            type="button"
            aria-label={item.label}
            aria-pressed={isActive}
            onClick={() => onSelectItem(item.id)}
            className={cn(
              "group relative flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 rounded-md px-1 text-[10.5px] font-medium outline-none transition-colors duration-150",
              "focus-visible:ring-2 focus-visible:ring-primary/40",
              isActive
                ? "bg-foreground/[0.07] text-foreground dark:bg-foreground/[0.09]"
                : "text-muted-foreground hover:bg-foreground/[0.045] hover:text-foreground",
            )}
          >
            <span className="truncate">{item.label}</span>
            {badge > 0 ? (
              <span className="rounded-full bg-violet-500/15 px-1 text-[9px] font-semibold tabular-nums text-violet-500">
                {badge > 9 ? "9+" : badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}
