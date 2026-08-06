import { useEffect, useState } from "react";
import {
  OperationsNavigationShell,
  type OperationsView,
} from "@altai/agent-ui";
import { NotificationInboxPanel } from "@/modules/ai/components/NotificationInboxPanel";
import { TaskRunsPanel } from "@/modules/ai/components/TaskRunsPanel";
import { WorkHubPanel } from "@/modules/ai/components/WorkHubPanel";
import { CommandCenter } from "./CommandCenter";

type Props = {
  repoRoot: string;
  navigation?: {
    action: "new-work";
    key: number;
  };
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
  const [view, setView] = useState<OperationsView>("overview");
  const [newWorkKey, setNewWorkKey] = useState<number | undefined>(
    navigation?.key,
  );

  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Local workspace";

  useEffect(() => {
    if (!navigation) return;
    // New work must land on Overview (composer host lives there).
    setView("overview");
    setNewWorkKey(navigation.key);
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
        <WorkHubPanel initialView="runs" presentation="embedded" />
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
