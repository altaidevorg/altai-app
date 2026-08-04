import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type IconBtnProps = {
  title: string;
  onClick: () => void;
  disabled?: boolean;
  className?: string;
  children?: ReactNode;
};

/**
 * Compact ghost icon button used in the AI status bar controls. Replaces
 * Desktop's `Button` (variant=ghost, size=icon) with a native styled
 * `<button>`. Purely presentational.
 */
export function IconBtn({
  title,
  onClick,
  disabled,
  className,
  children,
}: IconBtnProps) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-foreground/[0.055] hover:text-foreground disabled:opacity-40",
        className,
      )}
    >
      {children}
    </button>
  );
}
