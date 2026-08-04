import type { ReactNode } from "react";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type InboxSectionProps = {
  title: string;
  count: number;
  children?: ReactNode;
};

/**
 * Section wrapper for inbox panels: renders a `SurfaceSectionHeader` followed
 * by a spaced content area. Purely presentational.
 */
export function InboxSection({ title, count, children }: InboxSectionProps) {
  return (
    <section>
      <SurfaceSectionHeader title={title} count={count} className="mb-2 px-0.5" />
      <div className="space-y-2">{children}</div>
    </section>
  );
}
