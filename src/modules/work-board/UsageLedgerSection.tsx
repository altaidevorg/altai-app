import { useCallback, useEffect, useState } from "react";
import { formatRelativeTime } from "@altai/agent-ui";
import type { WorkUsage } from "@altai/host-contract";
import { cn } from "@/lib/utils";
import { native } from "@/modules/ai/lib/native";
import {
  projectUsageLedger,
  summarizeUsageLedger,
  type UsageLedgerRow,
} from "./lib/usageLedgerProjection";

type Props = {
  workspacePath: string;
  onOpenWork: (workId: string) => void;
};

type LoadStatus = "loading" | "ready" | "error";

/**
 * Usage ledger (package 065, PR 2). Every run's token usage is already
 * durable in the agent event journal; this surface joins it to Work via
 * the attempt's chat binding. Rows drill into the Work that spent the
 * tokens. Token counts are the honest ledger — cost needs a pricing
 * source the host does not have yet.
 */
export function UsageLedgerSection({ workspacePath, onOpenWork }: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [entries, setEntries] = useState<WorkUsage[]>([]);

  const load = useCallback(async () => {
    try {
      const next = await native.workUsageRecent(20, workspacePath);
      setEntries(next);
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
    setEntries([]);
    setStatus("loading");
    void load();
  }, [load]);

  const rows = projectUsageLedger(entries);
  const summary = summarizeUsageLedger(rows);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      {status === "loading" ? (
        <p className="px-3 py-6 text-[11px] text-muted-foreground">
          Loading the usage ledger…
        </p>
      ) : null}
      {status === "error" ? (
        <div className="m-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-[11px] text-red-500">
          <p>{error ?? "Usage ledger failed to load."}</p>
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
          No attempts yet — token usage lands here as chats run.
        </p>
      ) : null}

      {status === "ready" && rows.length > 0 ? (
        <>
          <p className="shrink-0 border-b border-border-subtle px-3 py-1.5 text-[10.5px] text-muted-foreground">
            {summary.attemptCount} attempts · {summary.attributedCount}{" "}
            attributed · {summary.unattributedCount} without a chat ·{" "}
            {summary.totalTokens.toLocaleString("en-US")} total tokens
          </p>
          <ul className="min-h-0 flex-1 divide-y divide-border-subtle overflow-y-auto">
            {rows.map((row) => (
              <UsageRow key={row.id} row={row} onOpenWork={onOpenWork} />
            ))}
          </ul>
        </>
      ) : null}
    </div>
  );
}

function UsageRow({
  row,
  onOpenWork,
}: {
  row: UsageLedgerRow;
  onOpenWork: (workId: string) => void;
}) {
  return (
    <li>
      <button
        type="button"
        onClick={() => onOpenWork(row.workId)}
        className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors hover:bg-muted/50"
        aria-label={`${row.tokenLabel} — ${row.workTitle}`}
      >
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[12px] text-foreground">
            {row.workTitle}
            <span className="ml-1.5 font-mono text-[10.5px] text-muted-foreground">
              {row.attemptLabel} · {row.phaseLabel}
            </span>
          </span>
          <span
            className={cn(
              "block truncate text-[10.5px]",
              row.tokens ? "text-muted-foreground" : "text-muted-foreground/70",
            )}
          >
            {row.tokenLabel}
            {row.cacheLabel ? ` · ${row.cacheLabel}` : ""}
          </span>
        </span>
        <span className="shrink-0 text-[10.5px] text-muted-foreground">
          {formatRelativeTime(row.atMs)}
        </span>
      </button>
    </li>
  );
}
