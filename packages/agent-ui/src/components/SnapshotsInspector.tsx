import { HistoryRow } from "./HistoryRow.js";
import { InspectorEmpty } from "./InspectorEmpty.js";

export type SnapshotsInspectorAppliedItem = {
  id: string;
  path: string;
  isNewFile: boolean;
};

export type SnapshotsInspectorCheckpointItem = {
  id: string;
  path: string;
  label: string;
};

export type SnapshotsInspectorProps = {
  applied: SnapshotsInspectorAppliedItem[];
  items: SnapshotsInspectorCheckpointItem[];
  restoringId: string | null;
  error: string | null;
  onRestoreApplied: (id: string) => void;
  onRestoreCheckpoint: (id: string) => void;
};

/**
 * Recovery snapshots as flat grouped lists.
 */
export function SnapshotsInspector({
  applied,
  items,
  restoringId,
  error,
  onRestoreApplied,
  onRestoreCheckpoint,
}: SnapshotsInspectorProps) {
  if (!items.length && !applied.length) {
    return (
      <InspectorEmpty>
        Before-agent-edit and reviewed-change snapshots will appear here, ready
        to restore safely.
      </InspectorEmpty>
    );
  }

  return (
    <div>
      {applied.length ? (
        <section>
          <div className="px-0.5 pb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            Plan review
          </div>
          <ul className="divide-y divide-border-subtle">
            {[...applied].reverse().map((item) => (
              <li key={item.id}>
                <HistoryRow
                  path={item.path}
                  detail={`Accepted change · ${item.isNewFile ? "removes new file" : "restores prior content"}`}
                  restoring={restoringId === item.id}
                  onRestore={() => onRestoreApplied(item.id)}
                />
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {items.length ? (
        <section className={applied.length ? "mt-2" : undefined}>
          <div className="px-0.5 pb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            Agent edits
          </div>
          <ul className="divide-y divide-border-subtle">
            {items.map((item) => (
              <li key={item.id}>
                <HistoryRow
                  path={item.path}
                  detail={item.label}
                  restoring={restoringId === item.id}
                  onRestore={() => onRestoreCheckpoint(item.id)}
                />
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {error ? (
        <div className="mt-2 border border-destructive/20 bg-destructive/[0.05] px-2 py-1.5 text-[11px] text-destructive">
          {error}
        </div>
      ) : null}
    </div>
  );
}
