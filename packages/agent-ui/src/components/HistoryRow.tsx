import { FileEditIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type HistoryRowProps = {
  path: string;
  detail: string;
  restoring: boolean;
  onRestore: () => void;
};

function basename(p: string): string {
  const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return i >= 0 ? p.slice(i + 1) : p;
}

/**
 * Restore-point row used by recovery / plan history.
 */
export function HistoryRow({
  path,
  detail,
  restoring,
  onRestore,
}: HistoryRowProps) {
  return (
    <div className="flex items-center gap-2 py-2">
      <HugeiconsIcon
        icon={FileEditIcon}
        size={12}
        strokeWidth={1.75}
        className="shrink-0 text-muted-foreground"
      />
      <div className="min-w-0 flex-1">
        <div
          className="truncate text-[11px] font-medium text-foreground"
          title={path}
        >
          {basename(path)}
        </div>
        <div
          className="truncate text-[10.5px] text-muted-foreground"
          title={detail}
        >
          {detail}
        </div>
      </div>
      <button
        type="button"
        disabled={restoring}
        onClick={onRestore}
        className="inline-flex h-7 items-center rounded-md px-2 text-[11px] text-foreground transition-colors hover:bg-foreground/[0.06] disabled:opacity-40"
      >
        {restoring ? "Restoring…" : "Restore"}
      </button>
    </div>
  );
}
