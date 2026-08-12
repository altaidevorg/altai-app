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
 * Flat collapsible section — History-style group header, no nested card chrome.
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
    <section className="border-b border-border-subtle">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        className="group flex w-full items-center gap-2 px-2.5 py-2 text-left transition-colors hover:bg-foreground/[0.03]"
      >
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] font-medium text-foreground">
              {title}
            </span>
            {count ? (
              <span className="rounded bg-foreground/[0.06] px-1.5 text-[10px] tabular-nums text-muted-foreground">
                {count}
              </span>
            ) : null}
          </div>
          <div className="mt-0.5 truncate text-[10.5px] text-muted-foreground">
            {summary}
          </div>
        </div>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={12}
          strokeWidth={2}
          className={cn(
            "shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180",
          )}
        />
      </button>
      {open ? <div className="px-2.5 pb-2.5 pt-0.5">{children}</div> : null}
    </section>
  );
}
