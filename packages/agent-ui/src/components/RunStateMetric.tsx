export type RunStateMetricProps = {
  label: string;
  value: string;
};

/**
 * Compact metric tile shown in the run state header (Approvals / Subagents /
 * Input / Output tokens). Visually distinct from `InspectorMetric`, which is
 * used inside collapsible inspector sections. Purely presentational.
 */
export function RunStateMetric({ label, value }: RunStateMetricProps) {
  return (
    <div className="rounded-md bg-foreground/[0.035] px-2 py-1.5">
      <div className="text-[9.5px] text-muted-foreground">{label}</div>
      <div className="mt-0.5 font-medium tabular-nums text-foreground">{value}</div>
    </div>
  );
}
