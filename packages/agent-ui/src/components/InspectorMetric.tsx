export type InspectorMetricProps = {
  label: string;
  value: string;
};

/**
 * Compact metric tile for run inspector panels (Plan / Changes / Approvals /
 * Subagents). Purely presentational; hosts supply the computed value.
 */
export function InspectorMetric({ label, value }: InspectorMetricProps) {
  return (
    <div className="bg-card px-2.5 py-2">
      <div className="text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground/65">
        {label}
      </div>
      <div className="mt-0.5 text-[11px] font-semibold tabular-nums text-foreground">
        {value}
      </div>
    </div>
  );
}
