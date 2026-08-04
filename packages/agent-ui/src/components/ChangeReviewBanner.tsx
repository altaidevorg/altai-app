import { FileEditIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type ChangeReviewBannerProps = {
  queueLen: number;
  onOpen: () => void;
};

/**
 * Banner shown when planned edits are waiting for review. Purely
 * presentational; the host supplies the queue length and open handler.
 */
export function ChangeReviewBanner({
  queueLen,
  onOpen,
}: ChangeReviewBannerProps) {
  if (queueLen === 0) return null;
  return (
    <div className="altai-ai-review-banner mx-3 mb-2 flex shrink-0 items-center gap-2.5 rounded-lg border border-primary/20 bg-primary/[0.055] px-3 py-2">
      <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
        <HugeiconsIcon icon={FileEditIcon} size={13} strokeWidth={1.8} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-[11px] font-medium text-foreground">
          Changes ready
        </span>
        <span className="block truncate text-[10px] text-muted-foreground">
          {queueLen} proposed change{queueLen === 1 ? "" : "s"} waiting for
          review
        </span>
      </span>
      <button
        type="button"
        onClick={onOpen}
        className="rounded-md bg-primary px-2.5 py-1.5 text-[10.5px] font-medium text-primary-foreground transition-colors hover:bg-primary/90"
      >
        Review changes
      </button>
    </div>
  );
}
