import { SparklesIcon } from "@hugeicons/core-free-icons";
import { cn } from "../lib/cn.js";
import { SurfaceHeader } from "./AuxiliarySurface.js";

export type RunDetailsStatus = "idle" | "running" | "blocked";

export type RunDetailsHeaderProps = {
  subtitle: string;
  status: RunDetailsStatus;
  onClose?: () => void;
  onStop?: () => void;
};

/**
 * Run details panel header: title, status pill, optional stop action.
 * Host owns stopAgent and subtitle derivation from run meta.
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
    <SurfaceHeader
      title="Run details"
      eyebrow="Current run"
      icon={SparklesIcon}
      subtitle={subtitle}
      status={
        <span
          className={cn(
            "rounded px-1.5 py-0.5 text-[8.5px] font-semibold",
            status === "running"
              ? "bg-primary/10 text-primary"
              : status === "blocked"
                ? "bg-destructive/10 text-destructive"
                : "bg-muted text-muted-foreground",
          )}
        >
          {statusLabel}
        </span>
      }
      onClose={onClose}
      actions={
        status === "running" && onStop ? (
          <button
            type="button"
            onClick={onStop}
            className="rounded-md border border-destructive/25 bg-destructive/[0.06] px-2 py-1 text-[9.5px] font-medium text-destructive hover:bg-destructive/10"
          >
            Stop run
          </button>
        ) : null
      }
    />
  );
}
