import { cn } from "../lib/cn.js";

export type ComposerFollowupBarProps = {
  hint: string;
  showSteer?: boolean;
  showQueue?: boolean;
  canSteer?: boolean;
  canQueue?: boolean;
  steerTitle?: string;
  queueTitle?: string;
  onSteer?: () => void;
  onQueue?: () => void;
  className?: string;
};

/**
 * Active-run follow-up strip: hint copy plus Steer / Queue actions.
 * Host owns composer policy and whether the bar is mounted.
 */
export function ComposerFollowupBar({
  hint,
  showSteer = false,
  showQueue = false,
  canSteer = false,
  canQueue = false,
  steerTitle,
  queueTitle,
  onSteer,
  onQueue,
  className,
}: ComposerFollowupBarProps) {
  return (
    <div
      className={cn(
        "altai-composer-followup-bar flex items-center gap-1.5 border-t border-border-subtle px-2.5 py-1.5",
        className,
      )}
    >
      <span className="min-w-0 flex-1 truncate text-[10px] text-muted-foreground">
        {hint}
      </span>
      {showSteer ? (
        <button
          type="button"
          onClick={onSteer}
          disabled={!canSteer}
          title={steerTitle}
          className="h-6 rounded-md px-2 text-[11px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-45"
        >
          Steer now
        </button>
      ) : null}
      {showQueue ? (
        <button
          type="button"
          onClick={onQueue}
          disabled={!canQueue}
          title={queueTitle}
          className="h-6 rounded-md bg-secondary px-2 text-[11px] text-secondary-foreground transition-colors hover:bg-secondary/80 disabled:pointer-events-none disabled:opacity-45"
        >
          Queue next
        </button>
      ) : null}
    </div>
  );
}
