import { formatRelativeTime } from "@altai/agent-ui";
import type { WorkTimelineRow } from "./lib/runTimelineProjection";

type Props = {
  rows: readonly WorkTimelineRow[];
};

/**
 * The timeline slice of the Run Inspector (package 063, PR 1): the Work
 * store's own transition log, oldest first. Label and detail stay separate
 * columns; a row without a typed fact renders no detail slot.
 */
export function WorkTimelineSection({ rows }: Props) {
  if (rows.length === 0) return null;
  return (
    <section className="shrink-0 border-t border-border-subtle px-3 py-3">
      <h3 className="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground">
        Timeline
      </h3>
      <ol className="divide-y divide-border-subtle overflow-hidden rounded-lg border border-border">
        {rows.map((row) => (
          <li
            key={row.id}
            className="flex items-baseline gap-2 px-2.5 py-1.5 text-[11px]"
          >
            <span className="min-w-0 flex-1 truncate text-foreground">
              {row.label}
            </span>
            {row.detail ? (
              <span className="min-w-0 shrink-0 truncate font-mono text-[10.5px] text-muted-foreground">
                {row.detail}
              </span>
            ) : null}
            <span className="shrink-0 text-[10.5px] text-muted-foreground">
              {formatRelativeTime(row.atMs)}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
