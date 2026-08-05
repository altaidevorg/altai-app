import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type SurfaceLoadingStateProps = {
  children: ReactNode;
  /** Centered list-panel loading (tasks queue) vs compact inline (automations). */
  density?: "panel" | "inline";
  className?: string;
};

/**
 * Loading row/chrome for Work queue and Automations list. Host supplies Spinner
 * (or any indicator) as children so Desktop keeps its own Spinner component.
 */
export function SurfaceLoadingState({
  children,
  density = "panel",
  className,
}: SurfaceLoadingStateProps) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 text-muted-foreground",
        density === "panel"
          ? "justify-center py-8 text-[11px]"
          : "text-[10px]",
        className,
      )}
    >
      {children}
    </div>
  );
}
