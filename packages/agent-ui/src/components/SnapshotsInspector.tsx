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
 * Run-inspector snapshots panel: reviewed plan edits and pre-edit agent
 * checkpoints. Purely presentational; the host owns restore transport.
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
    <div className="space-y-2">
      {applied.length ? (
        <section className="space-y-2">
          <div className="px-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            Plan review
          </div>
          {[...applied].reverse().map((item) => (
            <HistoryRow
              key={item.id}
              path={item.path}
              detail={`Accepted change · ${item.isNewFile ? "removes new file" : "restores prior content"}`}
              restoring={restoringId === item.id}
              onRestore={() => onRestoreApplied(item.id)}
            />
          ))}
        </section>
      ) : null}
      {items.length ? (
        <div className="px-1 pt-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
          Agent edits
        </div>
      ) : null}
      {items.map((item) => (
        <HistoryRow
          key={item.id}
          path={item.path}
          detail={item.label}
          restoring={restoringId === item.id}
          onRestore={() => onRestoreCheckpoint(item.id)}
        />
      ))}
      {error ? (
        <div className="border border-destructive/30 bg-destructive/[0.06] p-2 text-[10.5px] text-destructive">
          {error}
        </div>
      ) : null}
    </div>
  );
}
