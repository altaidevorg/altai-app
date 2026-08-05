import { useEffect, useState } from "react";
import { OperationsNavigationShell } from "@altai/agent-ui";
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
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
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
    <OperationsNavigationShell view="overview" onViewChange={() => {}} availableViews={["overview"]}>
      <CommandCenter
      repoRoot={repoRoot}
      workspaceName={workspaceName}
      onCreateWork={createWork}
      newWorkRequestKey={newWorkKey}
      />
    </OperationsNavigationShell>
  );
}
