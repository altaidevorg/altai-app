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
 * Slim status + metrics strip under the Details header (not a nested card).
 */
export function RunOverviewCard({
  statusPill,
  tokenLabel,
  step,
  metrics,
}: RunOverviewCardProps) {
  return (
    <section className="border-b border-border-subtle bg-card">
      <div className="flex h-8 items-center gap-2 px-2.5">
        <div className="min-w-0 shrink-0">{statusPill}</div>
        {step ? (
          <p className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
            {step}
          </p>
        ) : (
          <div className="min-w-0 flex-1" />
        )}
        <span className="shrink-0 text-[10.5px] tabular-nums text-muted-foreground">
          {tokenLabel}
        </span>
      </div>
      {metrics.length ? (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-border-subtle px-2.5 py-1.5">
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
