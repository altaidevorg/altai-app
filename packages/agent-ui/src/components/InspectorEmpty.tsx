import type { ReactNode } from "react";

export type InspectorEmptyProps = {
  children?: ReactNode;
};

/**
 * Compact empty-state text used inside collapsible inspector sections.
 * Visually distinct from `SurfaceEmptyState` (no border, no icon, smaller
 * padding) — designed for tight inspector panels.
 */
export function InspectorEmpty({ children }: InspectorEmptyProps) {
  return (
    <div className="px-2 py-8 text-center text-[11px] leading-relaxed text-muted-foreground">
      {children}
    </div>
  );
}
