import { useEffect, useState } from "react";
import {
  WorkHubNavigation,
  type WorkHubView,
} from "@altai/agent-ui";
import { AutomationsPanel } from "./AutomationsPanel";
import { TaskRunsPanel } from "./TaskRunsPanel";

export type { WorkHubView };

/**
 * One home for agent work. Runs are individual executions; scheduled work is
 * the durable definition that can create executions later. Navigation chrome
 * comes from `@altai/agent-ui`; this host mounts the focused panel bodies.
 */
export function WorkHubPanel({
  initialView,
  onClose,
  presentation = "overlay",
}: {
  initialView: WorkHubView;
  onClose?: () => void;
  presentation?: "overlay" | "embedded";
}) {
  const [view, setView] = useState<WorkHubView>(initialView);

  useEffect(() => {
    setView(initialView);
  }, [initialView]);

  const navigation = (
    <WorkHubNavigation view={view} onViewChange={setView} />
  );

  return view === "runs" ? (
    <TaskRunsPanel
      onClose={onClose}
      navigation={navigation}
      presentation={presentation}
    />
  ) : (
    <AutomationsPanel
      onClose={onClose}
      navigation={navigation}
      presentation={presentation}
    />
  );
}
