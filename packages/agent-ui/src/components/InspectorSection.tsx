import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState, type ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type InspectorSectionProps = {
  title: string;
  summary: string;
  count: number;
  defaultOpen?: boolean;
  children?: ReactNode;
};

/**
 * Collapsible section chrome for the run inspector. Purely presentational;
 * uses a local open toggle (no Radix) so hosts stay dependency-light.
 */
export function InspectorSection({
  title,
  summary,
  count,
  defaultOpen = false,
  children,
}: InspectorSectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="overflow-hidden rounded-lg border border-border bg-card">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        className="group flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-accent/60"
      >
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="text-[10.5px] font-semibold text-foreground">
              {title}
            </span>
            {count ? (
              <span className="rounded bg-foreground/[0.06] px-1.5 text-[8.5px] tabular-nums text-muted-foreground">
                {count}
              </span>
            ) : null}
          </div>
          <div className="mt-0.5 truncate text-[9px] text-muted-foreground">
            {summary}
          </div>
        </div>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={11}
          strokeWidth={2}
          className={cn(
            "shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180",
          )}
        />
      </button>
      {open ? (
        <div className="border-t border-border-subtle bg-muted/10 p-2.5">
          {children}
        </div>
      ) : null}
    </div>
  );
}
