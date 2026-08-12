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
 * Pending tool approvals with Deny / Approve actions.
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
    <ul className="divide-y divide-border-subtle">
      {approvals.map((approval) => (
        <li key={approval.id} className="py-2 first:pt-0 last:pb-0">
          <div className="flex items-center gap-2">
            <span className="size-1.5 shrink-0 animate-pulse rounded-full bg-foreground/70" />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium text-foreground">
              {approval.action}
            </span>
          </div>
          <pre className="mt-1.5 max-h-24 max-w-full min-w-0 overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted/60 px-2 py-1.5 font-mono text-[10.5px] leading-relaxed text-muted-foreground [overflow-wrap:anywhere]">
            {approvalPreview(approval.payload)}
          </pre>
          <div className="mt-2 flex justify-end gap-1.5">
            <button
              type="button"
              onClick={() => onRespond(approval.id, false)}
              className="inline-flex h-7 items-center rounded-md px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
            >
              Deny
            </button>
            <button
              type="button"
              onClick={() => onRespond(approval.id, true)}
              className="inline-flex h-7 items-center rounded-md bg-foreground px-2 text-[11px] font-medium text-background transition-opacity hover:opacity-90"
            >
              Approve
            </button>
          </div>
        </li>
      ))}
    </ul>
  );
}
