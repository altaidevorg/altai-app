import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type RunDetailsStatus = "idle" | "running" | "blocked";

export type RunDetailsHeaderProps = {
  subtitle: string;
  status: RunDetailsStatus;
  onClose?: () => void;
  onStop?: () => void;
};

/**
 * Flat Details toolbar — matches Files / History / Desktop `h-10` chrome.
 */
export function RunDetailsHeader({
  subtitle,
  status,
  onClose,
  onStop,
}: RunDetailsHeaderProps) {
  const statusLabel =
    status === "blocked" ? "Blocked" : status === "running" ? "Running" : "Idle";

  return (
    <header className="flex h-10 shrink-0 items-center gap-2 border-b border-border-subtle bg-raised px-2.5">
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <h2 className="truncate text-[12px] font-semibold text-foreground">
            Details
          </h2>
          <span
            className={cn(
              "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium",
              status === "running"
                ? "bg-foreground/[0.08] text-foreground"
                : status === "blocked"
                  ? "bg-destructive/10 text-destructive"
                  : "bg-muted text-muted-foreground",
            )}
          >
            {statusLabel}
          </span>
        </div>
        <p className="truncate text-[10.5px] text-muted-foreground" title={subtitle}>
          {subtitle}
        </p>
      </div>
      {status === "running" && onStop ? (
        <button
          type="button"
          onClick={onStop}
          className="inline-flex h-7 shrink-0 items-center rounded-md border border-border px-2 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/[0.06]"
        >
          Stop run
        </button>
      ) : null}
      {onClose ? (
        <button
          type="button"
          onClick={onClose}
          aria-label="Close details"
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
        >
          <HugeiconsIcon icon={Cancel01Icon} size={13} strokeWidth={1.75} />
        </button>
      ) : null}
    </header>
  );
}
