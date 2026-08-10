import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AiRunInspectorFrameVariant = "sidebar" | "compact";

export type AiRunInspectorFrameProps = {
  /** Host-wired run header (usually `RunDetailsHeader`). */
  header: ReactNode;
  /** Optional overview card immediately below the header. */
  summary?: ReactNode;
  /** Inspector sections or compact follow-on detail content. */
  children?: ReactNode;
  variant?: AiRunInspectorFrameVariant;
  className?: string;
  bodyClassName?: string;
  summaryClassName?: string;
  "aria-label"?: string;
};

/**
 * Shared run-details composition frame. Hosts retain stores, RPC callbacks,
 * and detail sections while the common header → overview → content structure
 * stays identical across Desktop and VS Code.
 */
export function AiRunInspectorFrame({
  header,
  summary,
  children,
  variant = "sidebar",
  className,
  bodyClassName,
  summaryClassName,
  "aria-label": ariaLabel = "Run details",
}: AiRunInspectorFrameProps) {
  const body = (
    <div
      className={cn(
        variant === "sidebar"
          ? "min-h-0 flex-1 space-y-2.5 overflow-y-auto p-2.5"
          : undefined,
        bodyClassName,
      )}
    >
      {summary ? (
        <div className={summaryClassName}>{summary}</div>
      ) : null}
      {children}
    </div>
  );

  if (variant === "compact") {
    return (
      <section
        aria-label={ariaLabel}
        data-ai-run-inspector-frame
        data-variant="compact"
        className={className}
      >
        {header}
        {body}
      </section>
    );
  }

  return (
    <aside
      aria-label={ariaLabel}
      data-ai-run-inspector-frame
      data-variant="sidebar"
      className={cn(
        "flex min-h-0 min-w-0 flex-col border-l border-border-subtle bg-card",
        className,
      )}
    >
      {header}
      {body}
    </aside>
  );
}
