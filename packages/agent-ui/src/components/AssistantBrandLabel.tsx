import { SparklesIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type AssistantBrandLabelProps = {
  /** Brand label, defaults to ALTAI. */
  brand?: string;
  streaming?: boolean;
  streamingLabel?: string;
  className?: string;
};

/**
 * Assistant message header: brand mark + optional streaming hint.
 * Wave 4 / A6.5.
 */
export function AssistantBrandLabel({
  brand = "ALTAI",
  streaming = false,
  streamingLabel = "thinking…",
  className,
}: AssistantBrandLabelProps) {
  return (
    <div
      className={cn(
        "altai-ai-assistant-label mb-0.5 flex items-center gap-1.5 text-[9.5px] font-semibold uppercase tracking-[0.1em] text-muted-foreground",
        className,
      )}
    >
      <span className="flex size-5 items-center justify-center rounded-md bg-primary/10 text-primary">
        <HugeiconsIcon icon={SparklesIcon} size={11} strokeWidth={1.8} />
      </span>
      {brand}
      {streaming ? (
        <span className="ml-0.5 font-normal normal-case tracking-normal text-muted-foreground/75">
          {streamingLabel}
        </span>
      ) : null}
    </div>
  );
}
