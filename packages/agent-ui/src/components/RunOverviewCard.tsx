import type { ReactNode } from "react";
import { InspectorMetric } from "./InspectorMetric.js";

export type RunOverviewMetric = {
  label: string;
  value: string;
};

export type RunOverviewCardProps = {
  statusPill: ReactNode;
  tokenLabel: string;
  step?: string | null;
  metrics: RunOverviewMetric[];
};

/**
 * Top summary card in the run details inspector: status, usage, step, metrics.
 * Host supplies status pill node and computed metric values.
 */
export function RunOverviewCard({
  statusPill,
  tokenLabel,
  step,
  metrics,
}: RunOverviewCardProps) {
  return (
    <section className="rounded-lg border border-border bg-muted/30 p-3">
      <div className="flex items-center gap-2">
        {statusPill}
        <span className="ml-auto text-[9.5px] tabular-nums text-muted-foreground">
          {tokenLabel}
        </span>
      </div>
      {step ? (
        <p className="mt-2 line-clamp-2 text-[10.5px] leading-relaxed text-foreground">
          {step}
        </p>
      ) : null}
      {metrics.length ? (
        <div className="mt-3 grid grid-cols-2 gap-px overflow-hidden rounded-md border border-border bg-border">
          {metrics.map((metric) => (
            <InspectorMetric
              key={metric.label}
              label={metric.label}
              value={metric.value}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}
