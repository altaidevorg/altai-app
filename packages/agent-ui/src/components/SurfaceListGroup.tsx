import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type SurfaceListGroupProps = {
  title: string;
  description?: ReactNode;
  count?: number;
  children: ReactNode;
  /** Use `ul` when the injected rows render list items. */
  containerAs?: "div" | "ul";
  containerAriaLabel?: string;
  className?: string;
  containerClassName?: string;
};

/**
 * Shared titled-list chrome for Work queues and Scheduled lists. The host
 * supplies rows and owns their state, actions, and confirmation dialogs.
 */
export function SurfaceListGroup({
  title,
  description,
  count,
  children,
  containerAs: Container = "div",
  containerAriaLabel,
  className,
  containerClassName,
}: SurfaceListGroupProps) {
  return (
    <section className={className}>
      <SurfaceSectionHeader
        title={title}
        description={description}
        count={count}
        className="mb-2 px-0.5"
      />
      <Container
        aria-label={containerAriaLabel}
        className={cn(
          "overflow-hidden rounded-lg border border-border bg-card",
          containerClassName,
        )}
      >
        {children}
      </Container>
    </section>
  );
}
