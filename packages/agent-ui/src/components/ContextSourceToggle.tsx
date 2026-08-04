import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type ContextSourceToggleProps = {
  icon: IconSvgElement;
  label: string;
  detail: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  className?: string;
};

/**
 * Switch-style toggle row for selecting context sources (active file, terminal,
 * etc.) in the task runs panel. Purely presentational; the host owns the
 * checked state and change handler.
 */
export function ContextSourceToggle({
  icon,
  label,
  detail,
  checked,
  disabled,
  onChange,
  className,
}: ContextSourceToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "flex w-full items-center gap-2 p-2.5 text-left transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent",
        className,
      )}
    >
      <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
        <HugeiconsIcon icon={icon} size={13} strokeWidth={1.75} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-[10.5px] font-medium text-foreground">
          {label}
        </span>
        <span className="block truncate text-[9px] text-muted-foreground">
          {detail}
        </span>
      </span>
      <span
        aria-hidden="true"
        className={cn(
          "relative h-4 w-7 shrink-0 rounded-full border transition-colors",
          checked
            ? "border-primary bg-primary"
            : "border-border bg-muted",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 size-2.5 rounded-full bg-background shadow-sm transition-transform",
            checked ? "translate-x-3" : "translate-x-0.5",
          )}
        />
      </span>
    </button>
  );
}
