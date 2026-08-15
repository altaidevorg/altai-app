import { useCallback, useEffect, useState } from "react";
import { formatRelativeTime } from "@altai/agent-ui";
import type { AuditEvent } from "@altai/host-contract";
import { cn } from "@/lib/utils";
import { native } from "@/modules/ai/lib/native";
import {
  projectAuditFeed,
  type AuditFeedRow,
} from "./lib/auditFeedProjection";

type Props = {
  workspacePath: string;
  onOpenWork: (workId: string) => void;
  className?: string;
};

type LoadStatus = "loading" | "ready" | "error";

/**
 * Audit surface (package 065, PR 1). The store's transition log — every
 * decision, stop, and lifecycle move, recorded in the same transaction as
 * the mutation — as one workspace-wide feed. Each row names the Work it
 * happened to and drills into it; the feed is read-only, because the
 * audit trail is a record, not an editor.
 */
export function AuditPanel({ workspacePath, onOpenWork, className }: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [events, setEvents] = useState<AuditEvent[]>([]);

  const load = useCallback(async () => {
    try {
      const next = await native.workEventsRecent(50, workspacePath);
      setEvents(next);
      setError(null);
      setStatus("ready");
    } catch (loadError) {
      setError(
        loadError instanceof Error ? loadError.message : String(loadError),
      );
      setStatus("error");
    }
  }, [workspacePath]);

  useEffect(() => {
    setEvents([]);
    setStatus("loading");
    void load();
  }, [load]);

  const rows = projectAuditFeed(events);

  return (
    <div
      className={cn(
        "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-baseline gap-2 border-b border-border-subtle px-3 py-2">
        <h2 className="min-w-0 flex-1 text-[13px] font-semibold text-foreground">
          Audit
        </h2>
        <p className="shrink-0 text-[10px] text-muted-foreground">
          Decisions, stops and transitions
        </p>
      </header>

      {status === "loading" ? (
        <p className="px-3 py-6 text-[11px] text-muted-foreground">
          Loading the audit feed…
        </p>
      ) : null}
      {status === "error" ? (
        <div className="m-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-[11px] text-red-500">
          <p>{error ?? "Audit feed failed to load."}</p>
          <button
            type="button"
            onClick={() => void load()}
            className="mt-1 underline underline-offset-2"
          >
            Retry
          </button>
        </div>
      ) : null}

      {status === "ready" && rows.length === 0 ? (
        <p className="px-3 py-6 text-center text-[11px] text-muted-foreground">
          No recorded activity yet — decisions and stops land here as they
          happen.
        </p>
      ) : null}

      {status === "ready" && rows.length > 0 ? (
        <ul className="min-h-0 flex-1 divide-y divide-border-subtle overflow-y-auto">
          {rows.map((row) => (
            <AuditRow key={row.id} row={row} onOpenWork={onOpenWork} />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function AuditRow({
  row,
  onOpenWork,
}: {
  row: AuditFeedRow;
  onOpenWork: (workId: string) => void;
}) {
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpenWork(row.workId)}
        className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors hover:bg-muted/50"
        aria-label={`${row.label} — ${row.workTitle}`}
      >
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[12px] text-foreground">
            {row.label}
            {row.detail ? (
              <span className="ml-1.5 font-mono text-[10.5px] text-muted-foreground">
                {row.detail}
              </span>
            ) : null}
          </span>
          <span className="block truncate text-[10.5px] text-muted-foreground">
            {row.workTitle}
          </span>
        </span>
        <span className="shrink-0 text-[10.5px] text-muted-foreground">
          {formatRelativeTime(row.atMs)}
        </span>
      </button>
    </li>
  );
}
