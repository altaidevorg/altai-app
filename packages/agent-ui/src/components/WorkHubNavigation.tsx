import { SurfaceTabs } from "./AuxiliarySurface.js";

export type WorkHubView = "runs" | "scheduled";

export type WorkHubNavigationProps = {
  view: WorkHubView;
  onViewChange: (view: WorkHubView) => void;
};

/**
 * Stable Runs / Scheduled tab strip for the Work hub. Presentational; the host
 * owns which panel body (task runs vs automations) is mounted.
 */
export function WorkHubNavigation({
  view,
  onViewChange,
}: WorkHubNavigationProps) {
  return (
    <div className="altai-work-hub-navigation shrink-0 border-b border-border-subtle bg-card px-3 py-2">
      <SurfaceTabs
        label="Work view"
        value={view}
        onChange={(value) => onViewChange(value as WorkHubView)}
        items={[
          { id: "runs", label: "Runs" },
          { id: "scheduled", label: "Scheduled" },
        ]}
        className="w-full"
      />
    </div>
  );
}
