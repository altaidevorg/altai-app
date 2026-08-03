import { Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type FilteredEmptyInboxProps = {
  label: string;
  onShowAll: () => void;
};

/**
 * Empty state shown when an inbox filter produces no results. The host
 * computes the label text (e.g. "Nothing needs your attention" vs "No
 * updates to show"). Purely presentational.
 */
export function FilteredEmptyInbox({
  label,
  onShowAll,
}: FilteredEmptyInboxProps) {
  return (
    <div className="flex flex-col items-center justify-center px-4 py-12 text-center">
      <span className="inline-flex size-9 items-center justify-center rounded-full bg-muted text-muted-foreground">
        <HugeiconsIcon icon={Tick02Icon} size={17} strokeWidth={1.75} />
      </span>
      <h3 className="mt-3 text-[11.5px] font-medium text-foreground">
        {label}
      </h3>
      <button
        type="button"
        onClick={onShowAll}
        className="mt-2 rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        Show all inbox items
      </button>
    </div>
  );
}
