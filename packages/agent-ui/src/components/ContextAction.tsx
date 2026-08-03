import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";

export type ContextActionProps = {
  icon: IconSvgElement;
  label: string;
  detail: string;
  disabled: boolean;
  onClick: () => void;
};

/**
 * Contextual menu row used by the composer "attach" popover (active file,
 * workspace map, terminal output, working tree diff). Purely presentational;
 * the host owns the click handlers and disabled state.
 */
export function ContextAction({
  icon,
  label,
  detail,
  disabled,
  onClick,
}: ContextActionProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-popover-foreground disabled:opacity-40 hover:bg-foreground/[0.055]"
    >
      <HugeiconsIcon
        icon={icon}
        size={13}
        strokeWidth={1.75}
        className="shrink-0 text-muted-foreground"
      />
      <span className="min-w-0">
        <span className="block text-[11px] font-medium">{label}</span>
        <span className="block truncate text-[9.5px] text-muted-foreground">
          {detail}
        </span>
      </span>
    </button>
  );
}
