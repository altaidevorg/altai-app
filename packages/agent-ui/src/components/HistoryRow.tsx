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
 * Restore-point row used by the plan diff review history panel. Renders the
 * file basename with a tooltip of the full path and a Restore button. The
 * host owns the restore orchestration (native checkpoint restore, stores).
 */
export function HistoryRow({
  path,
  detail,
  restoring,
  onRestore,
}: HistoryRowProps) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2">
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
          className="truncate text-[9.5px] text-muted-foreground"
          title={detail}
        >
          {detail}
        </div>
      </div>
      <button
        type="button"
        disabled={restoring}
        onClick={onRestore}
        className="h-6 rounded px-1.5 text-[10px] text-foreground hover:bg-foreground/[0.055] disabled:opacity-40"
      >
        {restoring ? "Restoring…" : "Restore"}
      </button>
    </div>
  );
}
