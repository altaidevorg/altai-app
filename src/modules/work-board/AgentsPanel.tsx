import { useCallback, useEffect, useState } from "react";
import type { AgentRecord, AgentStatusInput } from "@altai/host-contract";
import { cn } from "@/lib/utils";
import { native } from "@/modules/ai/lib/native";
import {
  projectAgentsAdmin,
  toAgentAdminError,
  toManagerOptions,
  type AgentAdminRow,
} from "./lib/agentsAdminProjection";

type Props = {
  workspacePath: string;
  className?: string;
};

type LoadStatus = "loading" | "ready" | "error";

const STATUS_DOT: Record<AgentStatusInput, string> = {
  active: "bg-emerald-500",
  paused: "bg-amber-500",
  terminated: "bg-zinc-400",
};

/**
 * Agents admin surface (package 064, PR 2). The registry the embedded
 * host made durable in PR 1, administered: every lifecycle and reporting
 * mutation is a store command — the surface never edits local state as if
 * it were the registry — and the store's rejections (cycles, terminal
 * agents) surface as inline errors, never swallowed.
 */
export function AgentsPanel({ workspacePath, className }: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [actionError, setActionError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [newManagerId, setNewManagerId] = useState("");

  const load = useCallback(async () => {
    try {
      const next = await native.agentList(workspacePath);
      setAgents(next);
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
    setAgents([]);
    setActionError(null);
    setBusyId(null);
    setConfirmingId(null);
    setStatus("loading");
    void load();
  }, [load]);

  const run = useCallback(
    async (id: string | null, action: () => Promise<unknown>) => {
      setBusyId(id);
      setActionError(null);
      try {
        await action();
        await load();
      } catch (actionFailure) {
        setActionError(
          toAgentAdminError(
            actionFailure instanceof Error
              ? actionFailure.message
              : String(actionFailure),
          ),
        );
      } finally {
        setBusyId(null);
        setConfirmingId(null);
      }
    },
    [load],
  );

  const create = useCallback(() => {
    const name = newName.trim();
    if (!name) {
      setActionError("An agent needs a name.");
      return;
    }
    void run(null, () =>
      native.agentCreate(name, newManagerId || null, workspacePath),
    );
    setNewName("");
    setNewManagerId("");
  }, [newName, newManagerId, run, workspacePath]);

  const transition = useCallback(
    (row: AgentAdminRow, next: AgentStatusInput) => {
      void run(row.id, () =>
        native.agentTransition(row.id, next, workspacePath),
      );
    },
    [run, workspacePath],
  );

  const setReporting = useCallback(
    (row: AgentAdminRow, reportsTo: string) => {
      void run(row.id, () =>
        native.agentSetReporting(row.id, reportsTo || null, workspacePath),
      );
    },
    [run, workspacePath],
  );

  const { rows } = projectAgentsAdmin(agents);
  const createOptions = toManagerOptions(agents, null);

  return (
    <div
      className={cn(
        "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 items-baseline gap-2 border-b border-border-subtle px-3 py-2">
        <h2 className="min-w-0 flex-1 text-[13px] font-semibold text-foreground">
          Agents
        </h2>
        <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">
          {rows.length}
        </span>
      </header>

      {status === "loading" ? (
        <p className="px-3 py-6 text-[11px] text-muted-foreground">
          Loading agents…
        </p>
      ) : null}
      {status === "error" ? (
        <div className="m-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-[11px] text-red-500">
          <p>{error ?? "Agents failed to load."}</p>
          <button
            type="button"
            onClick={() => void load()}
            className="mt-1 underline underline-offset-2"
          >
            Retry
          </button>
        </div>
      ) : null}

      {status === "ready" ? (
        <>
          {actionError ? (
            <p
              role="alert"
              className="mx-3 mt-3 rounded-md border border-red-500/30 bg-red-500/10 px-2 py-1.5 text-[10.5px] text-red-500"
            >
              {actionError}
            </p>
          ) : null}

          <ul className="min-h-0 flex-1 divide-y divide-border-subtle overflow-y-auto">
            {rows.map((row) => {
              const busy = busyId === row.id;
              return (
                <li
                  key={row.id}
                  className="flex flex-col gap-1.5 px-3 py-2.5"
                  aria-label={row.name}
                >
                  <div className="flex items-center gap-2">
                    <span
                      aria-hidden="true"
                      className={cn(
                        "size-1.5 shrink-0 rounded-full",
                        STATUS_DOT[row.status],
                      )}
                    />
                    <span className="min-w-0 flex-1 truncate text-[12px] font-medium text-foreground">
                      {row.name}
                    </span>
                    <span className="shrink-0 text-[10px] text-muted-foreground">
                      {row.statusLabel}
                    </span>
                  </div>
                  <p className="text-[10.5px] text-muted-foreground">
                    {row.reportsToName
                      ? `Reports to ${row.reportsToName}`
                      : "No reporting line"}
                  </p>
                  <div className="flex flex-wrap items-center gap-1.5">
                    {row.canPause ? (
                      <RowAction
                        disabled={busy}
                        onClick={() => transition(row, "paused")}
                      >
                        Pause
                      </RowAction>
                    ) : null}
                    {row.canResume ? (
                      <RowAction
                        disabled={busy}
                        onClick={() => transition(row, "active")}
                      >
                        Resume
                      </RowAction>
                    ) : null}
                    {row.canTerminate ? (
                      confirmingId === row.id ? (
                        <RowAction
                          disabled={busy}
                          destructive
                          onClick={() => transition(row, "terminated")}
                        >
                          Confirm terminate
                        </RowAction>
                      ) : (
                        <RowAction
                          disabled={busy}
                          destructive
                          onClick={() => {
                            setConfirmingId(row.id);
                            setActionError(null);
                          }}
                        >
                          Terminate
                        </RowAction>
                      )
                    ) : null}
                  </div>
                  {row.canTerminate ? (
                    <label className="flex items-center gap-1.5 text-[10.5px] text-muted-foreground">
                      <span className="shrink-0">Reports to</span>
                      <select
                        value={row.reportsToId ?? ""}
                        disabled={busy}
                        onChange={(event) =>
                          setReporting(row, event.target.value)
                        }
                        aria-label={`Reporting line for ${row.name}`}
                        className="h-6 min-w-0 flex-1 rounded border border-border bg-background px-1 text-[10.5px] text-foreground disabled:opacity-50"
                      >
                        <option value="">No one</option>
                        {toManagerOptions(agents, row.id).map((option) => (
                          <option key={option.id} value={option.id}>
                            {option.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  ) : null}
                </li>
              );
            })}
            {rows.length === 0 ? (
              <li className="px-3 py-6 text-center text-[11px] text-muted-foreground">
                No agents registered yet — add the first below.
              </li>
            ) : null}
          </ul>

          <form
            className="shrink-0 space-y-1.5 border-t border-border-subtle px-3 py-2.5"
            onSubmit={(event) => {
              event.preventDefault();
              create();
            }}
          >
            <div className="flex items-center gap-1.5">
              <input
                value={newName}
                onChange={(event) => setNewName(event.target.value)}
                placeholder="New agent name"
                aria-label="New agent name"
                className="h-7 min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-[11px] text-foreground placeholder:text-muted-foreground"
              />
              <button
                type="submit"
                disabled={busyId !== null}
                className="inline-flex h-7 shrink-0 items-center rounded-md bg-foreground px-2.5 text-[11px] font-medium text-background transition-opacity hover:opacity-90 disabled:opacity-50"
              >
                Add
              </button>
            </div>
            {createOptions.length > 0 ? (
              <label className="flex items-center gap-1.5 text-[10.5px] text-muted-foreground">
                <span className="shrink-0">Reports to</span>
                <select
                  value={newManagerId}
                  onChange={(event) => setNewManagerId(event.target.value)}
                  aria-label="Reporting line for the new agent"
                  className="h-6 min-w-0 flex-1 rounded border border-border bg-background px-1 text-[10.5px] text-foreground"
                >
                  <option value="">No one</option>
                  {createOptions.map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.name}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </form>
        </>
      ) : null}
    </div>
  );
}

function RowAction({
  disabled,
  destructive = false,
  onClick,
  children,
}: {
  disabled: boolean;
  destructive?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "inline-flex h-6 items-center rounded-md border px-2 text-[10.5px] font-medium transition-colors disabled:opacity-50",
        destructive
          ? "border-red-500/40 text-red-500 hover:bg-red-500/10"
          : "border-border text-foreground hover:bg-muted",
      )}
    >
      {children}
    </button>
  );
}
