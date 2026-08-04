import { SidebarRightIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type AiOpenControlProps = {
  active?: boolean;
  onOpen: () => void;
  /** Shown in the native title attribute (host can include shortcut text). */
  title: string;
};

/**
 * Compact status-bar toggle that shows/hides the AI agent surface.
 * Presentational; motion/shortcut formatting stay on the host if desired.
 */
export function AiOpenControl({
  active = false,
  onOpen,
  title,
}: AiOpenControlProps) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className={cn(
        "inline-flex size-6 items-center justify-center rounded-md transition-colors",
        active
          ? "bg-accent text-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-foreground",
      )}
      aria-label={active ? "Hide AI agent" : "Show AI agent"}
      aria-pressed={active}
      title={title}
    >
      <HugeiconsIcon icon={SidebarRightIcon} size={14} strokeWidth={1.75} />
    </button>
  );
}
