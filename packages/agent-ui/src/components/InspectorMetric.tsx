export type InspectorMetricProps = {
  label: string;
  value: string;
};

/**
 * Inline metric for the Details strip (Plan / Changes / Approvals / Subagents).
 */
export function InspectorMetric({ label, value }: InspectorMetricProps) {
  return (
    <div className="inline-flex items-baseline gap-1 text-[11px]">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums text-foreground">{value}</span>
    </div>
  );
}
