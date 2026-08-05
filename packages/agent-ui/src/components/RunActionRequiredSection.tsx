import type { ReactNode } from "react";

export type RunActionRequiredSectionProps = {
  children: ReactNode;
  title?: string;
};

/**
 * Warning-styled “Action required” wrapper above ApprovalsInspector in the
 * run details panel. Host supplies ApprovalsInspector + respond callbacks.
 */
export function RunActionRequiredSection({
  children,
  title = "Action required",
}: RunActionRequiredSectionProps) {
  return (
    <section>
      <div className="mb-1.5 px-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-warning">
        {title}
      </div>
      {children}
    </section>
  );
}
