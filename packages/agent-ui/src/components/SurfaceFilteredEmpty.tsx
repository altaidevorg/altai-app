import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type SurfaceFilteredEmptyProps = {
  message?: ReactNode;
  onClear: () => void;
  clearLabel?: string;
  className?: string;
};

/**
 * Dashed empty state when a filtered list has no matches.
 * Host owns filter/query reset via `onClear`.
 */
export function SurfaceFilteredEmpty({
  message = "No items match this view.",
  onClear,
  clearLabel = "Clear filters",
  className,
}: SurfaceFilteredEmptyProps) {
  return (
    <div
      className={cn(
        "altai-surface-filtered-empty border border-dashed border-border px-4 py-8 text-center text-[11px] leading-relaxed text-muted-foreground",
        className,
      )}
    >
      {message}
      <button
        type="button"
        onClick={onClear}
        className="ml-1 font-medium text-foreground hover:underline"
      >
        {clearLabel}
      </button>
    </div>
  );
}
