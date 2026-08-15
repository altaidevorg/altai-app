import { useCallback, useEffect } from "react";
import {
  negotiateViaDesktop,
  probeOperationsHealth,
} from "./lib/operationsHealth";
import { useOperationsContextStore } from "./store/operationsContextStore";

type Props = {
  workspacePath: string | null;
  workspaceName: string | null;
};

function statusText(
  connection: string,
  deploymentMode: string | null,
  protocolVersion: string | null,
  detail: string | null,
): string {
  switch (connection) {
    case "healthy":
      return `Control plane healthy · ${deploymentMode ?? "unknown host"} · protocol ${protocolVersion ?? "?"}`;
    case "connecting":
      return "Connecting to control plane…";
    case "degraded":
      return `Control plane degraded — ${detail ?? "reason unknown"}`;
    default:
      return "Control plane offline — choose a project";
  }
}

/**
 * The operations shell's control-plane context line (package 061, PR 1).
 * Surfaces the explicit connection state and negotiated host identity for
 * the open workspace; the board/list views (package 062) mount under it.
 */
export function OperationsStatusBar({ workspacePath, workspaceName }: Props) {
  const connection = useOperationsContextStore((s) => s.connection);
  const deploymentMode = useOperationsContextStore((s) => s.deploymentMode);
  const protocolVersion = useOperationsContextStore((s) => s.protocolVersion);
  const detail = useOperationsContextStore((s) => s.detail);

  const probe = useCallback(async () => {
    if (!workspacePath) return;
    const health = await probeOperationsHealth({
      workspacePath,
      negotiate: negotiateViaDesktop,
    });
    useOperationsContextStore
      .getState()
      .applyHealth(workspacePath, health, Date.now());
  }, [workspacePath]);

  useEffect(() => {
    useOperationsContextStore.getState().setWorkspace(workspacePath, workspaceName);
    void probe();
  }, [probe, workspacePath, workspaceName]);

  return (
    <p
      role="status"
      className="mt-1 flex items-center gap-2 text-[10px] text-muted-foreground"
    >
      <span
        aria-hidden="true"
        className={
          connection === "healthy"
            ? "size-1.5 rounded-full bg-emerald-500"
            : connection === "offline"
              ? "size-1.5 rounded-full bg-zinc-400"
              : "size-1.5 rounded-full bg-amber-500"
        }
      />
      <span>{statusText(connection, deploymentMode, protocolVersion, detail)}</span>
      {workspacePath ? (
        <button
          type="button"
          onClick={() => void probe()}
          className="rounded px-1 text-[10px] underline underline-offset-2 hover:text-foreground"
        >
          Recheck
        </button>
      ) : null}
    </p>
  );
}
