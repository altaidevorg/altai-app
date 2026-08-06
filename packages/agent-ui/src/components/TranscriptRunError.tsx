import { cn } from "../lib/cn.js";

export type TranscriptRunErrorVariant = "error" | "attention";

export type TranscriptRunErrorProps = {
  message: string;
  variant?: TranscriptRunErrorVariant;
  title?: string;
  onDismiss?: () => void;
  dismissLabel?: string;
  className?: string;
};

/**
 * Assertive chat run failure / attention banner with optional dismiss.
 * Wave 4 / A6.8 — host decides `attention` vs fatal from run/policy copy.
 */
export function TranscriptRunError({
  message,
  variant = "error",
  title,
  onDismiss,
  dismissLabel = "Dismiss",
  className,
}: TranscriptRunErrorProps) {
  const resolvedTitle =
    title ??
    (variant === "attention" ? "Run needs attention" : "Something went wrong.");

  return (
    <div
      role="alert"
      aria-atomic="true"
      className={cn(
        "rounded-md border px-3 py-2 text-xs",
        variant === "attention"
          ? "border-warning/40 bg-warning/10 text-foreground"
          : "border-destructive/40 bg-destructive/10 text-destructive",
        className,
      )}
    >
      <div className="font-medium">{resolvedTitle}</div>
      <div className="mt-0.5 leading-relaxed opacity-90">{message}</div>
      {onDismiss ? (
        <button
          type="button"
          onClick={onDismiss}
          className="mt-1 underline opacity-80 hover:opacity-100"
        >
          {dismissLabel}
        </button>
      ) : null}
    </div>
  );
}
