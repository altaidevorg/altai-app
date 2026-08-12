import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AiRunInspectorFrameVariant = "sidebar" | "compact";

export type AiRunInspectorFrameProps = {
  /** Host-wired run header (usually `RunDetailsHeader`). */
  header: ReactNode;
  /** Optional overview strip immediately below the header. */
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
 * Shared Details composition: raised header → metric strip → flat sections.
 */
export function AiRunInspectorFrame({
  header,
  summary,
  children,
  variant = "sidebar",
  className,
  bodyClassName,
  summaryClassName,
  "aria-label": ariaLabel = "Details",
}: AiRunInspectorFrameProps) {
  const body = (
    <div
      className={cn(
        variant === "sidebar"
          ? "min-h-0 min-w-0 flex-1 overflow-y-auto"
          : undefined,
        bodyClassName,
      )}
    >
      {summary ? (
        <div className={cn("sticky top-0 z-10 bg-card", summaryClassName)}>
          {summary}
        </div>
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
