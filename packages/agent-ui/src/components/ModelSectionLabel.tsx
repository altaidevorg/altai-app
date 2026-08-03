import type { ReactNode } from "react";

export type ModelSectionLabelProps = {
  children: ReactNode;
};

/**
 * Section heading used inside the model dropdown list (Pinned / Recent /
 * All models). Purely presentational.
 */
export function ModelSectionLabel({ children }: ModelSectionLabelProps) {
  return (
    <div className="px-3 pt-2 pb-1 text-[9px] font-medium tracking-[0.12em] text-muted-foreground/70">
      {children}
    </div>
  );
}
