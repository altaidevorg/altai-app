import type { ReactNode } from "react";

export type RunActionRequiredSectionProps = {
  children: ReactNode;
  title?: string;
};

/**
 * Pinned “Action required” block above the section list.
 */
export function RunActionRequiredSection({
  children,
  title = "Action required",
}: RunActionRequiredSectionProps) {
  return (
    <section className="border-b border-border-subtle bg-card px-2.5 py-2">
      <div className="mb-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        {title}
      </div>
      {children}
    </section>
  );
}
