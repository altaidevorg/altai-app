import { useEffect, useState } from "react";
import { AutomationsPanel } from "./AutomationsPanel";
import { SurfaceTabs } from "@altai/agent-ui";
import { TaskRunsPanel } from "./TaskRunsPanel";

export type WorkHubView = "runs" | "scheduled";

/**
 * One home for agent work. Runs are individual executions; scheduled work is
 * the durable definition that can create executions later. The two existing
 * views keep their focused controls while this shell keeps navigation stable.
 */
export function WorkHubPanel({
  initialView,
  onClose,
}: {
  initialView: WorkHubView;
  onClose: () => void;
}) {
  const [view, setView] = useState<WorkHubView>(initialView);

  useEffect(() => {
    setView(initialView);
  }, [initialView]);

  const navigation = (
    <div className="shrink-0 border-b border-border-subtle bg-card px-3 py-2">
      <SurfaceTabs
        label="Work view"
        value={view}
        onChange={(value) => setView(value as WorkHubView)}
        items={[
          { id: "runs", label: "Runs" },
          { id: "scheduled", label: "Scheduled" },
        ]}
        className="w-full"
      />
    </div>
  );

  return view === "runs" ? (
    <TaskRunsPanel onClose={onClose} navigation={navigation} />
  ) : (
    <AutomationsPanel onClose={onClose} navigation={navigation} />
  );
}
