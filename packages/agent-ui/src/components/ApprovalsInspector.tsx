import { InspectorEmpty } from "./InspectorEmpty.js";

export type ApprovalsInspectorItem = {
  id: string;
  action: string;
  payload: unknown;
};

export type ApprovalsInspectorProps = {
  approvals: ApprovalsInspectorItem[];
  onRespond: (id: string, approved: boolean) => void;
};

/** Serialize an approval payload for compact inspector display. */
export function approvalPreview(payload: unknown): string {
  try {
    const serialized = JSON.stringify(payload, null, 2) ?? String(payload);
    return serialized.length > 900 ? `${serialized.slice(0, 900)}…` : serialized;
  } catch {
    return String(payload);
  }
}

/**
 * Run-inspector panel for pending tool approvals. Purely presentational;
 * the host supplies approvals and the respond callback.
 */
export function ApprovalsInspector({
  approvals,
  onRespond,
}: ApprovalsInspectorProps) {
  if (!approvals.length) {
    return (
      <InspectorEmpty>
        Actions that need your approval will appear here without interrupting
        the task view.
      </InspectorEmpty>
    );
  }
  return (
    <div className="space-y-2">
      {approvals.map((approval) => (
        <div
          key={approval.id}
          className="rounded-md border border-warning/30 bg-warning/[0.06] p-2.5"
        >
          <div className="flex items-center gap-2">
            <span className="size-1.5 animate-pulse rounded-full bg-warning" />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium">
              {approval.action}
            </span>
          </div>
          <pre className="mt-2 max-h-24 max-w-full min-w-0 overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted p-2 font-mono text-[9.5px] leading-relaxed text-muted-foreground [overflow-wrap:anywhere]">
            {approvalPreview(approval.payload)}
          </pre>
          <div className="mt-2 flex justify-end gap-1.5">
            <button
              type="button"
              onClick={() => onRespond(approval.id, false)}
              className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              Deny
            </button>
            <button
              type="button"
              onClick={() => onRespond(approval.id, true)}
              className="rounded-md bg-foreground px-2 py-1 text-[10px] font-medium text-background hover:bg-foreground/90"
            >
              Approve
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
