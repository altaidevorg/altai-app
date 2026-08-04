import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type CreateFormActionsProps = {
  status?: ReactNode;
  statusTone?: "muted" | "destructive";
  onCancel: () => void;
  cancelLabel?: string;
  submitLabel: ReactNode;
  submitDisabled?: boolean;
  /** When true, wraps in a bordered section used by task create. */
  sectioned?: boolean;
  className?: string;
};

/**
 * Shared cancel / submit footer for create-task and create-automation forms.
 * Host owns form submit handlers and disabled policy.
 */
export function CreateFormActions({
  status,
  statusTone = "muted",
  onCancel,
  cancelLabel = "Cancel",
  submitLabel,
  submitDisabled = false,
  sectioned = false,
  className,
}: CreateFormActionsProps) {
  const row = (
    <div
      className={cn(
        "altai-create-form-actions flex items-center gap-2",
        sectioned ? undefined : "mt-4 justify-between border-t border-border-subtle pt-3",
        className,
      )}
    >
      {status != null && status !== false ? (
        <span
          className={cn(
            "min-w-0 flex-1 truncate",
            sectioned ? "text-[10px]" : "text-[9.5px]",
            statusTone === "destructive"
              ? "text-destructive"
              : "text-muted-foreground",
          )}
        >
          {status}
        </span>
      ) : (
        <span className="flex-1" />
      )}
      <button
        type="button"
        onClick={onCancel}
        className={cn(
          "rounded-md px-2.5 py-1.5 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground",
          !sectioned && "ml-auto",
        )}
      >
        {cancelLabel}
      </button>
      <button
        type="submit"
        disabled={submitDisabled}
        className={cn(
          "inline-flex items-center gap-1.5 rounded-md bg-primary font-semibold text-primary-foreground disabled:cursor-not-allowed disabled:opacity-45",
          sectioned
            ? "px-3 py-1.5 text-[10.5px] transition-opacity hover:opacity-90"
            : "px-3 py-1.5 text-[10px]",
        )}
      >
        {submitLabel}
      </button>
    </div>
  );

  if (!sectioned) return row;

  return (
    <section className="border-t border-border-subtle px-3.5 py-3">
      {row}
    </section>
  );
}
