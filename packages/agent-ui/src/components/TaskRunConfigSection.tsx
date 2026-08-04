import type { ReactNode } from "react";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type TaskRunConfigSectionProps = {
  children: ReactNode;
};

/**
 * Create-task run-configuration section chrome. Host mounts agent / model /
 * permission pickers as children.
 */
export function TaskRunConfigSection({ children }: TaskRunConfigSectionProps) {
  return (
    <section className="altai-task-run-config-section px-3.5 py-3.5">
      <SurfaceSectionHeader
        title="Run configuration"
        description="Choose how the isolated agent should work."
      />
      <div className="mt-3 flex flex-wrap items-center gap-1.5">{children}</div>
    </section>
  );
}
