import { useState } from "react";

export type RunRecoveryActionsProps = {
  /** When set, the banner is informational (run still working). */
  warning: boolean;
  title: string;
  detail: string;
  canContinue: boolean;
  canRetry: boolean;
  onContinue: () => void | Promise<void>;
  onRetry: () => void | Promise<void>;
  onSteer: () => void;
  onStop: () => void;
  onDismiss: () => void;
};

/**
 * Inline recovery strip under the chat transcript. Purely presentational;
 * the host computes copy and owns continue/retry/steer/stop transport.
 */
export function RunRecoveryActions({
  warning,
  title,
  detail,
  canContinue,
  canRetry,
  onContinue,
  onRetry,
  onSteer,
  onStop,
  onDismiss,
}: RunRecoveryActionsProps) {
  const [submitting, setSubmitting] = useState(false);

  const runAction = async (action: () => void | Promise<void>) => {
    if (submitting) return;
    setSubmitting(true);
    try {
      await action();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      role={warning ? "status" : "alert"}
      className="mx-3 mb-2 rounded-lg border border-warning/35 bg-warning/[0.08] px-3 py-2.5"
    >
      <div className="text-[11px] font-medium text-foreground">{title}</div>
      <div className="mt-0.5 text-[10.5px] leading-relaxed text-muted-foreground">
        {detail}
      </div>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {canContinue ? (
          <button
            type="button"
            disabled={submitting}
            onClick={() => void runAction(onContinue)}
            className="rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background disabled:opacity-50"
          >
            Continue
          </button>
        ) : null}
        {canRetry ? (
          <button
            type="button"
            disabled={submitting}
            onClick={() => void runAction(onRetry)}
            className="rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background disabled:opacity-50"
          >
            Retry
          </button>
        ) : null}
        {warning || canContinue ? (
          <button
            type="button"
            onClick={onSteer}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Steer
          </button>
        ) : null}
        {warning ? (
          <button
            type="button"
            onClick={onStop}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Stop
          </button>
        ) : null}
        {warning ? (
          <button
            type="button"
            onClick={onDismiss}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Dismiss
          </button>
        ) : null}
      </div>
    </div>
  );
}
