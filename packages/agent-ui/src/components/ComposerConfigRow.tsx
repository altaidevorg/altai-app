import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ComposerConfigRowProps = {
  agentSlot?: ReactNode;
  modelSlot: ReactNode;
  className?: string;
};

/**
 * Composer agent/model configuration row. Host supplies picker mounts
 * (AgentSwitcher / ModelDropdown bridges).
 */
export function ComposerConfigRow({
  agentSlot,
  modelSlot,
  className,
}: ComposerConfigRowProps) {
  const showAgent = Boolean(agentSlot);

  return (
    <div
      className={cn(
        "altai-composer-config-row grid w-full min-w-0 gap-1 border-t border-border-subtle px-2.5 py-1.5",
        showAgent ? "grid-cols-2" : "grid-cols-1",
        className,
      )}
      aria-label="Chat configuration"
    >
      {showAgent ? (
        <span className="altai-ai-composer-config-item altai-ai-composer-agent inline-flex min-w-0">
          {agentSlot}
        </span>
      ) : null}
      <span className="altai-ai-composer-config-item altai-ai-composer-model inline-flex min-w-0">
        {modelSlot}
      </span>
    </div>
  );
}
