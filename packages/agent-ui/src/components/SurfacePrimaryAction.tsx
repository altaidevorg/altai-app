import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type SurfacePrimaryActionProps = {
  children: ReactNode;
  onClick?: () => void;
  type?: "button" | "submit";
  className?: string;
  disabled?: boolean;
};

/**
 * Primary header action used by Work / Automations auxiliary surfaces
 * (Delegate work, New schedule, …). Host supplies label + icon children.
 */
export function SurfacePrimaryAction({
  children,
  onClick,
  type = "button",
  className,
  disabled,
}: SurfacePrimaryActionProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-md bg-primary px-2.5 text-[9.5px] font-semibold text-primary-foreground hover:bg-primary/85 disabled:pointer-events-none disabled:opacity-50",
        className,
      )}
    >
      {children}
    </button>
  );
}

export type SurfaceSecondaryActionProps = {
  children: ReactNode;
  onClick?: () => void;
  type?: "button" | "submit";
  className?: string;
  disabled?: boolean;
};

/**
 * Secondary header action (Queue / Schedules back buttons).
 */
export function SurfaceSecondaryAction({
  children,
  onClick,
  type = "button",
  className,
  disabled,
}: SurfaceSecondaryActionProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-muted px-2.5 text-[9.5px] font-medium text-foreground hover:bg-accent disabled:pointer-events-none disabled:opacity-50",
        className,
      )}
    >
      {children}
    </button>
  );
}
