import { useEffect, useState } from "react";
import {
  OperationsNavigationShell,
  type OperationsView,
} from "@altai/agent-ui";
import { CommandCenter } from "./CommandCenter";

type Props = {
  repoRoot: string;
  navigation?: {
    action: "new-work";
    key: number;
  };
};

/**
 * Local-first operations tab. Work is created, observed, reviewed, and
 * delivered from the command center without a second planning surface.
 * Secondary nav advertises available A7 slices; only Overview is live today.
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
  const [view, setView] = useState<OperationsView>("overview");
  const [newWorkKey, setNewWorkKey] = useState<number | undefined>(
    navigation?.key,
  );

  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Local workspace";

  useEffect(() => {
    if (navigation) setNewWorkKey(navigation.key);
  }, [navigation]);

  const createWork = () => {
    setNewWorkKey(Date.now());
  };

  return (
    <OperationsNavigationShell
      view={view}
      onViewChange={setView}
      availableViews={["overview"]}
    >
      {view === "overview" ? (
        <CommandCenter
          repoRoot={repoRoot}
          workspaceName={workspaceName}
          onCreateWork={createWork}
          newWorkRequestKey={newWorkKey}
        />
      ) : null}
    </OperationsNavigationShell>
  );
}
