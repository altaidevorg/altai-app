import { useEffect } from "react";
import {
  negotiateViaDesktop,
  probeOperationsHealth,
} from "./lib/operationsHealth";
import { useOperationsContextStore } from "./store/operationsContextStore";

function statusText(
  connection: string,
  scopeProject: string | null,
  deploymentMode: string | null,
  protocolVersion: string | null,
  detail: string | null,
): string {
  const scope = scopeProject ? ` · ${scopeProject}` : "";
  switch (connection) {
    case "healthy":
      return `Control plane healthy · ${deploymentMode ?? "unknown host"} · protocol ${protocolVersion ?? "?"}${scope}`;
    case "connecting":
      return `Connecting to control plane…${scope}`;
    case "degraded":
      return `Control plane degraded — ${detail ?? "reason unknown"}${scope}`;
    default:
      return "Control plane offline — choose a project";
  }
}

/**
 * Re-probe the currently open workspace on demand (the Recheck buttons).
 */
export async function recheckOperations(): Promise<void> {
  const { workspacePath } = useOperationsContextStore.getState();
  if (!workspacePath) return;
  const health = await probeOperationsHealth({
    workspacePath,
    negotiate: negotiateViaDesktop,
  });
  useOperationsContextStore
    .getState()
    .applyHealth(workspacePath, health, Date.now());
}

/**
 * The operations context store's single driver (package 061): announces the
 * open workspace and probes the control plane once per workspace change.
 * Mounted once at the app root; the status bar and DesktopHome line only
 * read the store.
 */
export function useOperationsProbe(
  workspacePath: string | null,
  workspaceName: string | null,
): void {
  useEffect(() => {
    useOperationsContextStore.getState().setWorkspace(workspacePath, workspaceName);
    if (!workspacePath) return;
    void recheckOperations();
  }, [workspacePath, workspaceName]);
}

/**
 * The operations shell's control-plane context line (package 061). Read-only:
 * surfaces the explicit connection state, negotiated host identity, and
 * workspace-local org/project scope for the open workspace.
 */
export function OperationsStatusBar() {
  const connection = useOperationsContextStore((s) => s.connection);
  const scope = useOperationsContextStore((s) => s.scope);
  const deploymentMode = useOperationsContextStore((s) => s.deploymentMode);
  const protocolVersion = useOperationsContextStore((s) => s.protocolVersion);
  const detail = useOperationsContextStore((s) => s.detail);
  const workspacePath = useOperationsContextStore((s) => s.workspacePath);

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
      <span>
        {statusText(
          connection,
          scope?.project ?? null,
          deploymentMode,
          protocolVersion,
          detail,
        )}
      </span>
      {workspacePath ? (
        <button
          type="button"
          onClick={() => void recheckOperations()}
          className="rounded px-1 text-[10px] underline underline-offset-2 hover:text-foreground"
        >
          Recheck
        </button>
      ) : null}
    </p>
  );
}
