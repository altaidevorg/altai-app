import { ArrowRight01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useId, useState, type ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type TranscriptToolGroupProps = {
  /** Short group name shown next to the icon (`Read`, `Web`, `Ran`). */
  label: string;
  /** Count phrase, e.g. `3 files` / `2 calls`. */
  countLabel: string;
  /** Collapsed-row preview (file basenames, queries, command heads). */
  preview?: string;
  /** Leading status icon (file/web/terminal). */
  icon: ReactNode;
  /** Expanded body (rows or inlined tool cards). */
  children: ReactNode;
  /** Uncontrolled open seed; host may remount with a new key to reset. */
  defaultOpen?: boolean;
  className?: string;
  previewMono?: boolean;
};

/**
 * Collapsible tool-call group for consecutive transcript tool cards.
 *
 * Wave 4 / A6.2: host-neutral chrome so Desktop AiChat and a future shared
 * transcript renderer collapse file/web/shell bursts the same way. Uses a
 * native state button (no Radix) so VS Code can import without extra deps.
 */
export function TranscriptToolGroup({
  label,
  countLabel,
  preview,
  icon,
  children,
  defaultOpen = false,
  className,
  previewMono = false,
}: TranscriptToolGroupProps) {
  const [open, setOpen] = useState(defaultOpen);
  const panelId = useId();

  return (
    <div
      className={cn(
        "min-w-0 max-w-full overflow-hidden rounded-md border border-border/50 bg-card/50",
        className,
      )}
      data-state={open ? "open" : "closed"}
    >
      <button
        type="button"
        className={cn(
          "flex w-full min-w-0 items-center gap-2 px-2 py-1.5 text-left text-[12px]",
          "transition-colors hover:bg-muted/50",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        )}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => {
          setOpen((value) => !value);
        }}
      >
        <HugeiconsIcon
          icon={ArrowRight01Icon}
          size={11}
          strokeWidth={2}
          className={cn(
            "shrink-0 text-muted-foreground transition-transform",
            open ? "rotate-90" : null,
          )}
        />
        <span className="shrink-0 text-muted-foreground">{icon}</span>
        <span className="shrink-0 font-medium text-foreground">{label}</span>
        <span className="shrink-0 text-[11px] text-muted-foreground">
          {countLabel}
        </span>
        {preview && !open ? (
          <span
            className={cn(
              "min-w-0 flex-1 truncate text-[11px] text-muted-foreground/80",
              previewMono ? "font-mono" : null,
            )}
          >
            · {preview}
          </span>
        ) : null}
      </button>
      {open ? (
        <div
          id={panelId}
          className="border-t border-border/30"
          data-altai-tool-group-panel=""
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}
