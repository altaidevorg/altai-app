import { Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type AgentOptionRowProps = {
  name: string;
  description?: string | null;
  icon: IconSvgElement;
  selected: boolean;
  /**
   * When true, the leading icon stays muted even if selected (custom agents).
   * Built-in / ML rows leave this false so the active icon lifts to foreground.
   */
  iconAlwaysMuted?: boolean;
  className?: string;
};

/**
 * Shared agent-picker option chrome (icon, name, description, checkmark).
 * Host wraps with DropdownMenuItem / button and owns selection handlers.
 */
export function AgentOptionRow({
  name,
  description = null,
  icon,
  selected,
  iconAlwaysMuted = false,
  className,
}: AgentOptionRowProps) {
  return (
    <span
      className={cn(
        "altai-agent-option-row flex min-w-0 flex-1 items-start gap-2",
        className,
      )}
    >
      <HugeiconsIcon
        icon={icon}
        size={13}
        strokeWidth={1.75}
        className={cn(
          "mt-0.5 shrink-0",
          iconAlwaysMuted || !selected
            ? "text-muted-foreground"
            : "text-foreground",
        )}
      />
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate">{name}</span>
        {description ? (
          <span className="line-clamp-1 text-[10.5px] text-muted-foreground">
            {description}
          </span>
        ) : null}
      </span>
      {selected ? (
        <HugeiconsIcon
          icon={Tick02Icon}
          size={12}
          strokeWidth={2}
          className="mt-0.5 shrink-0 text-foreground"
        />
      ) : null}
    </span>
  );
}
