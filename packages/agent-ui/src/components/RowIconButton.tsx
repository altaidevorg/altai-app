import type { MouseEvent, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type RowIconButtonProps = {
  title: string;
  onClick: (e: MouseEvent) => void;
  tone?: "destructive";
  children?: ReactNode;
};

/**
 * Compact icon button used in chat history and session list rows. Supports
 * a destructive tone for delete/danger actions. Purely presentational;
 * the host owns the click handler.
 */
export function RowIconButton({
  title,
  onClick,
  tone,
  children,
}: RowIconButtonProps) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onClick}
      className={cn(
        "inline-flex size-5 items-center justify-center rounded transition-colors",
        tone === "destructive"
          ? "text-muted-foreground/80 hover:bg-destructive/10 hover:text-destructive"
          : "text-muted-foreground/80 hover:bg-foreground/10 hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
