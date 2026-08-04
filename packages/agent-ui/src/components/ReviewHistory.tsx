import { HistoryRow } from "./HistoryRow.js";

export type ReviewHistoryItem = {
  id: string;
  path: string;
  detail: string;
};

export type ReviewHistoryProps = {
  items: ReviewHistoryItem[];
  restoringId: string | null;
  error: string | null;
  onRestore: (id: string) => void;
};

/**
 * Restore-points section for the plan review centre. Purely presentational;
 * the host supplies formatted rows and owns restore transport.
 */
export function ReviewHistory({
  items,
  restoringId,
  error,
  onRestore,
}: ReviewHistoryProps) {
  if (!items.length) return null;

  return (
    <section className="border-t border-border/45 pt-3">
      <div className="mb-1.5 px-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        Restore points
      </div>
      <p className="mb-2 px-0.5 text-[10px] leading-relaxed text-muted-foreground">
        Every agent edit has a pre-edit snapshot. Restoring a new file removes
        it; restoring an existing file puts its prior content back.
      </p>
      {error ? (
        <p className="mb-2 rounded-md bg-destructive/10 px-2 py-1.5 text-[10px] text-destructive">
          {error}
        </p>
      ) : null}
      <div className="space-y-1.5">
        {items.map((item) => (
          <HistoryRow
            key={item.id}
            path={item.path}
            detail={item.detail}
            restoring={restoringId === item.id}
            onRestore={() => onRestore(item.id)}
          />
        ))}
      </div>
    </section>
  );
}
