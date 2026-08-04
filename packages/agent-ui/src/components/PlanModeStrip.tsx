export type PlanModeStripProps = {
  active: boolean;
  queueLen: number;
  onReview: () => void;
  onExit: () => void;
};

/**
 * Status strip shown while plan mode is active. Purely presentational;
 * the host supplies active state, queue length, and action handlers.
 */
export function PlanModeStrip({
  active,
  queueLen,
  onReview,
  onExit,
}: PlanModeStripProps) {
  if (!active) return null;
  return (
    <div className="flex shrink-0 items-center gap-2 border-b border-border-subtle bg-warning/[0.035] px-3 py-1.5">
      <span className="size-1.5 shrink-0 rounded-full bg-warning" />
      <span className="text-[11px] font-medium text-foreground">Plan mode</span>
      <span className="text-[11px] text-muted-foreground">
        {queueLen > 0 ? `· ${queueLen} queued` : "· no edits queued"}
      </span>
      <span className="flex-1" />
      {queueLen > 0 ? (
        <button
          type="button"
          onClick={onReview}
          className="rounded-md px-1.5 py-0.5 text-[10.5px] font-medium text-foreground transition-colors hover:bg-foreground/[0.06]"
        >
          Review
        </button>
      ) : null}
      <button
        type="button"
        onClick={onExit}
        className="rounded-md px-1.5 py-0.5 text-[10.5px] text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
      >
        Exit
      </button>
    </div>
  );
}
