import { cn } from "../lib/cn.js";

export type CheckpointItem = {
  id: string;
  path: string;
  label: string;
  createdMs: number;
};

export type CheckpointMenuPanelProps = {
  items: CheckpointItem[];
  restoringId?: string | null;
  onRestore: (id: string) => void;
  /** Injected clock for relative-time labels in tests. Defaults to Date.now(). */
  nowMs?: number;
};

export function checkpointBasename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

export function formatCheckpointTimeAgo(
  createdMs: number,
  nowMs: number = Date.now(),
): string {
  const secs = Math.floor((nowMs - createdMs) / 1000);
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

/**
 * Presentational body for the edit-checkpoint popover. The host owns open
 * state, Popover chrome, and native checkpoint list/restore calls.
 */
export function CheckpointMenuPanel({
  items,
  restoringId = null,
  onRestore,
  nowMs,
}: CheckpointMenuPanelProps) {
  const clock = nowMs ?? Date.now();

  return (
    <div className="altai-checkpoint-menu-panel">
      <div className="border-b border-border/70 px-3 py-2.5">
        <div className="text-[12px] font-medium">Edit checkpoints</div>
        <div className="text-[11px] text-muted-foreground">
          Restore files to their state before the agent edited them.
        </div>
      </div>
      <div className="max-h-[16rem] overflow-y-auto">
        {items.length === 0 ? (
          <div className="px-3 py-6 text-center text-[11px] text-muted-foreground">
            No checkpoints yet. The runtime saves one before each edit.
          </div>
        ) : (
          <ul className="divide-y divide-border/40">
            {items.map((c) => {
              const restoring = restoringId === c.id;
              return (
                <li
                  key={c.id}
                  className="flex items-center gap-2 px-3 py-2 hover:bg-muted/50"
                >
                  <div className="min-w-0 flex-1">
                    <div
                      className="truncate text-[11px] font-medium"
                      title={c.path}
                    >
                      {checkpointBasename(c.path)}
                    </div>
                    <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
                      <span>{c.label}</span>
                      <span>·</span>
                      <span>{formatCheckpointTimeAgo(c.createdMs, clock)}</span>
                    </div>
                  </div>
                  <button
                    type="button"
                    disabled={restoring}
                    onClick={() => onRestore(c.id)}
                    className={cn(
                      "inline-flex h-6 items-center justify-center rounded-md border border-border/70 bg-secondary px-2 text-[10.5px] font-medium text-secondary-foreground transition-colors hover:bg-secondary/80 disabled:opacity-50",
                    )}
                  >
                    {restoring ? "Restoring…" : "Restore"}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
