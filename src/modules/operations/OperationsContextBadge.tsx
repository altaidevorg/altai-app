import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useOperationsContextStore } from "./store/operationsContextStore";
import { recheckOperations } from "./OperationsStatusBar";

function badgeLabel(connection: string): string {
  switch (connection) {
    case "healthy":
      return "Control plane";
    case "connecting":
      return "Control plane: connecting…";
    case "degraded":
      return "Control plane: degraded";
    default:
      return "Control plane: offline";
  }
}

/**
 * Compact shell-chrome indicator for the operations context (package 061).
 * Read-only like the status line; the driver is `useOperationsProbe` at the
 * app root. Clicking re-probes the open workspace.
 */
export function OperationsContextBadge() {
  const connection = useOperationsContextStore((s) => s.connection);
  const scope = useOperationsContextStore((s) => s.scope);
  const deploymentMode = useOperationsContextStore((s) => s.deploymentMode);
  const protocolVersion = useOperationsContextStore((s) => s.protocolVersion);
  const detail = useOperationsContextStore((s) => s.detail);
  const workspacePath = useOperationsContextStore((s) => s.workspacePath);

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={`${badgeLabel(connection)}${scope ? ` · ${scope.organization}/${scope.project}` : ""} — recheck`}
          onClick={() => void recheckOperations()}
          className="flex shrink-0 cursor-default items-center gap-1.5 rounded px-1 text-[10.5px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <span
            aria-hidden="true"
            className={cn(
              "size-1.5 rounded-full",
              connection === "healthy"
                ? "bg-emerald-500"
                : connection === "offline"
                  ? "bg-zinc-400"
                  : "bg-amber-500",
            )}
          />
          <span>{badgeLabel(connection)}</span>
        </button>
      </TooltipTrigger>
      <TooltipContent side="top" className="max-w-72 text-[11px] leading-relaxed">
        {scope ? (
          <p>
            Scope: {scope.organization} / {scope.project} (workspace-local —
            this host serves no org/project projections yet)
          </p>
        ) : (
          <p>No workspace open.</p>
        )}
        {deploymentMode ? (
          <p>
            Serving host: {deploymentMode}, protocol {protocolVersion ?? "?"}.
          </p>
        ) : null}
        {detail ? <p>{detail}</p> : null}
        {workspacePath ? (
          <p className="text-muted-foreground">Click to re-probe this workspace.</p>
        ) : (
          <p className="text-muted-foreground">Open a project to connect.</p>
        )}
      </TooltipContent>
    </Tooltip>
  );
}
