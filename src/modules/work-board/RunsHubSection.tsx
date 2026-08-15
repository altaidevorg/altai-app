import { formatRelativeTime } from "@altai/agent-ui";
import type { WorkAttemptPhase } from "@altai/host-contract";
import type { WorkRunRow } from "./lib/runsHubProjection";

type Props = {
  rows: readonly WorkRunRow[];
  onOpenWork: (workId: string) => void;
  className?: string;
};

const PHASE_DOT: Record<WorkAttemptPhase, string> = {
  queued: "bg-zinc-400",
  running: "bg-sky-500",
  waiting: "bg-amber-500",
  succeeded: "bg-emerald-500",
  failed: "bg-red-500",
  cancelled: "bg-zinc-400",
};

/**
 * The Runs hub (package 063, PR 2): the workspace's recent attempts, each
 * opening its Work detail — where the timeline from PR 1 lives. Renders
 * nothing when the workspace has no runs; an empty hub is no hub. Status
 * and phase stay separate chips, carrying package 062's gate.
 */
export function RunsHubSection({ rows, onOpenWork, className }: Props) {
  if (rows.length === 0) return null;
  return (
    <section
      className={`flex max-h-[220px] shrink-0 flex-col border-t border-border-subtle bg-card ${className ?? ""}`}
    >
      <h3 className="shrink-0 px-3 pb-1 pt-2.5 text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground">
        Recent runs
      </h3>
      <ul className="min-h-0 flex-1 divide-y divide-border-subtle overflow-y-auto">
        {rows.map((row) => (
          <li key={row.id}>
            <button
              type="button"
              onClick={() => onOpenWork(row.workId)}
              className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors hover:bg-muted/50"
            >
              <span
                aria-hidden="true"
                className={`mt-1 size-1.5 shrink-0 self-center rounded-full ${PHASE_DOT[row.phase]}`}
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[12px] text-foreground">
                  {row.workTitle}
                </span>
                <span className="block text-[10.5px] text-muted-foreground">
                  {row.attemptLabel} · {row.statusLabel}
                </span>
              </span>
              <span className="shrink-0 text-[10.5px] text-muted-foreground">
                {formatRelativeTime(row.updatedMs)}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
