import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type ProviderPillProps = {
  icon: IconSvgElement;
  title: string;
  active: boolean;
  onClick: () => void;
};

/**
 * Compact provider selector pill used by the model dropdown's provider rail.
 * Active state shows a primary accent bar on the right edge. Purely
 * presentational; the host owns the click handler and active state.
 */
export function ProviderPill({
  icon,
  title,
  active,
  onClick,
}: ProviderPillProps) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        "relative mx-auto flex size-7 items-center justify-center rounded-md transition-colors",
        active
          ? "bg-foreground/[0.085] text-popover-foreground after:absolute after:top-1.5 after:right-0 after:bottom-1.5 after:w-[2px] after:rounded-full after:bg-primary after:content-['']"
          : "text-muted-foreground hover:bg-foreground/[0.055]",
      )}
    >
      <HugeiconsIcon icon={icon} size={14} strokeWidth={1.5} />
    </button>
  );
}
