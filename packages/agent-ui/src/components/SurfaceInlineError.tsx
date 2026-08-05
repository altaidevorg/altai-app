import { cn } from "../lib/cn.js";

export type SurfaceInlineErrorProps = {
  message: string;
  onDismiss?: () => void;
  dismissLabel?: string;
  dismissAriaLabel?: string;
  className?: string;
};

/**
 * Compact dismissible error strip used by Work / Automations list modes.
 * Host supplies dismiss callback; Radix AlertDialog confirmations stay on host.
 */
export function SurfaceInlineError({
  message,
  onDismiss,
  dismissLabel = "Dismiss",
  dismissAriaLabel,
  className,
}: SurfaceInlineErrorProps) {
  return (
    <div
      role="alert"
      className={cn(
        "mx-3 mt-3 border border-destructive/30 bg-destructive/[0.06] px-2 py-1.5 text-[10px] text-destructive",
        className,
      )}
    >
      {message}
      {onDismiss ? (
        <button
          type="button"
          onClick={onDismiss}
          aria-label={dismissAriaLabel ?? dismissLabel}
          className="ml-2 underline"
        >
          {dismissLabel}
        </button>
      ) : null}
    </div>
  );
}
