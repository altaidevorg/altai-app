import { Alert02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type InboxLoadFailedProps = {
  onRetry: () => void;
};

/**
 * Inbox error state shown when the notification inbox could not be loaded.
 * Companion to `SurfaceEmptyState`; uses a destructive-tinted icon and a
 * retry action. Purely presentational.
 */
export function InboxLoadFailed({ onRetry }: InboxLoadFailedProps) {
  return (
    <div className="flex flex-col items-center justify-center px-4 py-12 text-center">
      <span className="inline-flex size-9 items-center justify-center rounded-full bg-destructive/10 text-destructive">
        <HugeiconsIcon icon={Alert02Icon} size={17} strokeWidth={1.75} />
      </span>
      <h3 className="mt-3 text-[11.5px] font-medium text-foreground">
        Inbox could not be loaded
      </h3>
      <button
        type="button"
        onClick={onRetry}
        className="mt-2 rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        Try again
      </button>
    </div>
  );
}
