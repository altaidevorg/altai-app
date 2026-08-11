import { cn } from "@/lib/utils";

/** Matches Header / AI topbar row height so chrome dividers align across columns. */
export const SIDEBAR_RAIL_HEIGHT = 40;

/** Top-rail destinations. Local source control lives inside GitHub (CommitBox)
 *  and remains available via the Source Control shortcut. */
export type SidebarRailItemId = "explorer" | "github" | "projects";

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
    { id: "github", label: "GitHub" },
    { id: "projects", label: "Work" },
  ];

  return (
    <nav
      aria-label="Workspace views"
      style={{ height: SIDEBAR_RAIL_HEIGHT }}
      className="flex shrink-0 items-stretch gap-1 border-b border-border-subtle bg-raised px-1.5"
    >
      {items.map((item) => {
        const isActive = item.id === activeItem;
        const badge = item.id === "projects" ? projectsBadge : 0;
        return (
          <button
            key={item.id}
            type="button"
            aria-label={item.label}
            title={item.label}
            aria-pressed={isActive}
            onClick={() => onSelectItem(item.id)}
            className={cn(
              "group relative flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 rounded-md px-1.5 text-[10.5px] font-medium outline-none transition-colors duration-150",
              "focus-visible:ring-2 focus-visible:ring-ring/40",
              isActive
                ? "bg-accent text-foreground"
                : "text-muted-foreground hover:bg-accent hover:text-foreground",
            )}
          >
            <span className="truncate">{item.label}</span>
            {badge > 0 ? (
              <span className="rounded-full bg-info/15 px-1 text-[9px] font-semibold tabular-nums text-info dark:bg-info/25">
                {badge > 9 ? "9+" : badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}
