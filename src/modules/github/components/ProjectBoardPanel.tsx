import { useEffect, useState } from "react";
import {
  OperationsNavigationShell,
  type OperationsView,
  type WorkHubView,
} from "@altai/agent-ui";
import { NotificationInboxPanel } from "@/modules/ai/components/NotificationInboxPanel";
import { TaskRunsPanel } from "@/modules/ai/components/TaskRunsPanel";
import { WorkHubPanel } from "@/modules/ai/components/WorkHubPanel";
import type { ProjectBoardNavigation } from "@/modules/tabs";
import { CommandCenter } from "./CommandCenter";

type Props = {
  repoRoot: string;
  navigation?: ProjectBoardNavigation;
};

/** Live Operations routes. Agents/governance stay gated until host bodies ship. */
const AVAILABLE_VIEWS: readonly OperationsView[] = [
  "overview",
  "work",
  "runs",
  "inbox",
];

/**
 * Local-first operations tab.
 * - Overview: attention + progress (host aggregation).
 * - Work: runs + scheduled via Work hub secondary strip.
 * - Runs: background task queue alone (no scheduled strip).
 * - Inbox: agent attention queue.
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
  const [view, setView] = useState<OperationsView>(
    navigation?.view ?? "overview",
  );
  const [workHubView, setWorkHubView] = useState<WorkHubView>(
    navigation?.workHubView ?? "runs",
  );
  const [newWorkKey, setNewWorkKey] = useState<number | undefined>(
    navigation?.action === "new-work" ? navigation.key : undefined,
  );

  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Local workspace";

  useEffect(() => {
    if (!navigation) return;
    if (navigation.action === "new-work") {
      // Create-work composer only mounts on Overview.
      setView("overview");
      setNewWorkKey(navigation.key);
      return;
    }
    if (navigation.view) setView(navigation.view);
    if (navigation.workHubView) setWorkHubView(navigation.workHubView);
  }, [navigation]);

  const createWork = () => {
    setView("overview");
    setNewWorkKey(Date.now());
  };

  return (
    <OperationsNavigationShell
      view={view}
      onViewChange={setView}
      availableViews={AVAILABLE_VIEWS}
    >
      {view === "overview" ? (
        <CommandCenter
          repoRoot={repoRoot}
          workspaceName={workspaceName}
          onCreateWork={createWork}
          newWorkRequestKey={newWorkKey}
        />
      ) : null}
      {view === "work" ? (
        <WorkHubPanel initialView={workHubView} presentation="embedded" />
      ) : null}
      {view === "runs" ? (
        <TaskRunsPanel
          presentation="embedded"
          surfaceTitle="Runs"
          surfaceEyebrow="Background executions"
        />
      ) : null}
      {view === "inbox" ? (
        <NotificationInboxPanel presentation="embedded" />
      ) : null}
    </OperationsNavigationShell>
  );
}
